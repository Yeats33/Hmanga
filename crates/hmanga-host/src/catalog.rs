use hmanga_core::{
    Capabilities, PluginHealth, PluginInfo, PluginKind, PluginMetaInfo, PluginRuntimeKind,
};

pub const SUPPORTED_SDK_VERSION: u32 = 1;
pub const SUPPORTED_SDK_VERSION_MIN: u32 = 1;
pub const SUPPORTED_SDK_VERSION_MAX: u32 = 1;

/// The official plugin catalog holds metadata for all bundled/installable
/// official plugins. It is separate from PluginRegistry which holds only
/// activated adapters.
#[derive(Debug, Clone)]
pub struct OfficialPluginCatalog {
    plugins: Vec<PluginInfo>,
}

impl OfficialPluginCatalog {
    pub fn new() -> Self {
        let jm_meta = PluginMetaInfo {
            id: "jm".to_string(),
            name: "J-Manga".to_string(),
            version: "0.1.0".to_string(),
            sdk_version: SUPPORTED_SDK_VERSION,
            icon: Vec::new(),
            description: "J-Manga plugin".to_string(),
            capabilities: Capabilities {
                search: true,
                login: false,
                favorites: false,
                ranking: false,
                weekly: false,
                tags_browsing: false,
            },
        };
        let jm_info = PluginInfo {
            meta: jm_meta,
            kind: PluginKind::OfficialBundled,
            runtime: PluginRuntimeKind::Native,
            installed: true,
            unlocked: false,
            enabled: false,
            health: PluginHealth::Healthy,
        };
        Self {
            plugins: vec![jm_info],
        }
    }

    pub fn all(&self) -> &[PluginInfo] {
        &self.plugins
    }

    pub fn find(&self, id: &str) -> Option<&PluginInfo> {
        self.plugins.iter().find(|p| p.meta.id == id)
    }
}

impl Default for OfficialPluginCatalog {
    fn default() -> Self {
        Self::new()
    }
}
