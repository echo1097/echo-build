//! bundled release notes for the tui.
//!
//! the markdown file is compiled into the binary so the welcome screen and
//! `/release-notes` work without a network request, cdn, or disk cache.

const BUNDLED_RELEASE_NOTES: &str = include_str!("../../assets/release-notes.md");

/// a single release-note entry used by the welcome screen.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ChangelogEntry {
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub breaking_change: bool,
}

/// bundled release notes in full markdown and welcome-screen entry formats.
pub struct Changelog {
    pub markdown: Option<String>,
    pub entries: Option<Vec<ChangelogEntry>>,
}

/// loads release notes compiled into the binary.
pub struct ChangelogManager;

impl Default for ChangelogManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ChangelogManager {
    pub fn new() -> Self {
        Self
    }

    pub fn fetch(&self) -> Changelog {
        Changelog {
            markdown: Some(BUNDLED_RELEASE_NOTES.to_string()),
            entries: Some(entries_from_markdown(BUNDLED_RELEASE_NOTES)),
        }
    }
}

fn entries_from_markdown(markdown: &str) -> Vec<ChangelogEntry> {
    markdown
        .lines()
        .filter_map(|line| line.trim().strip_prefix("- "))
        .filter(|description| !description.is_empty())
        .map(|description| ChangelogEntry {
            category: "release_notes".to_string(),
            description: description.to_string(),
            breaking_change: false,
        })
        .collect()
}

fn strip_markdown_inline(value: &str) -> String {
    value.replace("**", "").replace('`', "")
}

/// converts release-note entries to plain-text welcome bullets.
pub fn bullets_from_entries(entries: &[ChangelogEntry], max: usize) -> Vec<String> {
    entries
        .iter()
        .filter(|entry| !entry.description.is_empty())
        .take(max)
        .map(|entry| strip_markdown_inline(&entry.description))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_release_notes_are_available_offline() {
        let changelog = ChangelogManager::new().fetch();
        let markdown = changelog.markdown.expect("bundled markdown");
        let entries = changelog.entries.expect("bundled entries");

        assert!(markdown.contains("# Echo Build 0.2.106"));
        assert!(markdown.contains("OpenRouter"));
        assert!(!entries.is_empty());
        assert_eq!(entries[0].category, "release_notes");
    }

    #[test]
    fn markdown_list_items_become_entries() {
        let entries = entries_from_markdown("# Notes\n\n- First change\n- Second change\n");

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].description, "First change");
        assert_eq!(entries[1].description, "Second change");
    }

    #[test]
    fn bullets_strip_markdown_and_respect_max() {
        let entries = vec![
            ChangelogEntry {
                category: "features".into(),
                description: "Added **dark mode** support".into(),
                breaking_change: false,
            },
            ChangelogEntry {
                category: "fixes".into(),
                description: "Fixed `crash` on startup".into(),
                breaking_change: false,
            },
            ChangelogEntry {
                category: "performance".into(),
                description: "Faster rendering".into(),
                breaking_change: false,
            },
        ];

        let bullets = bullets_from_entries(&entries, 2);

        assert_eq!(
            bullets,
            vec!["Added dark mode support", "Fixed crash on startup"]
        );
    }

    #[test]
    fn tolerant_deserialization_keeps_partial_entries() {
        let json = r#"[{"category":"features"},{"description":"ok"}]"#;
        let entries: Vec<ChangelogEntry> = serde_json::from_str(json).unwrap();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].description, "");
        assert_eq!(entries[1].category, "");
        assert_eq!(entries[1].description, "ok");
    }
}
