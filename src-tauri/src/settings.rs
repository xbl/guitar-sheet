use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};
use crate::paths::AppPaths;

const FILE_NAME: &str = "github_settings.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubSettings {
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub repo: String,
    #[serde(default = "default_branch")]
    pub branch: String,
    /// Directory prefix inside the repo, e.g. `guitar-sheet/` or empty for repo root.
    #[serde(default)]
    pub prefix: String,
}

fn default_branch() -> String {
    "main".to_string()
}

impl Default for GitHubSettings {
    fn default() -> Self {
        Self {
            owner: String::new(),
            repo: String::new(),
            branch: default_branch(),
            prefix: String::new(),
        }
    }
}

impl GitHubSettings {
    /// Prefix used in remote paths; always non-empty and ends with `/`.
    pub fn normalized_prefix(&self) -> String {
        let mut p = self.prefix.trim().to_string();
        if p.is_empty() {
            return "sheets/".to_string();
        }
        if !p.ends_with('/') {
            p.push('/');
        }
        p
    }

    pub fn is_complete_for_api(&self) -> bool {
        !self.owner.trim().is_empty() && !self.repo.trim().is_empty()
    }
}

pub fn load(paths: &AppPaths) -> AppResult<GitHubSettings> {
    let p = paths.data_dir.join(FILE_NAME);
    if !p.exists() {
        return Ok(GitHubSettings::default());
    }
    let s = std::fs::read_to_string(&p).map_err(AppError::Io)?;
    let v: GitHubSettings = serde_json::from_str(&s).map_err(AppError::Json)?;
    Ok(v)
}

pub fn save(paths: &AppPaths, settings: &GitHubSettings) -> AppResult<()> {
    let p = paths.data_dir.join(FILE_NAME);
    let body = serde_json::to_string_pretty(settings).map_err(AppError::Json)?;
    std::fs::write(&p, body).map_err(AppError::Io)?;
    Ok(())
}
