//! Explicit import of supported non-secret Grok Build state.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub const MIGRATION_VERSION: u32 = 1;
pub const MANIFEST_FILE: &str = "grok-import-v1.json";

const FILE_RESOURCES: &[&str] = &["config.toml", "pager.toml"];
const DIRECTORY_RESOURCES: &[&str] = &["themes", "skills", "personas", "roles"];
const NEVER_IMPORT: &[&str] = &[
    "auth.json",
    "mcp_credentials.json",
    "telemetry.json",
    "leader.json",
    "leader.sock",
    "locks",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictPolicy {
    KeepDestination,
    UseSource,
}

#[derive(Debug, Clone)]
pub struct MigrationOptions {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub dry_run: bool,
    pub skip: bool,
    pub include_memory: bool,
    pub conflict_policy: ConflictPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ResourceStatus {
    Copied,
    WouldCopy,
    AlreadyImported,
    KeptDestination,
    Missing,
    Quarantined,
    WouldQuarantine,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceResult {
    pub resource: String,
    pub status: ResourceStatus,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationManifest {
    pub version: u32,
    pub source_path: PathBuf,
    pub destination_path: PathBuf,
    pub timestamp_unix_seconds: u64,
    pub dry_run: bool,
    pub skipped: bool,
    pub resources: Vec<ResourceResult>,
}

#[derive(Debug, thiserror::Error)]
pub enum MigrationError {
    #[error("migration source and destination must be different")]
    SameTree,
    #[error("migration path escapes its expected root: {0}")]
    UnsafePath(PathBuf),
    #[error("migration filesystem error: {0}")]
    Io(#[from] std::io::Error),
    #[error("migration manifest error: {0}")]
    Manifest(#[from] serde_json::Error),
    #[error("cannot safely import {path}: {source}")]
    UnsafeConfig {
        path: PathBuf,
        source: toml::de::Error,
    },
}

pub fn default_legacy_home() -> Option<PathBuf> {
    #[allow(deprecated)]
    std::env::home_dir().map(|home| home.join(".grok"))
}

pub fn migrate(options: &MigrationOptions) -> Result<MigrationManifest, MigrationError> {
    if paths_match(&options.source, &options.destination) {
        return Err(MigrationError::SameTree);
    }

    let mut manifest = MigrationManifest {
        version: MIGRATION_VERSION,
        source_path: options.source.clone(),
        destination_path: options.destination.clone(),
        timestamp_unix_seconds: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        dry_run: options.dry_run,
        skipped: options.skip,
        resources: Vec::new(),
    };

    if options.skip {
        return Ok(manifest);
    }

    for resource in FILE_RESOURCES {
        import_resource(options, resource, false, &mut manifest)?;
    }
    for resource in DIRECTORY_RESOURCES {
        import_resource(options, resource, false, &mut manifest)?;
    }

    if options.include_memory {
        import_resource(options, "memory", false, &mut manifest)?;
    } else {
        push_skipped(
            &mut manifest,
            "memory",
            "requires explicit --include-memory",
        );
    }

    // executable resources need a fresh trust decision and stay outside active discovery
    import_resource(options, "hooks", true, &mut manifest)?;
    import_resource(options, "plugins", true, &mut manifest)?;

    for resource in NEVER_IMPORT {
        if options.source.join(resource).exists() {
            push_skipped(
                &mut manifest,
                resource,
                "secret or ephemeral state is never imported",
            );
        }
    }

    if !options.dry_run {
        std::fs::create_dir_all(&options.destination)?;
        let manifest_path = options.destination.join(MANIFEST_FILE);
        let contents = serde_json::to_string_pretty(&manifest)?;
        crate::fs_atomic::write_atomically(&manifest_path, &contents, Some(0o600))?;
    }

    Ok(manifest)
}

fn import_resource(
    options: &MigrationOptions,
    resource: &str,
    quarantine: bool,
    manifest: &mut MigrationManifest,
) -> Result<(), MigrationError> {
    let source = checked_child(&options.source, resource)?;
    let relative_destination = if quarantine {
        PathBuf::from("imports/grok/requires-trust").join(resource)
    } else {
        PathBuf::from(resource)
    };
    let destination = checked_child_path(&options.destination, &relative_destination)?;

    if !source.exists() {
        manifest.resources.push(ResourceResult {
            resource: resource.to_owned(),
            status: ResourceStatus::Missing,
            detail: None,
        });
        return Ok(());
    }

    if destination.exists() && options.conflict_policy == ConflictPolicy::KeepDestination {
        manifest.resources.push(ResourceResult {
            resource: resource.to_owned(),
            status: ResourceStatus::KeptDestination,
            detail: None,
        });
        return Ok(());
    }

    if options.dry_run {
        manifest.resources.push(ResourceResult {
            resource: resource.to_owned(),
            status: if quarantine {
                ResourceStatus::WouldQuarantine
            } else {
                ResourceStatus::WouldCopy
            },
            detail: quarantine
                .then(|| "requires a new folder trust decision before activation".into()),
        });
        return Ok(());
    }

    if matches!(resource, "config.toml" | "pager.toml") {
        copy_sanitized_toml(&source, &destination)?;
    } else {
        copy_entry_atomically(&source, &destination)?;
    }
    manifest.resources.push(ResourceResult {
        resource: resource.to_owned(),
        status: if quarantine {
            ResourceStatus::Quarantined
        } else {
            ResourceStatus::Copied
        },
        detail: quarantine.then(|| "requires a new folder trust decision before activation".into()),
    });
    Ok(())
}

fn copy_sanitized_toml(source: &Path, destination: &Path) -> Result<(), MigrationError> {
    let contents = std::fs::read_to_string(source)?;
    let mut config: toml::Value =
        toml::from_str(&contents).map_err(|parse_error| MigrationError::UnsafeConfig {
            path: source.to_path_buf(),
            source: parse_error,
        })?;
    remove_secret_fields(&mut config);

    let parent = destination
        .parent()
        .ok_or_else(|| MigrationError::UnsafePath(destination.into()))?;
    std::fs::create_dir_all(parent)?;
    crate::fs_atomic::write_atomically(destination, &config.to_string(), Some(0o600))?;
    Ok(())
}

fn remove_secret_fields(value: &mut toml::Value) {
    match value {
        toml::Value::Table(table) => {
            table.retain(|key, _| !is_secret_field(key));
            for (_, child) in table.iter_mut() {
                remove_secret_fields(child);
            }
        }
        toml::Value::Array(values) => {
            for child in values {
                remove_secret_fields(child);
            }
        }
        _ => {}
    }
}

fn is_secret_field(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    key.contains("api_key")
        || key.contains("apikey")
        || key.ends_with("token")
        || key.contains("password")
        || key.contains("secret")
        || matches!(key.as_str(), "authorization" | "deployment_key")
}

fn copy_entry_atomically(source: &Path, destination: &Path) -> Result<(), MigrationError> {
    let parent = destination
        .parent()
        .ok_or_else(|| MigrationError::UnsafePath(destination.into()))?;
    std::fs::create_dir_all(parent)?;
    let staging = parent.join(format!(
        ".echo-import-{}.tmp",
        destination
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
    ));
    if staging.exists() {
        if staging.is_dir() {
            std::fs::remove_dir_all(&staging)?;
        } else {
            std::fs::remove_file(&staging)?;
        }
    }

    copy_entry(source, &staging)?;
    if destination.exists() {
        if destination.is_dir() {
            std::fs::remove_dir_all(destination)?;
        } else {
            std::fs::remove_file(destination)?;
        }
    }
    std::fs::rename(&staging, destination)?;
    tighten_config_permissions(destination);
    Ok(())
}

fn copy_entry(source: &Path, destination: &Path) -> Result<(), MigrationError> {
    let metadata = std::fs::symlink_metadata(source)?;
    if metadata.file_type().is_symlink() {
        return Err(MigrationError::UnsafePath(source.into()));
    }
    if metadata.is_dir() {
        std::fs::create_dir(destination)?;
        std::fs::set_permissions(destination, metadata.permissions())?;
        for entry in std::fs::read_dir(source)? {
            let entry = entry?;
            copy_entry(&entry.path(), &destination.join(entry.file_name()))?;
        }
    } else if metadata.is_file() {
        std::fs::copy(source, destination)?;
        std::fs::set_permissions(destination, metadata.permissions())?;
        tighten_config_permissions(destination);
    }
    Ok(())
}

#[cfg(unix)]
fn tighten_config_permissions(path: &Path) {
    use std::os::unix::fs::PermissionsExt as _;
    if matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("config.toml" | "pager.toml")
    ) {
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
}

#[cfg(not(unix))]
fn tighten_config_permissions(_path: &Path) {}

fn checked_child(root: &Path, child: &str) -> Result<PathBuf, MigrationError> {
    checked_child_path(root, Path::new(child))
}

fn checked_child_path(root: &Path, child: &Path) -> Result<PathBuf, MigrationError> {
    if child.is_absolute()
        || child
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err(MigrationError::UnsafePath(child.into()));
    }
    Ok(root.join(child))
}

fn paths_match(left: &Path, right: &Path) -> bool {
    dunce::canonicalize(left).unwrap_or_else(|_| left.to_path_buf())
        == dunce::canonicalize(right).unwrap_or_else(|_| right.to_path_buf())
}

fn push_skipped(manifest: &mut MigrationManifest, resource: &str, detail: &str) {
    manifest.resources.push(ResourceResult {
        resource: resource.to_owned(),
        status: ResourceStatus::Skipped,
        detail: Some(detail.to_owned()),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(root: &tempfile::TempDir) -> MigrationOptions {
        MigrationOptions {
            source: root.path().join(".grok"),
            destination: root.path().join(".echo-build"),
            dry_run: false,
            skip: false,
            include_memory: false,
            conflict_policy: ConflictPolicy::KeepDestination,
        }
    }

    #[test]
    fn imports_non_secret_config_and_ignores_auth() {
        let root = tempfile::tempdir().unwrap();
        let options = options(&root);
        std::fs::create_dir_all(&options.source).unwrap();
        std::fs::write(
            options.source.join("config.toml"),
            "[ui]\nvim_mode = true\n",
        )
        .unwrap();
        std::fs::write(options.source.join("auth.json"), r#"{"token":"nope"}"#).unwrap();

        let report = migrate(&options).unwrap();

        assert!(options.destination.join("config.toml").exists());
        assert!(!options.destination.join("auth.json").exists());
        assert!(report.resources.iter().any(
            |result| result.resource == "auth.json" && result.status == ResourceStatus::Skipped
        ));
    }

    #[test]
    fn strips_nested_credentials_from_config() {
        let root = tempfile::tempdir().unwrap();
        let options = options(&root);
        std::fs::create_dir_all(&options.source).unwrap();
        std::fs::write(
            options.source.join("config.toml"),
            "[models.custom]\nname = \"safe\"\napi_key = \"secret\"\n[telemetry]\ntelemetry_api_key = \"also-secret\"\n",
        )
        .unwrap();

        migrate(&options).unwrap();
        let imported = std::fs::read_to_string(options.destination.join("config.toml")).unwrap();

        assert!(imported.contains("name = \"safe\""));
        assert!(!imported.contains("secret"));
        assert!(!imported.contains("api_key"));
    }

    #[test]
    fn dry_run_does_not_create_destination() {
        let root = tempfile::tempdir().unwrap();
        let mut options = options(&root);
        options.dry_run = true;
        std::fs::create_dir_all(&options.source).unwrap();
        std::fs::write(options.source.join("config.toml"), "[ui]\n").unwrap();

        let report = migrate(&options).unwrap();

        assert!(!options.destination.exists());
        assert!(
            report
                .resources
                .iter()
                .any(|result| result.status == ResourceStatus::WouldCopy)
        );
    }

    #[test]
    fn rerun_keeps_existing_destination() {
        let root = tempfile::tempdir().unwrap();
        let options = options(&root);
        std::fs::create_dir_all(&options.source).unwrap();
        std::fs::write(options.source.join("config.toml"), "value = \"source\"\n").unwrap();
        migrate(&options).unwrap();
        std::fs::write(options.destination.join("config.toml"), "destination").unwrap();

        let report = migrate(&options).unwrap();

        assert_eq!(
            std::fs::read_to_string(options.destination.join("config.toml")).unwrap(),
            "destination"
        );
        assert!(
            report
                .resources
                .iter()
                .any(|result| result.status == ResourceStatus::KeptDestination)
        );
    }

    #[test]
    fn use_source_replaces_a_conflicting_destination() {
        let root = tempfile::tempdir().unwrap();
        let mut options = options(&root);
        options.conflict_policy = ConflictPolicy::UseSource;
        std::fs::create_dir_all(&options.source).unwrap();
        std::fs::create_dir_all(&options.destination).unwrap();
        std::fs::write(options.source.join("config.toml"), "value = \"source\"\n").unwrap();
        std::fs::write(
            options.destination.join("config.toml"),
            "value = \"destination\"\n",
        )
        .unwrap();

        migrate(&options).unwrap();

        let imported = std::fs::read_to_string(options.destination.join("config.toml")).unwrap();
        assert!(imported.contains("source"));
        assert!(!imported.contains("destination"));
    }

    #[test]
    fn retry_removes_stale_staging_state() {
        let root = tempfile::tempdir().unwrap();
        let options = options(&root);
        std::fs::create_dir_all(options.source.join("themes")).unwrap();
        std::fs::write(options.source.join("themes/new.toml"), "name = \"new\"\n").unwrap();
        std::fs::create_dir_all(&options.destination).unwrap();
        let stale = options.destination.join(".echo-import-themes.tmp");
        std::fs::create_dir_all(&stale).unwrap();
        std::fs::write(stale.join("partial.toml"), "partial").unwrap();

        migrate(&options).unwrap();

        assert!(!stale.exists());
        assert!(options.destination.join("themes/new.toml").exists());
        assert!(!options.destination.join("themes/partial.toml").exists());
    }

    #[test]
    fn hooks_and_plugins_are_quarantined() {
        let root = tempfile::tempdir().unwrap();
        let options = options(&root);
        std::fs::create_dir_all(options.source.join("hooks")).unwrap();
        std::fs::write(options.source.join("hooks/example.json"), "{}").unwrap();

        let report = migrate(&options).unwrap();
        let quarantined = options
            .destination
            .join("imports/grok/requires-trust/hooks/example.json");

        assert!(quarantined.exists());
        assert!(!options.destination.join("hooks/example.json").exists());
        assert!(report.resources.iter().any(
            |result| result.resource == "hooks" && result.status == ResourceStatus::Quarantined
        ));
    }

    #[cfg(unix)]
    #[test]
    fn imported_config_is_owner_only() {
        use std::os::unix::fs::PermissionsExt as _;
        let root = tempfile::tempdir().unwrap();
        let options = options(&root);
        std::fs::create_dir_all(&options.source).unwrap();
        std::fs::write(options.source.join("config.toml"), "[ui]\n").unwrap();

        migrate(&options).unwrap();

        let mode = std::fs::metadata(options.destination.join("config.toml"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }
}
