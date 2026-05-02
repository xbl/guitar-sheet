use base64::Engine;
use reqwest::blocking::Client;
use serde::Deserialize;

use crate::error::{AppError, AppResult};

const ACCEPT: &str = "application/vnd.github+json";

#[derive(Debug, Deserialize)]
struct ContentsFile {
    sha: String,
    content: Option<String>,
    encoding: Option<String>,
}

#[derive(Debug)]
pub struct RemoteFile {
    pub sha: String,
    pub bytes: Vec<u8>,
}

pub struct GitHubRest {
    client: Client,
    token: String,
    owner: String,
    repo: String,
    pub branch: String,
}

impl GitHubRest {
    pub fn new(token: String, owner: String, repo: String, branch: String) -> AppResult<Self> {
        let client = Client::builder()
            .user_agent("guitar-sheet/0.1 (Tauri)")
            .build()
            .map_err(|e| AppError::Http(e.to_string()))?;
        Ok(Self {
            client,
            token,
            owner,
            repo,
            branch,
        })
    }

    fn contents_url(&self, path: &str) -> String {
        format!(
            "https://api.github.com/repos/{}/{}/contents/{}",
            self.owner, self.repo, path
        )
    }

    /// `None` when the path does not exist (404).
    pub fn get_contents(&self, path: &str) -> AppResult<Option<RemoteFile>> {
        let url = self.contents_url(path);
        let resp = self
            .client
            .get(&url)
            .header("Accept", ACCEPT)
            .header("Authorization", format!("Bearer {}", self.token))
            .query(&[("ref", self.branch.as_str())])
            .send()
            .map_err(|e| AppError::Http(e.to_string()))?;

        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        if !resp.status().is_success() {
            let body = resp.text().unwrap_or_default();
            return Err(AppError::Http(format!(
                "GitHub GET {}: {}",
                path, body
            )));
        }

        let parsed: ContentsFile = resp.json().map_err(|e| AppError::Http(e.to_string()))?;
        let encoding = parsed.encoding.as_deref().unwrap_or("");
        let content_b64 = parsed.content.ok_or_else(|| {
            AppError::Http("GitHub response missing content (large file / unsupported)".into())
        })?;
        if !encoding.eq_ignore_ascii_case("base64") {
            return Err(AppError::Http(format!(
                "unexpected encoding from GitHub: {encoding}"
            )));
        }
        let cleaned: String = content_b64.chars().filter(|c| !c.is_whitespace()).collect();
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(cleaned.as_bytes())
            .map_err(|e| AppError::Http(format!("base64 decode: {e}")))?;
        Ok(Some(RemoteFile {
            sha: parsed.sha,
            bytes,
        }))
    }

    pub fn put_contents(
        &self,
        path: &str,
        message: &str,
        bytes: &[u8],
        previous_sha: Option<&str>,
    ) -> AppResult<String> {
        let url = self.contents_url(path);
        let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
        let mut body = serde_json::json!({
            "message": message,
            "content": b64,
        });
        if let Some(sha) = previous_sha {
            body["sha"] = serde_json::Value::String(sha.to_string());
        }
        let resp = self
            .client
            .put(&url)
            .header("Accept", ACCEPT)
            .header("Authorization", format!("Bearer {}", self.token))
            .json(&body)
            .send()
            .map_err(|e| AppError::Http(e.to_string()))?;

        if !resp.status().is_success() {
            let text = resp.text().unwrap_or_default();
            return Err(AppError::Http(format!(
                "GitHub PUT {}: {}",
                path, text
            )));
        }
        let v: serde_json::Value = resp.json().map_err(|e| AppError::Http(e.to_string()))?;
        let new_sha = v
            .pointer("/content/sha")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| AppError::Http("GitHub PUT response missing content.sha".into()))?;
        Ok(new_sha)
    }
}
