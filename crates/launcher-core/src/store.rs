//! Local state in SQLite. Schema changes are appended to `MIGRATIONS`; `PRAGMA user_version`
//! records how many have run.

use crate::problem::{Problem, ProblemCode, Result};
use crate::recipe::{AppKind, Recipe};
use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;

const MIGRATIONS: &[&str] = &[
    r#"
CREATE TABLE projects (
    id            TEXT PRIMARY KEY,
    source        TEXT NOT NULL,
    name          TEXT NOT NULL,
    description   TEXT,
    owner         TEXT,
    repo          TEXT,
    ref_name      TEXT,
    commit_sha    TEXT,
    subdir        TEXT,
    stars         INTEGER,
    license       TEXT,
    avatar_url    TEXT,
    status        TEXT NOT NULL,
    recipe_json   TEXT,
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL
);
CREATE UNIQUE INDEX projects_source ON projects(source);
"#,
    r#"
ALTER TABLE projects ADD COLUMN is_private INTEGER NOT NULL DEFAULT 0;
ALTER TABLE projects ADD COLUMN archived INTEGER NOT NULL DEFAULT 0;
ALTER TABLE projects ADD COLUMN repo_created_at TEXT;
ALTER TABLE projects ADD COLUMN owner_created_at TEXT;
ALTER TABLE projects ADD COLUMN consented_fingerprint TEXT;
ALTER TABLE projects ADD COLUMN consented_at INTEGER;
ALTER TABLE projects ADD COLUMN keys_reviewed INTEGER NOT NULL DEFAULT 0;
-- Index of keychain entries we created; the keychain itself is never enumerated.
CREATE TABLE secret_keys (
    key         TEXT PRIMARY KEY,
    project_id  TEXT,
    created_at  INTEGER NOT NULL
);
-- Values of settings a project marks as not secret. Secret values live in the keychain.
CREATE TABLE env_values (
    project_id  TEXT NOT NULL,
    name        TEXT NOT NULL,
    value       TEXT NOT NULL,
    PRIMARY KEY (project_id, name)
);
CREATE TABLE trusted_publishers (
    owner       TEXT PRIMARY KEY,
    added_at    INTEGER NOT NULL
);
CREATE TABLE settings (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL
);
"#,
    r#"
ALTER TABLE projects ADD COLUMN start_at_login INTEGER NOT NULL DEFAULT 0;
ALTER TABLE projects ADD COLUMN use_system_terminal INTEGER NOT NULL DEFAULT 0;
-- One row per launch. Rows without ended_at after a crash are checked on the next start.
CREATE TABLE runs (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    project_id      TEXT NOT NULL,
    pid             INTEGER,
    pgid            INTEGER,
    proc_start_ms   INTEGER,
    boot_ms         INTEGER,
    url             TEXT,
    started_at      INTEGER NOT NULL,
    ended_at        INTEGER,
    exit_code       INTEGER,
    end_reason      TEXT
);
CREATE INDEX runs_project ON runs(project_id, id);
-- Every shortcut file we created, so removing a project (or the app) removes them.
CREATE TABLE shortcuts (
    path        TEXT PRIMARY KEY,
    project_id  TEXT NOT NULL,
    kind        TEXT NOT NULL,
    target      TEXT NOT NULL,
    created_at  INTEGER NOT NULL
);
"#,
];

/// Finished runs kept per project.
const RUNS_KEPT: i64 = 20;

#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ProjectStatus {
    Added,
    Installing,
    Installed,
    NeedsAttention,
}

