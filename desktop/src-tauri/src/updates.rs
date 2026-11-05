use serde::{Deserialize, Serialize};
use std::time::Duration;

const RELEASES: &str = "https://api.github.com/repos/hypler-dev/OpenDrobo/releases/latest";
const RELEASE_PREFIX: &str = "https://github.com/hypler-dev/OpenDrobo/releases/tag/";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateResponse {
    state: &'static str,
    message: String,
    release_url: Option<String>,
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    html_url: String,
    draft: bool,
    prerelease: bool,
}

pub fn validate_release_url(url: &str) -> Result<(), String> {
    let tag = url
        .strip_prefix(RELEASE_PREFIX)
        .ok_or("Release link is outside OpenDrobo")?;
    version(tag).ok_or("Release link has an unsupported version")?;
    Ok(())
}

fn version(tag: &str) -> Option<Vec<u64>> {
    let values = tag
        .strip_prefix('v')
        .unwrap_or(tag)
        .split('.')
        .map(|part| {
            if part.is_empty() || !part.bytes().all(|c| c.is_ascii_digit()) {
                return None;
            }
            part.parse().ok()
        })
        .collect::<Option<Vec<_>>>()?;
    if values.len() != 3 {
        return None;
    }
    Some(values)
}

fn evaluate(status: u16, body: &[u8]) -> Result<UpdateResponse, String> {
    if status == 404 {
        return Ok(UpdateResponse {
            state: "none",
            message: "No app releases have been published yet.".into(),
            release_url: None,
        });
    }
    if status != 200 {
        return Err(format!("Release lookup failed (HTTP {status})."));
    }
    let release: Release = serde_json::from_slice(body).map_err(|_| "Invalid release response")?;
    if release.draft || release.prerelease {
        return Err("Only stable published releases are supported.".into());
    }
    let released = version(&release.tag_name).ok_or("Release tag is not a supported version")?;
    validate_release_url(&release.html_url)?;
    if release.html_url != format!("{RELEASE_PREFIX}{}", release.tag_name) {
        return Err("Release page and tag disagree.".into());
    }
    let installed = version(env!("CARGO_PKG_VERSION")).ok_or("Invalid installed version")?;
    if released > installed {
        Ok(UpdateResponse {
            state: "available",
            message: format!("OpenDrobo {} is available.", release.tag_name),
            release_url: Some(release.html_url),
        })
    } else {
        Ok(UpdateResponse {
            state: "current",
            message: "This build is current.".into(),
            release_url: None,
        })
    }
}

pub async fn check() -> UpdateResponse {
    let result = async {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(12))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent("OpenDrobo/0.2.0")
            .build()
            .map_err(|_| "Could not start release lookup".to_string())?;
        let mut response = client
            .get(RELEASES)
            .send()
            .await
            .map_err(|_| "Release lookup could not connect".to_string())?;
        let status = response.status().as_u16();
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| "Incomplete release response".to_string())?
        {
            if bytes.len() + chunk.len() > 1_048_576 {
                return Err("Release response is too large".into());
            }
            bytes.extend_from_slice(&chunk);
        }
        evaluate(status, &bytes)
    }
    .await;
    result.unwrap_or_else(|message: String| UpdateResponse {
        state: "error",
        message,
        release_url: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn release_policy_is_project_scoped_and_numeric() {
        assert!(version("v0.10.0") > version("v0.2.0"));
        for invalid in ["latest", "0.2", "0.2.0-beta", "0.2.0?x=1", "0.2.0/other"] {
            assert!(version(invalid).is_none());
        }
        assert!(validate_release_url("https://evil.example/releases/tag/v1.0.0").is_err());
        assert!(
            validate_release_url("https://github.com/other/project/releases/tag/v1.0.0").is_err()
        );
        assert!(validate_release_url(&format!("{RELEASE_PREFIX}v1.0.0")).is_ok());
        assert_eq!(evaluate(404, b"").unwrap().state, "none");
        assert!(evaluate(429, b"").is_err());
        assert!(evaluate(200, b"not JSON").is_err());
        for draft in [true, false] {
            let body = serde_json::json!({"tag_name":"v1.0.0","html_url":format!("{RELEASE_PREFIX}v1.0.0"),"draft":draft,"prerelease":false});
            assert_eq!(evaluate(200, body.to_string().as_bytes()).is_ok(), !draft);
        }
    }
}
