use std::collections::HashMap;
use std::sync::Arc;

use hmanga_core::PluginInfo;

use super::catalog::OfficialPluginCatalog;

/// PluginRegistry holds activated plugin adapters. Official plugins are
/// visible but locked until the user confirms donation unlock.
pub struct PluginRegistry {
    catalog: OfficialPluginCatalog,
    active: HashMap<String, Arc<dyn PluginAdapter>>,
}

impl std::fmt::Debug for PluginRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginRegistry")
            .field("active_ids", &self.active.keys().collect::<Vec<_>>())
            .finish()
    }
}

pub trait PluginAdapter: Send + Sync {
    fn id(&self) -> &str;
    fn info(&self) -> &PluginInfo;
}

impl PluginRegistry {
    pub fn new(catalog: OfficialPluginCatalog) -> Self {
        Self {
            catalog,
            active: HashMap::new(),
        }
    }

    /// All plugins known to the system (official catalog).
    pub fn visible_plugins(&self) -> Vec<PluginInfo> {
        self.catalog.all().to_vec()
    }

    /// Currently activated plugins.
    pub fn active_plugins(&self) -> Vec<PluginInfo> {
        self.active
            .values()
            .map(|adapter| adapter.info().clone())
            .collect()
    }

    /// Confirm donation unlock for a plugin, activating it.
    pub fn confirm_unlock(&mut self, id: &str) {
        if let Some(info) = self.catalog.find(id) {
            if info.kind == hmanga_core::PluginKind::OfficialBundled
                || info.kind == hmanga_core::PluginKind::OfficialInstallable
            {
                let mut info = info.clone();
                info.unlocked = true;
                info.enabled = true;
                info.health = hmanga_core::PluginHealth::Healthy;
                self.active
                    .insert(id.to_string(), Arc::new(NativeSimplePluginAdapter { info }));
            }
        }
    }
}

/// Minimal native adapter for SimplePlugin.
struct NativeSimplePluginAdapter {
    info: PluginInfo,
}

impl PluginAdapter for NativeSimplePluginAdapter {
    fn id(&self) -> &str {
        &self.info.meta.id
    }
    fn info(&self) -> &PluginInfo {
        &self.info
    }
}