impl ProjectStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Added => "added",
            Self::Installing => "installing",
            Self::Installed => "installed",
            Self::NeedsAttention => "needs_attention",
        }
    }
    fn parse(s: &str) -> Self {
        match s {
            "installing" => Self::Installing,
            "installed" => Self::Installed,
            "needs_attention" => Self::NeedsAttention,
            _ => Self::Added,
        }
    }
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: String,
    /// `github:owner/repo` or `local:/absolute/path`.
    pub source: String,
    pub name: String,
    pub description: Option<String>,
    pub owner: Option<String>,
    pub repo: Option<String>,
    pub ref_name: Option<String>,
    pub commit: Option<String>,
    pub subdir: Option<String>,
    #[cfg_attr(feature = "ts", ts(type = "number | null"))]
    pub stars: Option<u64>,
    pub license: Option<String>,
    pub avatar_url: Option<String>,
    /// Private on GitHub: downloads need the user's GitHub sign-in.
    pub is_private: bool,
    pub archived: bool,
    /// ISO dates from GitHub, used for "very new project" warnings.
    pub repo_created_at: Option<String>,
    pub owner_created_at: Option<String>,
    /// The recipe fingerprint the user agreed to install and run.
    #[serde(skip)]
    pub consented_fingerprint: Option<String>,
    /// The user has seen the keys form for this project once.
    pub keys_reviewed: bool,
    /// Launch this project when Repo Launcher starts at login.
    pub start_at_login: bool,
    /// Terminal programs open in the system's terminal instead of the built-in console.
    pub use_system_terminal: bool,
    pub status: ProjectStatus,
    pub kind: Option<AppKind>,
    #[serde(skip)]
    pub recipe: Option<Recipe>,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub created_at: i64,
    #[cfg_attr(feature = "ts", ts(type = "number"))]
    pub updated_at: i64,
}

/// A launch as recorded in the database: enough to find the process again after a crash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunRecord {
    pub id: i64,
    pub project_id: String,
    pub pid: Option<u32>,
    /// The POSIX process group (the project's whole tree); `None` on Windows.
    pub pgid: Option<u32>,
    /// When the process started, in ms since the epoch, to tell it apart from a reused pid.
    pub proc_start_ms: Option<i64>,
    pub boot_ms: Option<i64>,
    pub url: Option<String>,
}

/// Why a run ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EndReason {
    UserStop,
    Exited,
    Crashed,
    AppQuit,
    /// Found gone after the Launcher itself stopped unexpectedly.
    Lost,
    /// Still running after the Launcher stopped unexpectedly, and stopped on its next start.
    Orphaned,
}

impl EndReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::UserStop => "user_stop",
            Self::Exited => "exited",
            Self::Crashed => "crashed",
            Self::AppQuit => "app_quit",
            Self::Lost => "lost",
            Self::Orphaned => "orphaned",
        }
    }
}

#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ShortcutKind {
    /// Windows Start menu.
    StartMenu,
    Desktop,
    /// macOS ~/Applications (Launchpad, Spotlight).
    Applications,
    /// Linux applications menu.
    AppMenu,
}

impl ShortcutKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::StartMenu => "start_menu",
            Self::Desktop => "desktop",
            Self::Applications => "applications",
            Self::AppMenu => "app_menu",
        }
    }
    fn parse(s: &str) -> Self {
        match s {
            "start_menu" => Self::StartMenu,
            "desktop" => Self::Desktop,
            "applications" => Self::Applications,
            _ => Self::AppMenu,
        }
    }
}

/// A shortcut file Repo Launcher created. `target` is what it opens (the app's path or a link),
/// so shortcuts can be repaired when the app moves.
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Shortcut {
    pub project_id: String,
    pub kind: ShortcutKind,
    pub path: String,
    #[serde(skip)]
    pub target: String,
}

pub struct Store {
    conn: Mutex<Connection>,
}

pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Self::init(Connection::open(path)?)
    }

    pub fn in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
            conn.execute_batch(sql)?;
            conn.pragma_update(None, "user_version", (i + 1) as i64)?;
        }
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn insert(&self, p: &Project) -> Result<()> {
        let recipe = p
            .recipe
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(Problem::internal)?;
        self.conn().execute(
            "INSERT INTO projects (id, source, name, description, owner, repo, ref_name, commit_sha,
                subdir, stars, license, avatar_url, status, recipe_json, created_at, updated_at,
                is_private, archived, repo_created_at, owner_created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20)",
            params![
                p.id, p.source, p.name, p.description, p.owner, p.repo, p.ref_name, p.commit,
                p.subdir, p.stars.map(|s| s as i64), p.license, p.avatar_url, p.status.as_str(),
                recipe, p.created_at, p.updated_at, p.is_private, p.archived, p.repo_created_at,
                p.owner_created_at
            ],
        )?;
        Ok(())
    }

    pub fn get(&self, id: &str) -> Result<Project> {
        self.conn()
            .query_row("SELECT * FROM projects WHERE id = ?1", [id], from_row)
            .optional()?
            .ok_or_else(|| Problem::new(ProblemCode::ProjectNotFound).param("id", id))
    }

    pub fn find_by_source(&self, source: &str) -> Result<Option<Project>> {
        Ok(self
            .conn()
            .query_row(
                "SELECT * FROM projects WHERE source = ?1",
                [source],
                from_row,
            )
            .optional()?)
    }

    pub fn list(&self) -> Result<Vec<Project>> {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT * FROM projects ORDER BY created_at DESC, id")?;
        let rows = stmt
            .query_map([], from_row)?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    pub fn set_status(&self, id: &str, status: ProjectStatus) -> Result<()> {
        self.conn().execute(
            "UPDATE projects SET status = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, status.as_str(), now()],
        )?;
        Ok(())
    }

    pub fn set_recipe(&self, id: &str, recipe: &Recipe) -> Result<()> {
        let json = serde_json::to_string(recipe).map_err(Problem::internal)?;
        self.conn().execute(
            "UPDATE projects SET recipe_json = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, json, now()],
        )?;
        Ok(())
    }

    pub fn set_commit(&self, id: &str, commit: &str) -> Result<()> {
        self.conn().execute(
            "UPDATE projects SET commit_sha = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, commit, now()],
        )?;
        Ok(())
    }

    /// Name/description a project's own `launcher.json` asks for.
    pub fn set_display(
        &self,
        id: &str,
        name: Option<&str>,
        description: Option<&str>,
    ) -> Result<()> {
        self.conn().execute(
            "UPDATE projects SET name = COALESCE(?2, name), description = COALESCE(?3, description),
             updated_at = ?4 WHERE id = ?1",
            params![id, name, description, now()],
        )?;
        Ok(())
    }

    pub fn set_consent(&self, id: &str, fingerprint: Option<&str>) -> Result<()> {
        self.conn().execute(
            "UPDATE projects SET consented_fingerprint = ?2, consented_at = ?3 WHERE id = ?1",
            params![id, fingerprint, fingerprint.map(|_| now())],
        )?;
        Ok(())
    }

    pub fn set_keys_reviewed(&self, id: &str) -> Result<()> {
        self.conn()
            .execute("UPDATE projects SET keys_reviewed = 1 WHERE id = ?1", [id])?;
        Ok(())
    }

    pub fn set_owner_created_at(&self, id: &str, created: &str) -> Result<()> {
        self.conn().execute(
            "UPDATE projects SET owner_created_at = ?2 WHERE id = ?1",
            params![id, created],
        )?;
        Ok(())
    }

    pub fn set_start_at_login(&self, id: &str, on: bool) -> Result<()> {
        self.conn().execute(
            "UPDATE projects SET start_at_login = ?2 WHERE id = ?1",
            params![id, on],
        )?;
        Ok(())
    }

    pub fn set_use_system_terminal(&self, id: &str, on: bool) -> Result<()> {
        self.conn().execute(
            "UPDATE projects SET use_system_terminal = ?2 WHERE id = ?1",
            params![id, on],
        )?;
        Ok(())
    }

    /// Removes the project, its plain settings and its run history. Its keychain entries and
    /// shortcut files are removed by the caller first.
    pub fn delete(&self, id: &str) -> Result<()> {
        let conn = self.conn();
        conn.execute("DELETE FROM env_values WHERE project_id = ?1", [id])?;
        conn.execute("DELETE FROM runs WHERE project_id = ?1", [id])?;
        conn.execute("DELETE FROM shortcuts WHERE project_id = ?1", [id])?;
        conn.execute("DELETE FROM projects WHERE id = ?1", [id])?;
        Ok(())
    }

    // --- runs ------------------------------------------------------------------------------

    /// Records a launch; returns the run's id. Old finished runs of the project are pruned.
    pub fn begin_run(&self, r: &RunRecord) -> Result<i64> {
        let conn = self.conn();
        conn.execute(
            "INSERT INTO runs (project_id, pid, pgid, proc_start_ms, boot_ms, url, started_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                r.project_id,
                r.pid,
                r.pgid,
                r.proc_start_ms,
                r.boot_ms,
                r.url,
                now()
            ],
        )?;
        let id = conn.last_insert_rowid();
        conn.execute(
            "DELETE FROM runs WHERE project_id = ?1 AND ended_at IS NOT NULL AND id NOT IN
               (SELECT id FROM runs WHERE project_id = ?1 ORDER BY id DESC LIMIT ?2)",
            params![r.project_id, RUNS_KEPT],
        )?;
        Ok(id)
    }

    pub fn set_run_url(&self, run: i64, url: &str) -> Result<()> {
        self.conn()
            .execute("UPDATE runs SET url = ?2 WHERE id = ?1", params![run, url])?;
        Ok(())
    }

    pub fn end_run(&self, run: i64, exit_code: Option<i32>, reason: EndReason) -> Result<()> {
        self.conn().execute(
            "UPDATE runs SET ended_at = ?2, exit_code = ?3, end_reason = ?4
             WHERE id = ?1 AND ended_at IS NULL",
            params![run, now(), exit_code, reason.as_str()],
        )?;
        Ok(())
    }

    /// Runs that never recorded an end: still going, or the Launcher stopped unexpectedly.
    pub fn open_runs(&self) -> Result<Vec<RunRecord>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT id, project_id, pid, pgid, proc_start_ms, boot_ms, url FROM runs
             WHERE ended_at IS NULL ORDER BY id",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(RunRecord {
                    id: r.get(0)?,
                    project_id: r.get(1)?,
                    pid: r.get(2)?,
                    pgid: r.get(3)?,
                    proc_start_ms: r.get(4)?,
                    boot_ms: r.get(5)?,
                    url: r.get(6)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    /// The newest run of a project: (url, still open).
    pub fn latest_run(&self, project_id: &str) -> Result<Option<(Option<String>, bool)>> {
        Ok(self
            .conn()
            .query_row(
                "SELECT url, ended_at IS NULL FROM runs WHERE project_id = ?1
                 ORDER BY id DESC LIMIT 1",
                [project_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?)
    }

    // --- shortcuts -------------------------------------------------------------------------

    pub fn add_shortcut(&self, s: &Shortcut) -> Result<()> {
        self.conn().execute(
            "INSERT OR REPLACE INTO shortcuts (path, project_id, kind, target, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![s.path, s.project_id, s.kind.as_str(), s.target, now()],
        )?;
        Ok(())
    }

    pub fn remove_shortcut(&self, path: &str) -> Result<()> {
        self.conn()
            .execute("DELETE FROM shortcuts WHERE path = ?1", [path])?;
        Ok(())
    }

    /// Shortcuts of one project, or of every project with `None`.
    pub fn shortcuts(&self, project_id: Option<&str>) -> Result<Vec<Shortcut>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(
            "SELECT project_id, kind, path, target FROM shortcuts
             WHERE ?1 IS NULL OR project_id = ?1 ORDER BY path",
        )?;
        let rows = stmt
            .query_map([project_id], |r| {
                Ok(Shortcut {
                    project_id: r.get(0)?,
                    kind: ShortcutKind::parse(&r.get::<_, String>(1)?),
                    path: r.get(2)?,
                    target: r.get(3)?,
                })
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows)
    }

    // --- keychain index -------------------------------------------------------------------

    pub fn index_secret(&self, key: &str, project_id: Option<&str>) -> Result<()> {
        self.conn().execute(
            "INSERT OR REPLACE INTO secret_keys (key, project_id, created_at) VALUES (?1, ?2, ?3)",
            params![key, project_id, now()],
        )?;
        Ok(())
    }

    pub fn unindex_secret(&self, key: &str) -> Result<()> {
        self.conn()
            .execute("DELETE FROM secret_keys WHERE key = ?1", [key])?;
        Ok(())
    }

    pub fn secret_keys_for(&self, project_id: &str) -> Result<Vec<String>> {
        self.strings(
            "SELECT key FROM secret_keys WHERE project_id = ?1 ORDER BY key",
            [project_id],
        )
    }

    pub fn all_secret_keys(&self) -> Result<Vec<String>> {
        self.strings("SELECT key FROM secret_keys ORDER BY key", [])
    }

    pub fn has_secret(&self, key: &str) -> Result<bool> {
        Ok(self
            .conn()
            .query_row(
                "SELECT 1 FROM secret_keys WHERE key = ?1",
                [key],
                |_| Ok(()),
            )
            .optional()?
            .is_some())
    }

    // --- plain (non-secret) project settings -----------------------------------------------

    pub fn env_values(&self, project_id: &str) -> Result<BTreeMap<String, String>> {
        let conn = self.conn();
        let mut stmt = conn.prepare("SELECT name, value FROM env_values WHERE project_id = ?1")?;
        let rows = stmt
            .query_map([project_id], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<BTreeMap<String, String>>>()?;
        Ok(rows)
    }

    pub fn set_env_value(&self, project_id: &str, name: &str, value: Option<&str>) -> Result<()> {
        let conn = self.conn();
        match value {
            Some(v) => conn.execute(
                "INSERT OR REPLACE INTO env_values (project_id, name, value) VALUES (?1, ?2, ?3)",
                params![project_id, name, v],
            )?,
            None => conn.execute(
                "DELETE FROM env_values WHERE project_id = ?1 AND name = ?2",
                params![project_id, name],
            )?,
        };
        Ok(())
    }

    // --- trusted publishers ----------------------------------------------------------------

    pub fn trusted_publishers(&self) -> Result<Vec<String>> {
        self.strings("SELECT owner FROM trusted_publishers ORDER BY owner", [])
    }

    pub fn is_trusted(&self, owner: &str) -> Result<bool> {
        Ok(self
            .conn()
            .query_row(
                "SELECT 1 FROM trusted_publishers WHERE owner = ?1",
                [owner.to_lowercase()],
                |_| Ok(()),
            )
            .optional()?
            .is_some())
    }

    pub fn set_trusted(&self, owner: &str, trusted: bool) -> Result<()> {
        let owner = owner.to_lowercase();
        let conn = self.conn();
        if trusted {
            conn.execute(
                "INSERT OR IGNORE INTO trusted_publishers (owner, added_at) VALUES (?1, ?2)",
                params![owner, now()],
            )?;
        } else {
            conn.execute("DELETE FROM trusted_publishers WHERE owner = ?1", [owner])?;
        }
        Ok(())
    }

    // --- app settings ----------------------------------------------------------------------

    pub fn setting(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn()
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| {
                r.get(0)
            })
            .optional()?)
    }

    pub fn set_setting(&self, key: &str, value: Option<&str>) -> Result<()> {
        let conn = self.conn();
        match value {
            Some(v) => conn.execute(
                "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
                params![key, v],
            )?,
            None => conn.execute("DELETE FROM settings WHERE key = ?1", [key])?,
        };
        Ok(())
    }

    fn strings<P: rusqlite::Params>(&self, sql: &str, p: P) -> Result<Vec<String>> {
        let conn = self.conn();
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt
            .query_map(p, |r| r.get(0))?
            .collect::<rusqlite::Result<Vec<String>>>()?;
        Ok(rows)
    }
}

