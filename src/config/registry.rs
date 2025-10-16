use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, RwLock},
};

use log::warn;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::watch;

use crate::error::{AppError, Result};

use super::loader::{load_region_descriptors, RegionDescriptor};

#[derive(Clone)]
struct RegistryState {
    descriptors: HashMap<String, RegionDescriptor>,
    ordered: Arc<Vec<RegionDescriptor>>,
}

impl RegistryState {
    fn from_descriptors(mut descriptors: Vec<RegionDescriptor>) -> Self {
        descriptors.sort_by(|a, b| a.code.cmp(&b.code));

        let mut map = HashMap::with_capacity(descriptors.len());
        for descriptor in &descriptors {
            map.insert(descriptor.code.to_lowercase(), descriptor.clone());
        }

        Self {
            descriptors: map,
            ordered: Arc::new(descriptors),
        }
    }
}

/// Central cache of market descriptors loaded from disk, with optional file watching.
pub struct ConfigRegistry {
    root: PathBuf,
    state: RwLock<RegistryState>,
    updates_tx: watch::Sender<Arc<Vec<RegionDescriptor>>>,
    watcher: Mutex<Option<RecommendedWatcher>>,
}

impl ConfigRegistry {
    /// Build the registry by scanning `assets/configs` under the provided root.
    pub fn new(root: impl Into<PathBuf>) -> Result<Self> {
        let root = root.into();
        let descriptors = load_region_descriptors(&root)?;
        if descriptors.is_empty() {
            return Err(AppError::message(
                "No region descriptors found under assets/configs",
            ));
        }

        let state = RegistryState::from_descriptors(descriptors);
        let view = state.ordered.clone();
        let (updates_tx, _) = watch::channel(view.clone());

        Ok(Self {
            root,
            state: RwLock::new(state),
            updates_tx,
            watcher: Mutex::new(None),
        })
    }

    /// Return a cloned snapshot of the available region descriptors.
    pub fn snapshot(&self) -> Arc<Vec<RegionDescriptor>> {
        self.state.read().unwrap().ordered.clone()
    }

    /// Subscribe to descriptor updates. The returned receiver immediately yields the latest snapshot.
    #[allow(dead_code)]
    pub fn subscribe(&self) -> watch::Receiver<Arc<Vec<RegionDescriptor>>> {
        self.updates_tx.subscribe()
    }

    /// Fetch a specific descriptor by code (case-insensitive).
    pub fn get(&self, code: &str) -> Option<RegionDescriptor> {
        let key = code.to_lowercase();
        self.state.read().unwrap().descriptors.get(&key).cloned()
    }

    /// Force a reload from disk and broadcast updates when data changes.
    pub fn refresh(&self) -> Result<()> {
        let descriptors = load_region_descriptors(self.root())?;
        if descriptors.is_empty() {
            return Err(AppError::message(
                "No region descriptors found under assets/configs",
            ));
        }

        let new_state = RegistryState::from_descriptors(descriptors);

        {
            let mut state = self.state.write().unwrap();
            *state = new_state.clone();
        }

        let _ = self.updates_tx.send(new_state.ordered.clone());
        Ok(())
    }

    /// Begin watching the configuration directory for changes. Multiple invocations are no-ops.
    pub fn start_watching(self: &Arc<Self>) -> Result<()> {
        if self.watcher.lock().unwrap().is_some() {
            return Ok(());
        }

        let configs_dir = self.configs_dir();
        if !configs_dir.exists() {
            std::fs::create_dir_all(&configs_dir).map_err(AppError::from)?;
        }

        let registry = Arc::clone(self);
        let mut watcher =
            notify::recommended_watcher(move |res: notify::Result<notify::Event>| match res {
                Ok(event) if is_relevant_event(&event.kind) => {
                    if let Err(err) = registry.refresh() {
                        warn!("Failed to refresh region descriptors: {err}");
                    }
                }
                Ok(_) => {}
                Err(err) => warn!("Region config watch error: {err}"),
            })
            .map_err(|err| AppError::message(format!("Failed to start watcher: {err}")))?;

        watcher
            .watch(&configs_dir, RecursiveMode::NonRecursive)
            .map_err(|err| {
                AppError::message(format!("Failed to watch configs directory: {err}"))
            })?;
        *self.watcher.lock().unwrap() = Some(watcher);
        Ok(())
    }

    fn root(&self) -> &Path {
        &self.root
    }

    fn configs_dir(&self) -> PathBuf {
        self.root().join("assets").join("configs")
    }
}

fn is_relevant_event(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Create(_)
            | EventKind::Modify(_)
            | EventKind::Remove(_)
            | EventKind::Any
            | EventKind::Other
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_descriptors() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let registry = ConfigRegistry::new(root).expect("registry loads");
        let snapshot = registry.snapshot();
        assert!(
            !snapshot.is_empty(),
            "expected at least one descriptor in snapshot"
        );
        let cn = registry.get("cn").expect("cn descriptor");
        assert_eq!(cn.code, "CN");
    }

    #[test]
    fn refresh_reloads_changes() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let registry = ConfigRegistry::new(root).expect("registry loads");
        registry.refresh().expect("refresh succeeds");
    }
}
