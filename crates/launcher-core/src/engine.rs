//! The engine's public API. Both the desktop app and the `rl` CLI go through this type.

use crate::detect::{self, FileTree};
use crate::download::{self, Download};
use crate::events::{EventSink, LogStream, Stage};
use crate::install::{self, Context, ProjectLog};
use crate::paths::AppPaths;
use crate::problem::{FixAction, Problem, ProblemCode, Result};
use crate::recipe;
use crate::run::{LaunchInfo, LaunchRequest, Supervisor};
use crate::runtime;
use crate::source::{self, SourceSpec, archive, github::GitHub};
use crate::store::{self, Project, ProjectStatus, Store};
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;

pub struct Engine {
    paths: AppPaths,
    http: reqwest::Client,
    github: GitHub,
    store: Store,
    supervisor: Supervisor,
}

impl Engine {
    pub fn open(paths: AppPaths) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(concat!("RepoLauncher/", env!("CARGO_PKG_VERSION")))
            .connect_timeout(std::time::Duration::from_secs(15))
            .build()
            .map_err(Problem::internal)?;
        let store = Store::open(&paths.db())?;
        let github = GitHub::new(http.clone());
        Ok(Self {
            paths,
            http,
            github,
            store,
            supervisor: Supervisor::default(),
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
                // Metadata is nice-to-have: if the anonymous API quota is used up, carry on
                // with what the quota-free ref listing gives us.
                match self.github.metadata(r).await {
                    Ok(meta) => {
                        project.name = meta.name;
                        project.description = meta.description;
                        project.stars = Some(meta.stars);
                        project.license = meta
                            .license
                            .and_then(|l| l.spdx_id.or(l.name))
                            .filter(|l| l != "NOASSERTION");
                        project.avatar_url = meta.owner.avatar_url;
                    }
                    Err(p) if p.code == ProblemCode::RateLimited => project.name = r.repo.clone(),
                    Err(p) => return Err(p),
                }
                let resolved = self.github.resolve_ref(r).await?;
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
        self.store.set_status(id, ProjectStatus::Installing)?;
        let result = self.install_inner(&project, sink, &cancel).await;
        let status = match &result {
            Ok(()) => ProjectStatus::Installed,
            Err(p) if p.code == ProblemCode::Cancelled => ProjectStatus::Added,
            Err(_) => ProjectStatus::NeedsAttention,
        };
        self.store.set_status(id, status)?;
        result.and_then(|_| self.store.get(id))
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
        // Local folders change without a commit id; never resume their steps.
        let source_id = project
            .commit
            .clone()
            .unwrap_or_else(|| format!("local-{}", store::now()));
        let placeholder = runtime::Tools::default();
        let mut ctx = Context {
            app: &self.paths,
            project: &pp,
            root: &root,
            project_id: &project.id,
            sink: &sink,
            log: &log,
            cancel,
            tools: &placeholder,
            source_id: &source_id,
            last_progress: Default::default(),
        };

        ctx.progress(Stage::Downloading, 0.0, None);
        self.fetch_source(project, &ctx).await?;

        let tree = FileTree::scan(&root)?;
        let report = detect::detect(&tree);
        log.line(
            LogStream::Engine,
            format!("detected: {:?}", report.ecosystems),
        );
        let (recipe, display) = recipe::resolve(&tree, &report)?;
        if let Some(d) = display {
            self.store
                .set_display(&project.id, d.name.as_deref(), d.description.as_deref())?;
        }
        self.store.set_recipe(&project.id, &recipe)?;

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
            let r = source::url::RepoRef {
                owner: owner.clone(),
                repo: repo.clone(),
                tree: None,
            };
            let url = self.github.archive_url(&r, commit);
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
                    bearer: None,
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
        let r = source::url::RepoRef {
            owner: owner.clone(),
            repo: repo.clone(),
            tree: project.ref_name.clone(),
        };
        let resolved = self.github.resolve_ref(&r).await?;
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
                sink,
            })
            .await
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

    /// Stops the project and removes everything it put on disk.
    pub async fn uninstall(&self, id: &str) -> Result<()> {
        self.supervisor.stop(id).await;
        let pp = self.paths.project(id);
        if pp.root.exists() {
            tokio::fs::remove_dir_all(&pp.root).await?;
        }
        self.store.delete(id)
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
