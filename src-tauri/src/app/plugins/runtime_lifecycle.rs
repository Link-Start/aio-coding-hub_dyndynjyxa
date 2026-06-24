//! Usage: Host-owned plugin runtime lifecycle and cache retention boundary.

use crate::plugins::PluginDetail;
use std::sync::{Arc, RwLock};

pub(crate) trait PluginRuntimeCache: Send + Sync {
    fn retain_for_plugins(&self, plugins: &[PluginDetail]);
    #[allow(dead_code)]
    fn clear_all(&self);
}

#[derive(Default)]
pub(crate) struct RuntimeLifecycleRegistry {
    caches: RwLock<Vec<Arc<dyn PluginRuntimeCache>>>,
}

impl RuntimeLifecycleRegistry {
    pub(crate) fn register_cache(&self, cache: Arc<dyn PluginRuntimeCache>) {
        self.caches
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(cache);
    }

    pub(crate) fn retain_for_plugins(&self, plugins: &[PluginDetail]) {
        for cache in self.caches_snapshot() {
            cache.retain_for_plugins(plugins);
        }
    }

    #[allow(dead_code)]
    pub(crate) fn dispose_all(&self) {
        for cache in self.caches_snapshot() {
            cache.clear_all();
        }
    }

    fn caches_snapshot(&self) -> Vec<Arc<dyn PluginRuntimeCache>> {
        self.caches
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct RecordingCache {
        retain_calls: Mutex<Vec<Vec<String>>>,
        clear_calls: Mutex<u32>,
    }

    impl RecordingCache {
        fn retain_calls(&self) -> Vec<Vec<String>> {
            self.retain_calls.lock().unwrap().clone()
        }

        fn clear_calls(&self) -> u32 {
            *self.clear_calls.lock().unwrap()
        }
    }

    impl PluginRuntimeCache for RecordingCache {
        fn retain_for_plugins(&self, plugins: &[PluginDetail]) {
            self.retain_calls.lock().unwrap().push(
                plugins
                    .iter()
                    .map(|plugin| plugin.summary.plugin_id.clone())
                    .collect(),
            );
        }

        fn clear_all(&self) {
            *self.clear_calls.lock().unwrap() += 1;
        }
    }

    #[test]
    fn lifecycle_registry_retains_and_disposes_all_registered_runtime_caches() {
        let registry = RuntimeLifecycleRegistry::default();
        let first = std::sync::Arc::new(RecordingCache::default());
        let second = std::sync::Arc::new(RecordingCache::default());

        registry.register_cache(first.clone());
        registry.register_cache(second.clone());
        registry.retain_for_plugins(&[]);
        registry.dispose_all();

        assert_eq!(first.retain_calls(), vec![Vec::<String>::new()]);
        assert_eq!(second.retain_calls(), vec![Vec::<String>::new()]);
        assert_eq!(first.clear_calls(), 1);
        assert_eq!(second.clear_calls(), 1);
    }
}
