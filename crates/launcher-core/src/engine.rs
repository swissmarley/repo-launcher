//! The engine's public API. Both the desktop app and the `rl` CLI go through this type.

use crate::detect::{self, FileTree};
use crate::download::{self, Download};
use crate::events::{EventSink, LogStream, Stage};
use crate::install::{self, Context, ProjectLog};
use crate::paths::AppPaths;
use crate::problem::{FixAction, Problem, ProblemCode, Result};
use crate::recipe;
use crate::recipe::keys::{self, KnownKey};
use crate::run::{LaunchInfo, LaunchRequest, Supervisor};
use crate::runtime;
use crate::safety::{self, SafetyReport};
use crate::secrets::{self, KeychainStore, Secret, SecretStore, Vault};
use crate::source::auth::{
    self, AppConfig, Auth, DeviceCode, GitHubAccount, RemoteRepo, StoredToken,
};
use crate::source::{self, SourceSpec, archive, github::GitHub, url::RepoRef};
use crate::store::{self, Project, ProjectStatus, Store};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

/// The AI-suggested recipe for a project, kept beside its other bookkeeping.
pub const AI_RECIPE_FILE: &str = "ai-launcher.json";
const AI_ENABLED_SETTING: &str = "ai.enabled";
const GITHUB_LOGIN_SETTING: &str = "github.login";
const GITHUB_AVATAR_SETTING: &str = "github.avatar_url";

/// Where the engine keeps secrets and which servers it talks to. Tests swap these out.
pub struct EngineOptions {
    pub secrets: Arc<dyn SecretStore>,
    pub github_api: String,
    pub github_web: String,
    pub anthropic_api: String,
}

impl Default for EngineOptions {
    fn default() -> Self {
        Self {
            secrets: Arc::new(KeychainStore::new()),
            github_api: "https://api.github.com".into(),
            github_web: "https://github.com".into(),
            anthropic_api: "https://api.anthropic.com".into(),
        }
    }
}

/// A setting a project asks the user for, with whether it has a value. Secret values are never
/// sent to the UI; plain values are.
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvVarView {
    pub name: String,
    pub label: Option<String>,
    pub description: Option<String>,
    pub required: bool,
    pub secret: bool,
    pub help_url: Option<String>,
    pub known: Option<KnownKey>,
    pub has_value: bool,
    pub value: Option<String>,
}

/// The result of asking AI: whether a plan was saved, and what the model said about the project.
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiOutcome {
    pub recipe_saved: bool,
    pub explanation: String,
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    /// `None` when secrets can be stored; otherwise why not.
    pub keychain_problem: Option<Problem>,
    pub github_configured: bool,
    pub github_account: Option<GitHubAccount>,
    pub github_install_url: Option<String>,
    pub ai_enabled: bool,
    pub ai_has_key: bool,
    pub trusted_publishers: Vec<String>,
}

pub struct Engine {
    paths: AppPaths,
    http: reqwest::Client,
    github: GitHub,
    auth: Auth,
    store: Store,
    vault: Vault,
    supervisor: Supervisor,
    anthropic_api: String,
    pending_sign_in: Mutex<Option<auth::Pending>>,
    /// Refreshing rotates the refresh token, so only one refresh may run at a time.
    token_lock: tokio::sync::Mutex<()>,
}

impl Engine {
    pub fn open(paths: AppPaths) -> Result<Self> {
        Self::with_options(paths, EngineOptions::default())
    }