fn from_row(r: &Row<'_>) -> rusqlite::Result<Project> {
    let recipe: Option<Recipe> = r
        .get::<_, Option<String>>("recipe_json")?
        .and_then(|j| serde_json::from_str(&j).ok());
    Ok(Project {
        id: r.get("id")?,
        source: r.get("source")?,
        name: r.get("name")?,
        description: r.get("description")?,
        owner: r.get("owner")?,
        repo: r.get("repo")?,
        ref_name: r.get("ref_name")?,
        commit: r.get("commit_sha")?,
        subdir: r.get("subdir")?,
        stars: r.get::<_, Option<i64>>("stars")?.map(|s| s as u64),
        license: r.get("license")?,
        avatar_url: r.get("avatar_url")?,
        is_private: r.get("is_private")?,
        archived: r.get("archived")?,
        repo_created_at: r.get("repo_created_at")?,
        owner_created_at: r.get("owner_created_at")?,
        consented_fingerprint: r.get("consented_fingerprint")?,
        keys_reviewed: r.get("keys_reviewed")?,
        start_at_login: r.get("start_at_login")?,
        use_system_terminal: r.get("use_system_terminal")?,
        status: ProjectStatus::parse(&r.get::<_, String>("status")?),
        kind: recipe.as_ref().map(|rc| rc.kind),
        recipe,
        created_at: r.get("created_at")?,
        updated_at: r.get("updated_at")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(id: &str, source: &str) -> Project {
        Project {
            id: id.into(),
            source: source.into(),
            name: "demo".into(),
            description: None,
            owner: Some("o".into()),
            repo: Some("r".into()),
            ref_name: None,
            commit: None,
            subdir: None,
            stars: Some(5),
            license: None,
            avatar_url: None,
            is_private: false,
            archived: false,
            repo_created_at: None,
            owner_created_at: None,
            consented_fingerprint: None,
            keys_reviewed: false,
            start_at_login: false,
            use_system_terminal: false,
            status: ProjectStatus::Added,
            kind: None,
            recipe: None,
            created_at: 1,
            updated_at: 1,
        }
    }

    #[test]
    fn round_trip() {
        let s = Store::in_memory().unwrap();
        s.insert(&sample("a1", "github:o/r")).unwrap();
        s.set_status("a1", ProjectStatus::Installed).unwrap();
        s.set_commit("a1", "abc").unwrap();
        let p = s.get("a1").unwrap();
        assert_eq!(p.status, ProjectStatus::Installed);
        assert_eq!(p.commit.as_deref(), Some("abc"));
        assert_eq!(p.stars, Some(5));
        assert!(s.find_by_source("github:o/r").unwrap().is_some());
        assert!(
            s.insert(&sample("a2", "github:o/r")).is_err(),
            "source is unique"
        );
        s.delete("a1").unwrap();
        assert_eq!(s.get("a1").unwrap_err().code, ProblemCode::ProjectNotFound);
    }

    #[test]
    fn runs_open_until_ended_and_old_ones_are_pruned() {
        let s = Store::in_memory().unwrap();
        let rec = |pid| RunRecord {
            id: 0,
            project_id: "a1".into(),
            pid: Some(pid),
            pgid: Some(pid),
            proc_start_ms: Some(1000),
            boot_ms: None,
            url: None,
        };
        let first = s.begin_run(&rec(10)).unwrap();
        s.set_run_url(first, "http://127.0.0.1:8501/").unwrap();
        let open = s.open_runs().unwrap();
        assert_eq!(open.len(), 1);
        assert_eq!(open[0].url.as_deref(), Some("http://127.0.0.1:8501/"));
        s.end_run(first, Some(0), EndReason::Exited).unwrap();
        assert!(s.open_runs().unwrap().is_empty());
        for pid in 0..30 {
            let id = s.begin_run(&rec(pid)).unwrap();
            s.end_run(id, None, EndReason::UserStop).unwrap();
        }
        let kept: i64 = s
            .conn()
            .query_row("SELECT COUNT(*) FROM runs", [], |r| r.get(0))
            .unwrap();
        assert!(kept <= RUNS_KEPT + 1, "{kept}");
    }

    #[test]
    fn shortcuts_are_indexed_per_project() {
        let s = Store::in_memory().unwrap();
        s.insert(&sample("a1", "github:o/r")).unwrap();
        let sc = Shortcut {
            project_id: "a1".into(),
            kind: ShortcutKind::Desktop,
            path: "/x/demo.lnk".into(),
            target: "/app".into(),
        };
        s.add_shortcut(&sc).unwrap();
        assert_eq!(s.shortcuts(Some("a1")).unwrap(), vec![sc.clone()]);
        assert_eq!(s.shortcuts(None).unwrap().len(), 1);
        assert!(s.shortcuts(Some("zz")).unwrap().is_empty());
        s.delete("a1").unwrap();
        assert!(s.shortcuts(None).unwrap().is_empty());
    }

    #[test]
    fn migrations_are_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let db = dir.path().join("s.db");
        Store::open(&db)
            .unwrap()
            .insert(&sample("x", "local:/x"))
            .unwrap();
        assert_eq!(Store::open(&db).unwrap().list().unwrap().len(), 1);
    }
}
