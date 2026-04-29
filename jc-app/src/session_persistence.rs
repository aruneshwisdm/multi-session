use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedSession {
    pub project_path: PathBuf,
    pub label: String,
    #[serde(default)]
    pub uuid: Option<String>,
    #[serde(default)]
    pub busy: bool,
    #[serde(default)]
    pub has_ever_been_busy: bool,
    pub saved_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SessionStore {
    #[serde(default)]
    pub sessions: Vec<PersistedSession>,
}

impl SessionStore {
    pub fn for_project(&self, project_path: &Path) -> Vec<&PersistedSession> {
        self.sessions
            .iter()
            .filter(|s| s.project_path == project_path)
            .collect()
    }
}

fn store_path() -> PathBuf {
    dirs::home_dir()
        .expect("could not determine home directory")
        .join(".config/jc/sessions.json")
}

pub fn load() -> Result<SessionStore> {
    let path = store_path();
    match std::fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents)
            .with_context(|| format!("failed to parse {}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(SessionStore::default()),
        Err(e) => Err(anyhow::anyhow!("failed to read {}: {e}", path.display())),
    }
}

pub fn save(store: &SessionStore) -> Result<()> {
    let path = store_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let contents =
        serde_json::to_string_pretty(store).context("failed to serialize session store")?;
    std::fs::write(&path, contents)
        .with_context(|| format!("failed to write {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_session() -> PersistedSession {
        PersistedSession {
            project_path: PathBuf::from("/tmp/test-project"),
            label: "Session 1".into(),
            uuid: Some("abc-123".into()),
            busy: false,
            has_ever_been_busy: true,
            saved_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn roundtrip_serialize_deserialize() {
        let mut store = SessionStore::default();
        store.sessions.push(sample_session());

        let json = serde_json::to_string_pretty(&store).unwrap();
        let parsed: SessionStore = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.sessions.len(), 1);
        assert_eq!(parsed.sessions[0].label, "Session 1");
        assert_eq!(parsed.sessions[0].uuid, Some("abc-123".into()));
        assert!(parsed.sessions[0].has_ever_been_busy);
    }

    #[test]
    fn default_store_is_empty() {
        let store = SessionStore::default();
        assert!(store.sessions.is_empty());
    }

    #[test]
    fn for_project_filters_correctly() {
        let mut store = SessionStore::default();
        let now = chrono::Utc::now();
        store.sessions.push(PersistedSession {
            project_path: PathBuf::from("/tmp/project-a"),
            label: "A-1".into(),
            uuid: None,
            busy: false,
            has_ever_been_busy: false,
            saved_at: now,
        });
        store.sessions.push(PersistedSession {
            project_path: PathBuf::from("/tmp/project-b"),
            label: "B-1".into(),
            uuid: None,
            busy: false,
            has_ever_been_busy: false,
            saved_at: now,
        });
        store.sessions.push(PersistedSession {
            project_path: PathBuf::from("/tmp/project-a"),
            label: "A-2".into(),
            uuid: None,
            busy: false,
            has_ever_been_busy: false,
            saved_at: now,
        });

        let a_sessions = store.for_project(Path::new("/tmp/project-a"));
        assert_eq!(a_sessions.len(), 2);
        assert_eq!(a_sessions[0].label, "A-1");
        assert_eq!(a_sessions[1].label, "A-2");

        let b_sessions = store.for_project(Path::new("/tmp/project-b"));
        assert_eq!(b_sessions.len(), 1);

        let c_sessions = store.for_project(Path::new("/tmp/project-c"));
        assert!(c_sessions.is_empty());
    }

    #[test]
    fn forward_compatible_with_unknown_fields() {
        let json = r#"{
            "sessions": [{
                "project_path": "/tmp/test",
                "label": "S1",
                "uuid": null,
                "busy": false,
                "has_ever_been_busy": false,
                "saved_at": "2026-04-29T12:00:00Z",
                "future_field": "hello",
                "another_unknown": 42
            }]
        }"#;
        let store: SessionStore = serde_json::from_str(json).unwrap();
        assert_eq!(store.sessions.len(), 1);
        assert_eq!(store.sessions[0].label, "S1");
    }

    #[test]
    fn missing_optional_fields_use_defaults() {
        let json = r#"{
            "sessions": [{
                "project_path": "/tmp/test",
                "label": "S1",
                "saved_at": "2026-04-29T12:00:00Z"
            }]
        }"#;
        let store: SessionStore = serde_json::from_str(json).unwrap();
        assert_eq!(store.sessions[0].uuid, None);
        assert!(!store.sessions[0].busy);
        assert!(!store.sessions[0].has_ever_been_busy);
    }

    #[test]
    fn multiple_sessions_roundtrip() {
        let mut store = SessionStore::default();
        let now = chrono::Utc::now();
        for i in 0..5 {
            store.sessions.push(PersistedSession {
                project_path: PathBuf::from("/tmp/project"),
                label: format!("Session {i}"),
                uuid: Some(format!("uuid-{i}")),
                busy: i % 2 == 0,
                has_ever_been_busy: true,
                saved_at: now,
            });
        }

        let json = serde_json::to_string(&store).unwrap();
        let parsed: SessionStore = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.sessions.len(), 5);
        assert_eq!(parsed.for_project(Path::new("/tmp/project")).len(), 5);
    }

    #[test]
    fn empty_json_deserializes_to_default() {
        let store: SessionStore = serde_json::from_str("{}").unwrap();
        assert!(store.sessions.is_empty());
    }

    #[test]
    fn session_with_empty_uuid_roundtrips() {
        let mut store = SessionStore::default();
        store.sessions.push(PersistedSession {
            project_path: PathBuf::from("/tmp/test"),
            label: "No UUID".into(),
            uuid: None,
            busy: false,
            has_ever_been_busy: false,
            saved_at: chrono::Utc::now(),
        });

        let json = serde_json::to_string(&store).unwrap();
        let parsed: SessionStore = serde_json::from_str(&json).unwrap();
        assert!(parsed.sessions[0].uuid.is_none());
    }
}