    pub fn with_options(paths: AppPaths, opts: EngineOptions) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(concat!("RepoLauncher/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(Problem::internal)?;
        let store = Store::open(&paths.db())?;
        let github = GitHub::with_bases(http.clone(), &opts.github_api, &opts.github_web);
        let auth = Auth::new(http.clone(), &opts.github_web, &opts.github_api);
        Ok(Self {
            paths,
            http,
            github,
            auth,
            store,
            vault: Vault::new(opts.secrets),
            supervisor: Supervisor::default(),
            anthropic_api: opts.anthropic_api,
            pending_sign_in: Mutex::default(),
            token_lock: tokio::sync::Mutex::new(()),
        })
    }

    pub fn paths(&self) -> &AppPaths {
        &self.paths
    }

    pub fn list(&self) -> Result<Vec<Project>> {
        self.store.list()
    }

    pub fn get(&self, id: &str) -> Result<Project> {
        self.store.get(id)
    }

    pub fn is_running(&self, id: &str) -> bool {
        self.supervisor.is_running(id)
    }

    pub fn running_url(&self, id: &str) -> Option<String> {
        self.supervisor.url(id)
    }

    pub fn run_history(&self, id: &str) -> Vec<String> {
        self.supervisor.history(id)
    }

    /// The live run log if the project has run this session, else the newest log file on disk.
    pub fn recent_log(&self, id: &str, max_lines: usize) -> Vec<String> {
        let live = self.supervisor.history(id);
        if !live.is_empty() {
            return live.into_iter().rev().take(max_lines).rev().collect();
        }
        let logs = self.paths.project(id).logs();
        let newest = ["run.log", "install.log"]
            .iter()
            .map(|f| logs.join(f))
            .filter_map(|p| Some((std::fs::metadata(&p).ok()?.modified().ok()?, p)))
            .max_by_key(|(t, _)| *t);
        let Some((_, path)) = newest else {
            return Vec::new();
        };
        let text = std::fs::read_to_string(path).unwrap_or_default();
        let lines: Vec<&str> = text.lines().collect();
        lines[lines.len().saturating_sub(max_lines)..]
            .iter()
            .map(|l| l.split_once("] ").map_or(*l, |(_, rest)| rest).to_owned())
            .collect()
    }

    /// Registers a project from a pasted link or local folder. Nothing is downloaded yet.
    /// Adding the same project twice returns the existing one.
    pub async fn add(&self, input: &str) -> Result<Project> {
        let spec = SourceSpec::parse(input)?;
        let now = store::now();
        let mut project = Project {
            id: new_id(),
            source: String::new(),
            name: String::new(),
            description: None,
            owner: None,
            repo: None,
            ref_name: None,
            commit: None,
            subdir: None,
            stars: None,
            license: None,
            avatar_url: None,
            is_private: false,
            archived: false,
            repo_created_at: None,
            owner_created_at: None,
            consented_fingerprint: None,
            keys_reviewed: false,
            status: ProjectStatus::Added,
            kind: None,
            recipe: None,
            created_at: now,
            updated_at: now,
        };
        match &spec {
            SourceSpec::Local(path) => {
                project.source = format!("local:{}", path.display());
                project.name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
            }
            SourceSpec::GitHub(r) => {
                project.source = format!("github:{}", r.slug().to_lowercase());
                if r.tree.is_none()
                    && let Some(existing) = self.store.find_by_source(&project.source)?
                {
                    return Ok(existing);
                }
                let (meta, token) = self.lookup_repo(r).await?;
                match meta {
                    Some(meta) => {
                        project.name = meta.name;
                        project.description = meta.description;
                        project.stars = Some(meta.stars);
                        project.license = meta
                            .license
                            .and_then(|l| l.spdx_id.or(l.name))
                            .filter(|l| l != "NOASSERTION");
                        project.avatar_url = meta.owner.avatar_url;
                        project.archived = meta.archived;
                        project.repo_created_at = meta.created_at;
                        project.is_private = meta.private || token.is_some();
                    }
                    None => {
                        project.name = r.repo.clone();
                        project.is_private = token.is_some();
                    }
                }
                // Little-known public projects: one more call to learn the account's age.
                if !project.is_private && project.stars.is_some_and(|s| s < 50) {
                    project.owner_created_at =
                        self.github.owner_created_at(&r.owner).await.ok().flatten();
                }
                let resolved = self
                    .github
                    .resolve_ref(r, token.as_ref().map(Secret::expose))
                    .await?;
                project.owner = Some(r.owner.clone());
                project.repo = Some(r.repo.clone());
                if r.tree.is_some() {
                    project.source = format!("{}@{}", project.source, resolved.name);
                    if let Some(sub) = &resolved.subdir {
                        project.source = format!("{}/{sub}", project.source);
                    }
                    if let Some(existing) = self.store.find_by_source(&project.source)? {
                        return Ok(existing);
                    }
                }
                project.ref_name = Some(resolved.name);
                project.commit = Some(resolved.commit);
                project.subdir = resolved.subdir;
            }
        }
        self.store.insert(&project)?;
        Ok(project)
    }

    pub async fn install(
        &self,
        id: &str,
        sink: EventSink,
        cancel: CancellationToken,
    ) -> Result<Project> {
        let project = self.store.get(id)?;
        let before = project.status;
        self.store.set_status(id, ProjectStatus::Installing)?;
        let result = self.install_inner(&project, sink, &cancel).await;
        let status = match &result {
            Ok(()) => ProjectStatus::Installed,
            Err(p) if p.code == ProblemCode::Cancelled => ProjectStatus::Added,
            // An update whose plan changed: the old environment no longer matches the files.
            Err(p) if p.code == ProblemCode::ConsentRequired => match before {
                ProjectStatus::Added => ProjectStatus::Added,
                _ => ProjectStatus::NeedsAttention,
            },
            Err(_) => ProjectStatus::NeedsAttention,
        };
        self.store.set_status(id, status)?;
        result.and_then(|_| self.store.get(id))
    }

    /// Downloads the project (nothing runs) and returns what installing it would do, for the
    /// safety gate. Consent is then given with [`Engine::consent`].
    pub async fn review(
        &self,
        id: &str,
        sink: EventSink,
        cancel: CancellationToken,
    ) -> Result<SafetyReport> {
        let project = self.store.get(id)?;
        let pp = self.paths.project(&project.id);
        let log = ProjectLog::create(&project.id, &pp.logs().join("install.log"), sink.clone());
        let root = self.project_root(&project);
        let (source_id, placeholder) = (self.source_id(&project), runtime::Tools::default());
        let ctx = self.context(
            &project,
            &pp,
            &root,
            &sink,
            &log,
            &cancel,
            &placeholder,
            &source_id,
        );
        ctx.progress(Stage::Downloading, 0.0, None);
        let (recipe, tree) = self.prepare_source(&project, &ctx).await?;
        self.safety_report(&self.store.get(id)?, &recipe, &tree)
    }

    /// Records that the user agreed to install and run the plan with this fingerprint.
    pub fn consent(&self, id: &str, fingerprint: &str, trust_owner: bool) -> Result<()> {
        let project = self.store.get(id)?;
        let current = project.recipe.as_ref().map(recipe::Recipe::fingerprint);
        if current.as_deref() != Some(fingerprint) {
            // The plan changed since the user looked at it.
            return Err(Problem::new(ProblemCode::ConsentRequired).fix(FixAction::ReviewSafety));
        }
        self.store.set_consent(id, Some(fingerprint))?;
        if trust_owner && let Some(owner) = &project.owner {
            self.store.set_trusted(owner, true)?;
        }
        Ok(())
    }

    fn safety_report(
        &self,
        project: &Project,
        recipe: &recipe::Recipe,
        tree: &FileTree,
    ) -> Result<SafetyReport> {
        let trusted = match &project.owner {
            Some(o) => self.store.is_trusted(o)?,
            None => false,
        };
        Ok(safety::assess(safety::Input {
            project,
            recipe,
            tree,
            trusted_owner: trusted,
            now: store::now(),
        }))
    }

    fn source_id(&self, project: &Project) -> String {
        // Local folders change without a commit id; never resume their steps.
        project
            .commit
            .clone()
            .unwrap_or_else(|| format!("local-{}", store::now()))
    }

    #[allow(clippy::too_many_arguments)]
    fn context<'a>(
        &'a self,
        project: &'a Project,
        pp: &'a crate::paths::ProjectPaths,
        root: &'a Path,
        sink: &'a EventSink,
        log: &'a ProjectLog,
        cancel: &'a CancellationToken,
        tools: &'a runtime::Tools,
        source_id: &'a str,
    ) -> Context<'a> {
        Context {
            app: &self.paths,
            project: pp,
            root,
            project_id: &project.id,
            sink,
            log,
            cancel,
            tools,
            source_id,
            last_progress: Default::default(),
        }
    }

    /// Downloads the source and works out the recipe. Runs nothing from the project.
    async fn prepare_source(
        &self,
        project: &Project,
        ctx: &Context<'_>,
    ) -> Result<(recipe::Recipe, FileTree)> {
        self.fetch_source(project, ctx).await?;
        let tree = self.source_tree(project)?;
        let report = detect::detect(&tree);
        ctx.log.line(
            LogStream::Engine,
            format!("detected: {:?}", report.ecosystems),
        );
        let (recipe, display) = self.resolve_recipe(&project.id, &tree, &report)?;
        if let Some(d) = display {
            self.store
                .set_display(&project.id, d.name.as_deref(), d.description.as_deref())?;
        }
        self.store.set_recipe(&project.id, &recipe)?;
        Ok((recipe, tree))
    }

    /// The project's own files: what was downloaded, not what installing created since (a
    /// lockfile npm wrote must not change the plan, or a reinstall would ask for consent again).
    fn source_tree(&self, project: &Project) -> Result<FileTree> {
        let root = self.project_root(project);
        let scanned = FileTree::scan(&root)?;
        let manifest_path = self.paths.project(&project.id).meta().join("files.json");
        let Some(manifest) = source::sync::load_manifest(&manifest_path) else {
            return Ok(scanned);
        };
        let prefix = project
            .subdir
            .as_ref()
            .map(|s| format!("{}/", s.trim_end_matches('/')))
            .unwrap_or_default();
        let listing: Vec<String> = scanned
            .files()
            .filter(|f| manifest.contains_key(&format!("{prefix}{f}")))
            .map(str::to_owned)
            .collect();
        Ok(FileTree::from_parts(&root, listing))
    }

    /// The project's own launcher.json or heuristics; failing those, a saved AI suggestion.
    fn resolve_recipe(
        &self,
        id: &str,
        tree: &FileTree,
        report: &detect::DetectionReport,
    ) -> Result<(recipe::Recipe, Option<recipe::manifest::Display>)> {
        match recipe::resolve(tree, report) {
            Ok(found) => Ok(found),
            Err(p)
                if matches!(
                    p.code,
                    ProblemCode::NoEntryPoint | ProblemCode::UnsupportedProject
                ) =>
            {
                let ai = self.paths.project(id).meta().join(AI_RECIPE_FILE);
                if let Ok(text) = std::fs::read_to_string(&ai) {
                    let (mut r, d) = recipe::manifest::parse(&text, report)?;
                    r.source = recipe::RecipeSource::Ai;
                    if r.env_vars.is_empty() {
                        r.env_vars = recipe::env_from_hints(report);
                    }
                    return Ok((r, Some(d)));
                }
                Err(if self.ai_enabled() {
                    p.fix(FixAction::AskAi)
                } else {
                    p
                })
            }
            Err(p) => Err(p),
        }
    }

    async fn install_inner(
        &self,
        project: &Project,
        sink: EventSink,
        cancel: &CancellationToken,
    ) -> Result<()> {
        let pp = self.paths.project(&project.id);
        let log = ProjectLog::create(&project.id, &pp.logs().join("install.log"), sink.clone());
        let root = self.project_root(project);
        let source_id = self.source_id(project);
        let placeholder = runtime::Tools::default();
        let mut ctx = self.context(
            project,
            &pp,
            &root,
            &sink,
            &log,
            cancel,
            &placeholder,
            &source_id,
        );

        ctx.progress(Stage::Downloading, 0.0, None);
        let (recipe, tree) = self.prepare_source(project, &ctx).await?;

        // Installing runs the project's code, so it needs the user's consent to this exact plan.
        let report = self.safety_report(&self.store.get(&project.id)?, &recipe, &tree)?;
        if !report.consented {
            return Err(Problem::new(ProblemCode::ConsentRequired).fix(FixAction::ReviewSafety));
        }

        let tools = self.prepare_tools(&recipe, &root, Some(&ctx)).await?;
        ctx.tools = &tools;
        install::run_steps(&ctx, &recipe).await?;
        ctx.progress(Stage::Ready, 1.0, None);
        Ok(())
    }

    /// Works out (and with `ctx`, provisions) the runtimes a recipe needs.
    async fn prepare_tools(
        &self,
        recipe: &recipe::Recipe,
        root: &Path,
        ctx: Option<&Context<'_>>,
    ) -> Result<runtime::Tools> {
        let mut tools = runtime::Tools::default();
        let uv = runtime::uv_path(&self.paths);
        let venv_bin = ctx.map(|c| c.project.venv_bin());
        if let Some(bin) = venv_bin {
            tools.python = bin.join(runtime::exe("python"));
            tools.path.push(bin);
        }
        // node_modules/.bin of the app folder and of the repo root (monorepos hoist).
        let app_dir = root.join(&recipe.run.cwd);
        tools.path.push(app_dir.join("node_modules").join(".bin"));
        if recipe.run.cwd != "." {
            tools.path.push(root.join("node_modules").join(".bin"));
        }

        let progress = |stage_frac: f32, label: Option<recipe::StepLabel>| {
            move |done: u64, total: Option<u64>| {
                if let (Some(c), Some(t)) = (ctx, total.filter(|t| *t > 0)) {
                    c.progress(
                        Stage::Preparing,
                        stage_frac + 0.05 * (done as f32 / t as f32),
                        label.clone(),
                    );
                }
            }
        };
        let cancel = ctx.map(|c| c.cancel.clone()).unwrap_or_default();

        if let Some(node) = &recipe.node {
            let label = recipe::StepLabel::SetUpNode {
                version: node.version.clone(),
            };
            let dir = match ctx {
                Some(_) => {
                    runtime::ensure_node(
                        &self.paths,
                        &self.http,
                        &node.version,
                        &cancel,
                        progress(0.20, Some(label)),
                    )
                    .await?
                }
                None => runtime::node_dir(&self.paths, &node.version),
            };
            if let Some((package, version)) = runtime::package_manager_package(node) {
                let bin = match ctx {
                    Some(_) => {
                        runtime::ensure_package_manager(
                            &self.paths,
                            &dir,
                            package,
                            &version,
                            &cancel,
                        )
                        .await?
                    }
                    None => runtime::prefix_bin(&runtime::package_manager_prefix(
                        &self.paths,
                        package,
                        &version,
                    )),
                };
                tools.path.push(bin);
            }
            if node.package_manager == recipe::PackageManager::Bun {
                let bun = match ctx {
                    Some(_) => {
                        runtime::ensure_bun(&self.paths, &self.http, &cancel, progress(0.25, None))
                            .await?
                    }
                    None => runtime::bun_dir(&self.paths),
                };
                tools.path.push(bun);
            }
            tools.path.push(runtime::node_bin(&dir));
        }
        if recipe.go {
            let dir = match ctx {
                Some(_) => {
                    runtime::ensure_go(&self.paths, &self.http, &cancel, progress(0.20, None))
                        .await?
                }
                None => runtime::go_dir(&self.paths),
            };
            tools.path.push(dir.join("bin"));
        }
        if recipe.python.is_some() {
            if ctx.is_some() {
                runtime::ensure_uv(&self.paths, &self.http, &cancel, progress(0.25, None)).await?;
            }
            tools
                .path
                .push(uv.parent().map(Path::to_path_buf).unwrap_or_default());
        }
        for tool in &recipe.system_tools {
            let dir = find_system_tool(tool)
                .ok_or_else(|| Problem::new(ProblemCode::MissingSystemTool).param("tool", tool))?;
            if !tools.path.contains(&dir) {
                tools.path.push(dir);
            }
        }
        tools.uv = uv;
        Ok(tools)
    }

    fn project_root(&self, project: &Project) -> PathBuf {
        let src = self.paths.project(&project.id).src();
        match &project.subdir {
            Some(sub) => sub.split('/').fold(src, |p, part| p.join(part)),
            None => src,
        }
    }

    /// Brings `src/` to the project's recorded commit, keeping user work (see `source::sync`).
    /// Skips the download when `src/` already holds that commit (resume after an interruption).
    async fn fetch_source(&self, project: &Project, ctx: &Context<'_>) -> Result<()> {
        let pp = self.paths.project(&project.id);
        let src = pp.src();
        let marker = pp.meta().join("source");
        let current = tokio::fs::read_to_string(&marker).await.ok();
        if project.commit.is_some()
            && current.as_deref() == project.commit.as_deref()
            && src.is_dir()
        {
            ctx.log
                .line(LogStream::Engine, "files already downloaded".into());
            return Ok(());
        }
        let staging = pp.root.join("src.incoming");
        let _ = tokio::fs::remove_dir_all(&staging).await;

        if let Some(path) = project.source.strip_prefix("local:") {
            let (from, to) = (PathBuf::from(path), staging.clone());
            tokio::task::spawn_blocking(move || source::copy_local(&from, &to))
                .await
                .map_err(Problem::internal)??;
        } else {
            let (Some(owner), Some(repo), Some(commit)) =
                (&project.owner, &project.repo, &project.commit)
            else {
                return Err(Problem::internal(
                    "GitHub project without owner/repo/commit",
                ));
            };
            let r = RepoRef {
                owner: owner.clone(),
                repo: repo.clone(),
                tree: None,
            };
            // Private projects download through the API with the user's token; public ones
            // never send it.
            let token = self.token_for(project).await?;
            let url = match &token {
                Some(_) => self.github.api_tarball_url(&r, commit),
                None => self.github.archive_url(&r, commit),
            };
            let file = self.paths.downloads().join(format!(
                "{}-{}.tar.gz",
                project.id,
                &commit[..12.min(commit.len())]
            ));
            download::fetch(
                &self.http,
                Download {
                    url: &url,
                    dest: &file,
                    sha256: None,
                    bearer: token.as_ref().map(Secret::expose),
                },
                ctx.cancel,
                |done, total| {
                    let f = match total {
                        Some(t) if t > 0 => done as f32 / t as f32,
                        // GitHub often streams archives without a length: creep toward 90%.
                        _ => 1.0 - 1.0 / (1.0 + done as f32 / 2_000_000.0),
                    };
                    ctx.progress(Stage::Downloading, 0.20 * f.min(0.95), None);
                },
            )
            .await?;
            let (a, s) = (file.clone(), staging.clone());
            let extracted =
                tokio::task::spawn_blocking(move || archive::extract_tar_gz(&a, &s, true))
                    .await
                    .map_err(Problem::internal)??;
            ctx.log.line(
                LogStream::Engine,
                format!("downloaded {} files", extracted.files),
            );
            let _ = tokio::fs::remove_file(&file).await;
        }

        let manifest_path = pp.meta().join("files.json");
        let backups = pp.meta().join("backups").join(store::now().to_string());
        let (incoming, dest) = (staging.clone(), src.clone());
        let report = tokio::task::spawn_blocking(move || {
            let previous = source::sync::load_manifest(&manifest_path);
            let out = source::sync::apply(&incoming, &dest, previous.as_ref(), &backups)?;
            source::sync::save_manifest(&manifest_path, &out.0)?;
            Ok::<_, std::io::Error>(out.1)
        })
        .await
        .map_err(Problem::internal)??;
        if !report.backed_up.is_empty() {
            ctx.log.line(
                LogStream::Engine,
                format!(
                    "kept copies of your changes to: {}",
                    report.backed_up.join(", ")
                ),
            );
        }
        if let Some(commit) = &project.commit {
            tokio::fs::create_dir_all(pp.meta()).await?;
            tokio::fs::write(&marker, commit).await?;
        }
        if !self.project_root(project).is_dir() {
            return Err(
                Problem::new(ProblemCode::NoEntryPoint).detail("subfolder missing from download")
            );
        }
        Ok(())
    }

    /// Returns the newer commit if the project's branch or tag has moved. Quota-free.
    pub async fn check_update(&self, id: &str) -> Result<Option<String>> {
        let project = self.store.get(id)?;
        let (Some(owner), Some(repo)) = (&project.owner, &project.repo) else {
            return Ok(None);
        };
        let r = RepoRef {
            owner: owner.clone(),
            repo: repo.clone(),
            tree: project.ref_name.clone(),
        };
        let token = self.token_for(&project).await?;
        let resolved = self
            .github
            .resolve_ref(&r, token.as_ref().map(Secret::expose))
            .await?;
        Ok((Some(&resolved.commit) != project.commit.as_ref()).then_some(resolved.commit))
    }

    /// Moves the project to the latest commit of its branch and reinstalls what changed.
    /// Returns false when it was already up to date.
    pub async fn update(
        &self,
        id: &str,
        sink: EventSink,
        cancel: CancellationToken,
    ) -> Result<bool> {
        let Some(commit) = self.check_update(id).await? else {
            return Ok(false);
        };
        let was_running = self.supervisor.is_running(id);
        self.supervisor.stop(id).await;
        self.store.set_commit(id, &commit)?;
        self.install(id, sink.clone(), cancel).await?;
        if was_running {
            self.launch(id, sink).await?;
        }
        Ok(true)
    }

    /// Rebuilds the project's environment from scratch, keeping its files and the user's data.
    pub async fn repair(
        &self,
        id: &str,
        sink: EventSink,
        cancel: CancellationToken,
    ) -> Result<Project> {
        self.supervisor.stop(id).await;
        let project = self.store.get(id)?;
        let pp = self.paths.project(id);
        let _ = tokio::fs::remove_dir_all(pp.env()).await;
        let root = self.project_root(&project);
        let mut dirs = vec![root.join("node_modules")];
        if let Some(r) = &project.recipe {
            dirs.push(root.join(&r.run.cwd).join("node_modules"));
        }
        for d in dirs {
            let _ = tokio::fs::remove_dir_all(d).await;
        }
        install::forget_steps(&pp);
        self.install(id, sink, cancel).await
    }

    pub async fn launch(&self, id: &str, sink: EventSink) -> Result<LaunchInfo> {
        self.launch_with(id, sink, &[]).await
    }

    /// Launches with extra command-line arguments (terminal programs that need options).
    pub async fn launch_with(
        &self,
        id: &str,
        sink: EventSink,
        extra_args: &[String],
    ) -> Result<LaunchInfo> {
        let project = self.store.get(id)?;
        let recipe = match (&project.status, &project.recipe) {
            (ProjectStatus::Installed, Some(r)) => r.clone(),
            _ => return Err(Problem::new(ProblemCode::NotInstalled).fix(FixAction::Install)),
        };
        // Keys are read from the keychain now and handed only to this run's process.
        let user_env = self.launch_env(id, &recipe)?;
        let pp = self.paths.project(id);
        let root = self.project_root(&project);
        let mut tools = self.prepare_tools(&recipe, &root, None).await?;
        tools.python = pp.venv_python();
        tools.path.insert(0, pp.venv_bin());
        self.supervisor
            .launch(LaunchRequest {
                app: &self.paths,
                project: &pp,
                root: &root,
                project_id: id,
                recipe: &recipe,
                tools: &tools,
                extra_args,
                user_env: &user_env,
                sink,
            })
            .await
    }

    /// The values of the project's settings, or `MISSING_KEYS` if a required one is empty.
    fn launch_env(&self, id: &str, recipe: &recipe::Recipe) -> Result<BTreeMap<String, String>> {
        let plain = self.store.env_values(id)?;
        let mut env = BTreeMap::new();
        let mut missing = Vec::new();
        for spec in &recipe.env_vars {
            let value = if spec.secret {
                let key = secrets::project_env_key(id, &spec.name);
                if self.store.has_secret(&key)? {
                    self.vault.get(&key)?.map(|s| s.expose().to_owned())
                } else {
                    None
                }
            } else {
                plain.get(&spec.name).cloned()
            };
            match value {
                Some(v) if !v.is_empty() => {
                    env.insert(spec.name.clone(), v);
                }
                _ if spec.required => missing.push(spec.name.clone()),
                _ => {}
            }
        }
        if !missing.is_empty() {
            return Err(Problem::new(ProblemCode::MissingKeys)
                .param("names", missing.join(", "))
                .fix(FixAction::EnterKeys));
        }
        Ok(env)
    }

    pub async fn stop(&self, id: &str) {
        self.supervisor.stop(id).await;
    }

    /// Keyboard input from the built-in console.
    pub fn console_write(&self, id: &str, data: &str) -> Result<()> {
        match self.supervisor.console(id) {
            Some(c) => c.write(data).map_err(Problem::internal),
            None => Ok(()),
        }
    }

    pub fn console_resize(&self, id: &str, cols: u16, rows: u16) {
        if let Some(c) = self.supervisor.console(id) {
            c.resize(cols, rows);
        }
    }

    /// What the console has shown so far, for a console window that opens late.
    pub fn console_scrollback(&self, id: &str) -> String {
        self.supervisor
            .console(id)
            .map(|c| c.scrollback())
            .unwrap_or_default()
    }

    pub async fn stop_all(&self) {
        self.supervisor.stop_all().await;
    }

    /// Stops the project and removes everything it put on disk and in the keychain.
    pub async fn uninstall(&self, id: &str) -> Result<()> {
        self.supervisor.stop(id).await;
        // If the keychain can't be reached, its index rows stay so a later cleanup can finish.
        if let Err(p) = self.vault.delete_project(&self.store, id) {
            tracing::warn!("keeping keychain index for {id}: {p}");
        }
        let pp = self.paths.project(id);
        if pp.root.exists() {
            tokio::fs::remove_dir_all(&pp.root).await?;
        }
        self.store.delete(id)
    }

    // --- project settings (keys) ---------------------------------------------------------

    /// The settings a project asks for. Secret values are never returned.
    pub fn env_vars(&self, id: &str) -> Result<Vec<EnvVarView>> {
        let project = self.store.get(id)?;
        let Some(recipe) = &project.recipe else {
            return Ok(Vec::new());
        };
        let plain = self.store.env_values(id)?;
        recipe
            .env_vars
            .iter()
            .map(|spec| {
                let known = keys::lookup(&spec.name);
                let (has_value, value) = if spec.secret {
                    let key = secrets::project_env_key(id, &spec.name);
                    (self.store.has_secret(&key)?, None)
                } else {
                    let v = plain.get(&spec.name).cloned();
                    (v.is_some(), v)
                };
                Ok(EnvVarView {
                    name: spec.name.clone(),
                    label: spec.label.clone(),
                    description: spec.description.clone(),
                    required: spec.required,
                    secret: spec.secret,
                    help_url: spec
                        .help_url
                        .clone()
                        .filter(|u| u.starts_with("https://"))
                        .or_else(|| known.as_ref().map(|k| k.help_url.to_owned())),
                    known,
                    has_value,
                    value,
                })
            })
            .collect()
    }

    /// Saves (or with `None`, clears) one of a project's settings. Secret ones go to the
    /// keychain; the rest to the database.
    pub fn set_env_value(&self, id: &str, name: &str, value: Option<&str>) -> Result<()> {
        let project = self.store.get(id)?;
        let spec = project
            .recipe
            .as_ref()
            .and_then(|r| r.env_vars.iter().find(|e| e.name == name))
            .ok_or_else(|| Problem::internal(format!("{name} is not a setting of this project")))?;
        let value = value.map(str::trim).filter(|v| !v.is_empty());
        if spec.secret {
            let key = secrets::project_env_key(id, name);
            match value {
                Some(v) => self
                    .vault
                    .set(&self.store, &key, Some(id), &Secret::new(v))?,
                None => self.vault.delete(&self.store, &key)?,
            }
        } else {
            self.store.set_env_value(id, name, value)?;
        }
        Ok(())
    }

    /// The user has seen the settings form once; it isn't shown before launch again.
    pub fn mark_keys_reviewed(&self, id: &str) -> Result<()> {
        self.store.set_keys_reviewed(id)
    }

    // --- app settings ----------------------------------------------------------------------

    pub fn settings(&self) -> Result<AppSettings> {
        let cfg = self.github_config();
        Ok(AppSettings {
            keychain_problem: self.vault.check().err(),
            github_configured: cfg.is_some(),
            github_install_url: cfg.map(|c| c.install_url()),
            github_account: self.github_account(),
            ai_enabled: self.ai_enabled(),
            ai_has_key: self.store.has_secret(secrets::ANTHROPIC_KEY)?,
            trusted_publishers: self.store.trusted_publishers()?,
        })
    }

    pub fn set_trusted_publisher(&self, owner: &str, trusted: bool) -> Result<()> {
        self.store.set_trusted(owner, trusted)
    }

    pub fn ai_enabled(&self) -> bool {
        self.store
            .setting(AI_ENABLED_SETTING)
            .ok()
            .flatten()
            .is_some_and(|v| v == "1")
    }

    pub fn set_ai_enabled(&self, enabled: bool) -> Result<()> {
        self.store
            .set_setting(AI_ENABLED_SETTING, enabled.then_some("1"))
    }

    /// Saves (or with `None`, removes) the user's own Anthropic API key.
    pub fn set_ai_key(&self, key: Option<&str>) -> Result<()> {
        match key.map(str::trim).filter(|k| !k.is_empty()) {
            Some(k) => self
                .vault
                .set(&self.store, secrets::ANTHROPIC_KEY, None, &Secret::new(k)),
            None => self.vault.delete(&self.store, secrets::ANTHROPIC_KEY),
        }
    }

    // --- AI-suggested recipes ----------------------------------------------------------------

    /// Exactly what asking AI about this project would send. Needs the project's files (the
    /// review step downloads them), AI turned on, and the user's Anthropic key.
    pub fn ai_preview(&self, id: &str) -> Result<crate::ai::Preview> {
        if !self.ai_enabled() {
            return Err(Problem::new(ProblemCode::AiNotEnabled).fix(FixAction::OpenSettings));
        }
        if !self.store.has_secret(secrets::ANTHROPIC_KEY)? {
            return Err(Problem::new(ProblemCode::AiKeyMissing).fix(FixAction::OpenSettings));
        }
        let project = self.store.get(id)?;
        if !self.project_root(&project).is_dir() {
            return Err(Problem::new(ProblemCode::NotInstalled).fix(FixAction::Install));
        }
        let label = match (&project.owner, &project.repo) {
            (Some(o), Some(r)) => format!("{o}/{r}"),
            _ => project.name.clone(),
        };
        Ok(crate::ai::preview(&label, &self.source_tree(&project)?))
    }

    /// Sends the previewed message (only if `digest` still matches what the user saw), checks
    /// the answer with the launcher.json validator and saves it for the safety review.
    pub async fn ai_suggest(&self, id: &str, digest: &str) -> Result<AiOutcome> {
        let preview = self.ai_preview(id)?;
        if preview.digest != digest {
            return Err(Problem::new(ProblemCode::AiFailed)
                .param("reason", "changed")
                .fix(FixAction::AskAi));
        }
        let key = self
            .vault
            .get(secrets::ANTHROPIC_KEY)?
            .ok_or_else(|| Problem::new(ProblemCode::AiKeyMissing).fix(FixAction::OpenSettings))?;
        let answer = crate::ai::suggest(&self.http, &self.anthropic_api, &key, &preview).await?;
        let Some(text) = answer.launcher_json else {
            return Ok(AiOutcome {
                recipe_saved: false,
                explanation: answer.explanation,
            });
        };
        let project = self.store.get(id)?;
        let report = detect::detect(&self.source_tree(&project)?);
        recipe::manifest::parse(&text, &report).map_err(|p| {
            Problem::new(ProblemCode::AiInvalidRecipe).detail(p.detail.unwrap_or_default())
        })?;
        let meta = self.paths.project(id).meta();
        std::fs::create_dir_all(&meta)?;
        std::fs::write(meta.join(AI_RECIPE_FILE), text)?;
        Ok(AiOutcome {
            recipe_saved: true,
            explanation: answer.explanation,
        })
    }

    /// Deletes every keychain entry Repo Launcher created (full uninstall). Returns how many.
    pub fn forget_all_secrets(&self) -> Result<usize> {
        let n = self.vault.delete_all(&self.store)?;
        self.clear_github_account()?;
        Ok(n)
    }

    // --- GitHub sign-in ----------------------------------------------------------------------

    /// Saves the GitHub App this installation signs in with (an environment variable, if set,
    /// still wins). The client ID is public; GitHub Apps' device flow needs no secret.
    pub fn set_github_app(&self, client_id: Option<&str>, app_slug: Option<&str>) -> Result<()> {
        self.store.set_setting(auth::CLIENT_ID_SETTING, client_id)?;
        self.store.set_setting(auth::APP_SLUG_SETTING, app_slug)
    }

    pub fn github_config(&self) -> Option<AppConfig> {
        AppConfig::resolve(|k| self.store.setting(k).ok().flatten())
    }

    /// The connected account, from the database (no keychain access).
    pub fn github_account(&self) -> Option<GitHubAccount> {
        let login = self.store.setting(GITHUB_LOGIN_SETTING).ok().flatten()?;
        Some(GitHubAccount {
            login,
            avatar_url: self.store.setting(GITHUB_AVATAR_SETTING).ok().flatten(),
        })
    }

    /// Starts "Connect GitHub": returns the code the user types on github.com.
    pub async fn github_begin_sign_in(&self) -> Result<DeviceCode> {
        let cfg = self.github_config().ok_or_else(auth::not_configured)?;
        self.vault.check()?;
        let (code, pending) = self.auth.request_device_code(&cfg).await?;
        *self
            .pending_sign_in
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = Some(pending);
        Ok(code)
    }

    /// Waits for the user to approve the code from [`Engine::github_begin_sign_in`].
    pub async fn github_finish_sign_in(&self, cancel: CancellationToken) -> Result<GitHubAccount> {
        let cfg = self.github_config().ok_or_else(auth::not_configured)?;
        let pending = self
            .pending_sign_in
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
            .ok_or_else(|| {
                Problem::new(ProblemCode::SignInExpired).fix(FixAction::ConnectGithub)
            })?;
        let result = self
            .auth
            .poll_for_token(&cfg, &pending, &cancel, store::now)
            .await;
        *self
            .pending_sign_in
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = None;
        let token = result?;
        let account = self.auth.user(&token.access_token).await?;
        self.vault
            .set(&self.store, secrets::GITHUB_TOKEN, None, &token.to_secret())?;
        self.store
            .set_setting(GITHUB_LOGIN_SETTING, Some(&account.login))?;
        self.store
            .set_setting(GITHUB_AVATAR_SETTING, account.avatar_url.as_deref())?;
        Ok(account)
    }

    /// Forgets the GitHub sign-in on this computer. Returns the GitHub page where the user can
    /// also revoke Repo Launcher's access (revoking needs a server-side secret we don't have).
    pub fn github_sign_out(&self) -> Result<String> {
        self.vault.delete(&self.store, secrets::GITHUB_TOKEN)?;
        self.clear_github_account()?;
        Ok(auth::AUTHORIZATIONS_URL.to_owned())
    }

    fn clear_github_account(&self) -> Result<()> {
        self.store.set_setting(GITHUB_LOGIN_SETTING, None)?;
        self.store.set_setting(GITHUB_AVATAR_SETTING, None)
    }

    /// A working access token, refreshed when close to expiry; `None` when not signed in.
    pub async fn github_token(&self) -> Result<Option<Secret>> {
        if !self.store.has_secret(secrets::GITHUB_TOKEN)? {
            return Ok(None);
        }
        let _guard = self.token_lock.lock().await;
        let Some(saved) = self.vault.get(secrets::GITHUB_TOKEN)? else {
            self.store.unindex_secret(secrets::GITHUB_TOKEN)?;
            self.clear_github_account()?;
            return Ok(None);
        };
        let Some(token) = StoredToken::from_secret(&saved) else {
            self.github_sign_out()?;
            return Ok(None);
        };
        let now = store::now();
        if !token.needs_refresh(now) {
            return Ok(Some(Secret::new(token.access_token)));
        }
        let cfg = self.github_config().ok_or_else(auth::not_configured)?;
        let refreshed = match &token.refresh_token {
            Some(r) if token.refresh_expires_at.is_none_or(|t| t > now) => {
                self.auth.refresh(&cfg, r, now).await
            }
            _ => Err(auth::signed_out()),
        };
        match refreshed {
            Ok(new) => {
                self.vault
                    .set(&self.store, secrets::GITHUB_TOKEN, None, &new.to_secret())?;
                Ok(Some(Secret::new(new.access_token)))
            }
            Err(p) if p.code == ProblemCode::SignedOut => {
                self.github_sign_out()?;
                Err(p)
            }
            Err(p) => Err(p),
        }
    }

    /// The token for a private project; `None` for public ones (they never send it).
    async fn token_for(&self, project: &Project) -> Result<Option<Secret>> {
        if !project.is_private {
            return Ok(None);
        }
        self.github_token()
            .await?
            .map(Some)
            .ok_or_else(auth::signed_out)
    }

    /// Repositories the user shared with Repo Launcher ("Your projects").
    pub async fn github_repositories(&self) -> Result<Vec<RemoteRepo>> {
        let token = self.github_token().await?.ok_or_else(auth::signed_out)?;
        self.auth.repositories(token.expose()).await
    }

    /// Repo metadata, using the token only when anonymous access can't see the repo.
    async fn lookup_repo(
        &self,
        r: &RepoRef,
    ) -> Result<(Option<source::github::RepoMeta>, Option<Secret>)> {
        match self.github.metadata(r, None).await {
            Ok(meta) => return Ok((Some(meta), None)),
            // Quota used up: the quota-free ref listing tells public repos apart.
            Err(p) if p.code == ProblemCode::RateLimited => {
                match self.github.resolve_ref(r, None).await {
                    Ok(_) => return Ok((None, None)),
                    Err(p) if p.code == ProblemCode::RepoNotFoundOrPrivate => {}
                    Err(p) => return Err(p),
                }
            }
            Err(p) if p.code == ProblemCode::RepoNotFoundOrPrivate => {}
            Err(p) => return Err(p),
        }
        // Private, or doesn't exist.
        let not_found = || {
            Problem::new(ProblemCode::RepoNotFoundOrPrivate)
                .param("repo", r.slug())
                .fix(FixAction::ConnectGithub)
        };
        let token = match self.github_token().await {
            Ok(Some(t)) => t,
            Ok(None) => return Err(not_found()),
            Err(p) if p.code == ProblemCode::SignedOut => return Err(not_found()),
            Err(p) => return Err(p),
        };
        match self.github.metadata(r, Some(token.expose())).await {
            Ok(meta) => Ok((Some(meta), Some(token))),
            Err(p) if p.code == ProblemCode::RepoNotFoundOrPrivate => {
                let url = self
                    .github_config()
                    .map(|c| c.install_url())
                    .unwrap_or_else(|| "https://github.com/settings/installations".into());
                Err(Problem::new(ProblemCode::RepoNotShared)
                    .param("repo", r.slug())
                    .fix(FixAction::ChooseRepositories { url }))
            }
            Err(p) => Err(p),
        }
    }
}

/// Finds the folder holding a program the user installed themselves. Apps started from the
/// Dock or Start menu get a minimal PATH, so common install locations are checked too.
fn find_system_tool(name: &str) -> Option<PathBuf> {
    let mut dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    dirs.extend(crate::env::system_path());
    if cfg!(unix) {
        dirs.extend(["/opt/homebrew/bin", "/usr/local/bin", "/snap/bin"].map(PathBuf::from));
    }
    let names: Vec<String> = if cfg!(windows) {
        vec![
            format!("{name}.exe"),
            format!("{name}.cmd"),
            name.to_owned(),
        ]
    } else {
        vec![name.to_owned()]
    };
    dirs.into_iter()
        .find(|d| names.iter().any(|n| d.join(n).is_file()))
}

/// Eight lowercase characters, e.g. `k3f9x2ab`.
fn new_id() -> String {
    const ALPHABET: &[u8] = b"abcdefghijkmnpqrstuvwxyz23456789";
    uuid::Uuid::new_v4()
        .as_bytes()
        .iter()
        .take(8)
        .map(|b| ALPHABET[(*b as usize) % ALPHABET.len()] as char)
        .collect()
}
