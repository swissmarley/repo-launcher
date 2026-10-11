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
use std::path::PathBuf;
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
        let ctx = Context {
            app: &self.paths,
            project: &pp,
            root: &root,
            project_id: &project.id,
            sink: &sink,
            log: &log,
            cancel,
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
        let recipe = recipe::from_detection(&report)?;
        self.store.set_recipe(&project.id, &recipe)?;

        ctx.progress(Stage::Preparing, 0.25, None);
        let uv = runtime::ensure_uv(&self.paths, &self.http, cancel, |done, total| {
            if let Some(total) = total.filter(|t| *t > 0) {
                ctx.progress(
                    Stage::Preparing,
                    0.25 + 0.05 * (done as f32 / total as f32),
                    None,
                );
            }
        })
        .await?;

        install::run_steps(&ctx, &recipe, &uv).await?;
        ctx.progress(Stage::Ready, 1.0, None);
        Ok(())
    }

    fn project_root(&self, project: &Project) -> PathBuf {
        let src = self.paths.project(&project.id).src();
        match &project.subdir {
            Some(sub) => sub.split('/').fold(src, |p, part| p.join(part)),
            None => src,
        }
    }

    async fn fetch_source(&self, project: &Project, ctx: &Context<'_>) -> Result<()> {
        let pp = self.paths.project(&project.id);
        let src = pp.src();
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

        let _ = tokio::fs::remove_dir_all(&src).await;
        tokio::fs::rename(&staging, &src).await?;
        if !self.project_root(project).is_dir() {
            return Err(
                Problem::new(ProblemCode::NoEntryPoint).detail("subfolder missing from download")
            );
        }
        Ok(())
    }

    pub async fn launch(&self, id: &str, sink: EventSink) -> Result<LaunchInfo> {
        let project = self.store.get(id)?;
        let recipe = match (&project.status, &project.recipe) {
            (ProjectStatus::Installed, Some(r)) => r.clone(),
            _ => return Err(Problem::new(ProblemCode::NotInstalled).fix(FixAction::Install)),
        };
        let pp = self.paths.project(id);
        let root = self.project_root(&project);
        let uv = runtime::uv_path(&self.paths);
        self.supervisor
            .launch(LaunchRequest {
                app: &self.paths,
                project: &pp,
                root: &root,
                project_id: id,
                recipe: &recipe,
                uv: &uv,
                sink,
            })
            .await
    }

    pub async fn stop(&self, id: &str) {
        self.supervisor.stop(id).await;
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
