#![allow(non_snake_case)]

use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{Context, Result};
use semver::Version;
use tokio::process::Command;

pub const REPOSITORY_URL: &str = "https://github.com/echo1097/echo-build.git";
const INSTALLER_URL: &str = "https://raw.githubusercontent.com/echo1097/echo-build/main/install.sh";

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceUpdateStatus {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub repository: &'static str,
}

fn dataHome() -> PathBuf {
    if let Some(path) = std::env::var_os("XDG_DATA_HOME") {
        return PathBuf::from(path);
    }

    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".local/share")
}

fn managedInstaller() -> PathBuf {
    dataHome().join("echo-build/source/install.sh")
}

pub fn parseVersionTag(tag: &str) -> Result<Version> {
    let version = tag
        .strip_prefix('v')
        .ok_or_else(|| anyhow::anyhow!("release versions must start with 'v'"))?;
    let parsed = Version::parse(version).context("invalid SemVer release tag")?;

    if parsed.build.is_empty() {
        Ok(parsed)
    } else {
        anyhow::bail!("release tags cannot contain SemVer build metadata")
    }
}

pub async fn latestStableTag() -> Result<String> {
    eprintln!("Querying {REPOSITORY_URL} for stable release tags...");
    let output = Command::new("git")
        .args([
            "-c",
            "http.followRedirects=false",
            "ls-remote",
            "--tags",
            "--refs",
            REPOSITORY_URL,
            "refs/tags/v*",
        ])
        .stdin(Stdio::null())
        .output()
        .await
        .context("Git is required to check Echo Build releases")?;

    if !output.status.success() {
        anyhow::bail!(
            "GitHub release query failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split_whitespace().nth(1))
        .filter_map(|reference| reference.strip_prefix("refs/tags/"))
        .filter_map(|tag| parseVersionTag(tag).ok().map(|version| (tag, version)))
        .filter(|(_, version)| version.pre.is_empty())
        .max_by(|(_, left), (_, right)| left.cmp(right))
        .map(|(tag, _)| tag.to_string())
        .ok_or_else(|| anyhow::anyhow!("no stable Echo Build release tags were found"))
}

pub async fn check() -> Result<SourceUpdateStatus> {
    let currentVersion = Version::parse(xai_grok_version::VERSION)
        .context("installed Echo Build version is not SemVer")?;
    let latestTag = latestStableTag().await?;
    let latestVersion = parseVersionTag(&latestTag)?;

    Ok(SourceUpdateStatus {
        current_version: currentVersion.to_string(),
        latest_version: latestVersion.to_string(),
        update_available: latestVersion > currentVersion,
        repository: REPOSITORY_URL,
    })
}

async fn downloadInstaller(path: &Path) -> Result<()> {
    eprintln!("Downloading installer from {INSTALLER_URL}...");
    let status = Command::new("curl")
        .args([
            "--proto",
            "=https",
            "--tlsv1.2",
            "--fail",
            "--show-error",
            "--output",
        ])
        .arg(path)
        .arg(INSTALLER_URL)
        .stdin(Stdio::null())
        .status()
        .await
        .context("curl is required when the managed installer is unavailable")?;

    if !status.success() {
        anyhow::bail!("could not download the Echo-owned installer")
    }
    Ok(())
}

pub async fn install(versionTag: Option<&str>, allowDowngrade: bool) -> Result<()> {
    if let Some(tag) = versionTag {
        parseVersionTag(tag)?;
    }

    let managedPath = managedInstaller();
    let tempPath = std::env::temp_dir().join(format!(
        "echo-build-install-{}-{}.sh",
        std::process::id(),
        time::OffsetDateTime::now_utc().unix_timestamp()
    ));
    let installerPath = if managedPath.is_file() {
        managedPath
    } else {
        downloadInstaller(&tempPath).await?;
        tempPath.clone()
    };

    let mut command = Command::new("sh");
    command.arg(&installerPath);
    if let Some(tag) = versionTag {
        command.args(["--version", tag]);
    }
    if allowDowngrade {
        command.arg("--allow-downgrade");
    }
    command.stdin(Stdio::inherit());
    command.stdout(Stdio::inherit());
    command.stderr(Stdio::inherit());

    let status = command.status().await.context("failed to run install.sh")?;
    let _ = tokio::fs::remove_file(&tempPath).await;
    if !status.success() {
        anyhow::bail!("Echo Build installer exited with {status}")
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parseVersionTag;

    #[test]
    fn acceptsStableAndPrereleaseTags() {
        assert!(parseVersionTag("v1.2.3").is_ok());
        assert!(parseVersionTag("v0.3.0-beta.1").is_ok());
    }

    #[test]
    fn rejectsBranchesAndMalformedVersions() {
        for value in ["main", "1.2.3", "v1", "v01.2.3", "v1.2.3+local"] {
            assert!(parseVersionTag(value).is_err(), "accepted {value}");
        }
    }
}
