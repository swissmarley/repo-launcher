//! Local state in SQLite. Schema changes are appended to `MIGRATIONS`; `PRAGMA user_version`
//! records how many have run.

use crate::problem::{Problem, ProblemCode, Result};
use crate::recipe::{AppKind, Recipe};
use rusqlite::{Connection, OptionalExtension, Row, params};
use serde::Serialize;
use std::path::Path;
use std::sync::Mutex;

const MIGRATIONS: &[&str] = &[r#"
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
"#];

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
    pub stars: Option<u64>,
    pub license: Option<String>,
    pub avatar_url: Option<String>,
    pub status: ProjectStatus,
    pub kind: Option<AppKind>,
    #[serde(skip)]
    pub recipe: Option<Recipe>,
    pub created_at: i64,
    pub updated_at: i64,
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
                subdir, stars, license, avatar_url, status, recipe_json, created_at, updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
            params![
                p.id, p.source, p.name, p.description, p.owner, p.repo, p.ref_name, p.commit,
                p.subdir, p.stars.map(|s| s as i64), p.license, p.avatar_url, p.status.as_str(),
                recipe, p.created_at, p.updated_at
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

    pub fn delete(&self, id: &str) -> Result<()> {
        self.conn()
            .execute("DELETE FROM projects WHERE id = ?1", [id])?;
        Ok(())
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
