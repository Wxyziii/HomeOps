use crate::ApiError;
use serde::{Deserialize, Serialize};

pub const MAX_DOWNLOAD_BYTES: u64 = 1024 * 1024 * 1024;

/// Hosts a .mrpack manifest may reference, per the Modrinth pack format spec.
const ALLOWED_DOWNLOAD_HOSTS: &[&str] = &[
    "cdn.modrinth.com",
    "github.com",
    "raw.githubusercontent.com",
    "gitlab.com",
    "meta.fabricmc.net",
    "maven.fabricmc.net",
];

pub fn http_client() -> Result<reqwest::Client, ApiError> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .user_agent("HomeOpsPanel/0.1 (server-agent; https://github.com/Wxyziii/HomeOps)")
        .build()
        .map_err(|error| ApiError::internal("HTTP_CLIENT_FAILED", error.to_string()))
}

pub fn is_allowed_download_url(url: &str) -> bool {
    let Ok(parsed) = reqwest::Url::parse(url) else {
        return false;
    };
    if parsed.scheme() != "https" {
        return false;
    }
    parsed
        .host_str()
        .is_some_and(|host| ALLOWED_DOWNLOAD_HOSTS.contains(&host))
}

pub fn validate_project_id(project: &str) -> Result<String, ApiError> {
    let trimmed = project.trim();
    let candidate = trimmed
        .strip_prefix("https://modrinth.com/mod/")
        .or_else(|| trimmed.strip_prefix("https://www.modrinth.com/mod/"))
        .or_else(|| trimmed.strip_prefix("https://modrinth.com/modpack/"))
        .or_else(|| trimmed.strip_prefix("https://www.modrinth.com/modpack/"))
        .unwrap_or(trimmed)
        .trim_matches('/');
    let valid = !candidate.is_empty()
        && candidate.len() <= 100
        && candidate
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'));
    if valid {
        Ok(candidate.to_string())
    } else {
        Err(ApiError::bad_request(
            "INVALID_PROJECT",
            "Provide a Modrinth project slug, project id, or modrinth.com URL.",
        ))
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub downloads: u64,
    pub follows: u64,
    pub author: String,
    pub categories: Vec<String>,
    pub latest_version: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
    pub ok: bool,
    pub query: String,
    pub project_type: String,
    pub total_hits: u64,
    pub offset: u64,
    pub hits: Vec<SearchHit>,
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
    pub query: Option<String>,
    #[serde(rename = "type")]
    pub project_type: Option<String>,
    pub offset: Option<u64>,
    pub game_version: Option<String>,
}

pub async fn search(query: SearchQuery) -> Result<SearchResponse, ApiError> {
    let text = query.query.unwrap_or_default().trim().to_string();
    if text.len() > 100 {
        return Err(ApiError::bad_request(
            "INVALID_QUERY",
            "Search query is too long.",
        ));
    }
    let project_type = match query.project_type.as_deref().unwrap_or("mod") {
        "mod" => "mod",
        "modpack" => "modpack",
        _ => {
            return Err(ApiError::bad_request(
                "INVALID_TYPE",
                "type must be 'mod' or 'modpack'",
            ));
        }
    };
    let offset = query.offset.unwrap_or(0).min(1000);

    let mut facets = vec![
        vec![format!("project_type:{project_type}")],
        vec!["categories:fabric".to_string()],
    ];
    if let Some(game_version) = query
        .game_version
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if !game_version
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_'))
        {
            return Err(ApiError::bad_request(
                "INVALID_GAME_VERSION",
                "Invalid game version filter.",
            ));
        }
        facets.push(vec![format!("versions:{game_version}")]);
    }
    let facets_json = serde_json::to_string(&facets)
        .map_err(|error| ApiError::internal("MODRINTH_REQUEST_FAILED", error.to_string()))?;

    let client = http_client()?;
    let url = reqwest::Url::parse_with_params(
        "https://api.modrinth.com/v2/search",
        [
            ("query", text.as_str()),
            ("facets", facets_json.as_str()),
            ("limit", "20"),
            ("offset", &offset.to_string()),
            (
                "index",
                if text.is_empty() {
                    "downloads"
                } else {
                    "relevance"
                },
            ),
        ],
    )
    .map_err(|error| ApiError::internal("MODRINTH_REQUEST_FAILED", error.to_string()))?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| ApiError::internal("MODRINTH_REQUEST_FAILED", error.to_string()))?;
    if !response.status().is_success() {
        return Err(ApiError::internal(
            "MODRINTH_REQUEST_FAILED",
            format!("Modrinth search returned HTTP {}", response.status()),
        ));
    }
    let body: serde_json::Value = response
        .json()
        .await
        .map_err(|error| ApiError::internal("MODRINTH_RESPONSE_INVALID", error.to_string()))?;

    let hits = body
        .get("hits")
        .and_then(|value| value.as_array())
        .map(|hits| {
            hits.iter()
                .filter_map(|hit| {
                    Some(SearchHit {
                        project_id: hit.get("project_id")?.as_str()?.to_string(),
                        slug: hit.get("slug")?.as_str()?.to_string(),
                        title: hit.get("title")?.as_str()?.to_string(),
                        description: hit
                            .get("description")
                            .and_then(|value| value.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        icon_url: hit
                            .get("icon_url")
                            .and_then(|value| value.as_str())
                            .filter(|value| value.starts_with("https://cdn.modrinth.com/"))
                            .map(str::to_string),
                        downloads: hit
                            .get("downloads")
                            .and_then(|value| value.as_u64())
                            .unwrap_or(0),
                        follows: hit
                            .get("follows")
                            .and_then(|value| value.as_u64())
                            .unwrap_or(0),
                        author: hit
                            .get("author")
                            .and_then(|value| value.as_str())
                            .unwrap_or_default()
                            .to_string(),
                        categories: hit
                            .get("categories")
                            .and_then(|value| value.as_array())
                            .map(|values| {
                                values
                                    .iter()
                                    .filter_map(|value| value.as_str().map(str::to_string))
                                    .collect()
                            })
                            .unwrap_or_default(),
                        latest_version: hit
                            .get("versions")
                            .and_then(|value| value.as_array())
                            .and_then(|values| values.last())
                            .and_then(|value| value.as_str())
                            .map(str::to_string),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(SearchResponse {
        ok: true,
        query: text,
        project_type: project_type.to_string(),
        total_hits: body
            .get("total_hits")
            .and_then(|value| value.as_u64())
            .unwrap_or(0),
        offset,
        hits,
    })
}

#[derive(Debug, Clone)]
pub struct ResolvedVersionFile {
    pub version_number: String,
    pub file_name: String,
    pub url: String,
    pub size: u64,
}

/// Resolve the newest compatible version file of a Modrinth project.
pub async fn resolve_version_file(
    client: &reqwest::Client,
    project: &str,
    project_kind: &str,
    game_version: Option<&str>,
) -> Result<ResolvedVersionFile, ApiError> {
    let mut url =
        format!("https://api.modrinth.com/v2/project/{project}/version?loaders=%5B%22fabric%22%5D");
    if let Some(game_version) = game_version {
        url.push_str(&format!("&game_versions=%5B%22{game_version}%22%5D"));
    }
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|error| ApiError::internal("MODRINTH_REQUEST_FAILED", error.to_string()))?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Err(ApiError::not_found(
            "PROJECT_NOT_FOUND",
            "Modrinth project was not found.",
        ));
    }
    if !response.status().is_success() {
        return Err(ApiError::internal(
            "MODRINTH_REQUEST_FAILED",
            format!("Modrinth returned HTTP {}", response.status()),
        ));
    }
    let versions: Vec<serde_json::Value> = response
        .json()
        .await
        .map_err(|error| ApiError::internal("MODRINTH_RESPONSE_INVALID", error.to_string()))?;
    let version = versions.first().ok_or_else(|| {
        ApiError::not_found(
            "NO_COMPATIBLE_VERSION",
            format!(
                "No compatible Fabric {project_kind} version found{}.",
                game_version
                    .map(|value| format!(" for Minecraft {value}"))
                    .unwrap_or_default()
            ),
        )
    })?;

    let files = version
        .get("files")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let file = files
        .iter()
        .find(|file| {
            file.get("primary")
                .and_then(|value| value.as_bool())
                .unwrap_or(false)
        })
        .or_else(|| files.first())
        .ok_or_else(|| {
            ApiError::internal(
                "MODRINTH_RESPONSE_INVALID",
                "Version has no downloadable files.",
            )
        })?;
    let url = file
        .get("url")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
        .to_string();
    if !url.starts_with("https://cdn.modrinth.com/") {
        return Err(ApiError::forbidden(
            "UNTRUSTED_DOWNLOAD_HOST",
            "Refusing to download from a non-Modrinth CDN host.",
        ));
    }
    Ok(ResolvedVersionFile {
        version_number: version
            .get("version_number")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown")
            .to_string(),
        file_name: file
            .get("filename")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string(),
        url,
        size: file
            .get("size")
            .and_then(|value| value.as_u64())
            .unwrap_or(0),
    })
}

pub async fn download_to_file(
    client: &reqwest::Client,
    url: &str,
    target: &std::path::Path,
    max_bytes: u64,
) -> Result<u64, String> {
    if !is_allowed_download_url(url) {
        return Err(format!("download host is not allowed: {url}"));
    }
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("download returned HTTP {}", response.status()));
    }
    let bytes = response.bytes().await.map_err(|error| error.to_string())?;
    if bytes.len() as u64 > max_bytes {
        return Err("download exceeds the configured size limit".to_string());
    }
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)
        .map_err(|error| error.to_string())?;
    std::io::Write::write_all(&mut file, &bytes).map_err(|error| error.to_string())?;
    Ok(bytes.len() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_only_https_allowlisted_hosts() {
        assert!(is_allowed_download_url(
            "https://cdn.modrinth.com/data/abc/file.jar"
        ));
        assert!(is_allowed_download_url(
            "https://github.com/user/repo/releases/x.jar"
        ));
        assert!(!is_allowed_download_url("http://cdn.modrinth.com/file.jar"));
        assert!(!is_allowed_download_url(
            "https://evil.example.com/file.jar"
        ));
        assert!(!is_allowed_download_url(
            "https://cdn.modrinth.com.evil.com/file.jar"
        ));
        assert!(!is_allowed_download_url("file:///etc/passwd"));
    }

    #[test]
    fn project_id_validation_accepts_urls_and_slugs() {
        assert_eq!(validate_project_id("fabric-api").unwrap(), "fabric-api");
        assert_eq!(
            validate_project_id("https://modrinth.com/modpack/adrenaline").unwrap(),
            "adrenaline"
        );
        assert!(validate_project_id("bad slug!").is_err());
        assert!(validate_project_id("https://evil.com/mod/x").is_err());
    }
}
