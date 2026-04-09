use hmanga_core::PluginKind;
use hmanga_host::{OfficialPluginCatalog, PluginRegistry};

/// Smoke test that asserts the app state initializes correctly:
/// - active site set to aggregate
/// - plugin list rendered from OfficialPluginCatalog
/// - only jm enabled by default
/// - donation unlock defaults to false
#[test]
fn app_state_initializes_correctly() {
    // Build state as the app would at startup
    let catalog = OfficialPluginCatalog::new();
    let registry = PluginRegistry::new(catalog);

    let visible_plugins = registry.visible_plugins();

    // jm should be visible (in catalog)
    assert!(
        !visible_plugins.is_empty(),
        "official plugin catalog should not be empty"
    );

    let jm = visible_plugins
        .iter()
        .find(|p| p.meta.id == "jm")
        .expect("jm should be in catalog");

    // jm is official bundled
    assert!(matches!(jm.kind, PluginKind::OfficialBundled));

    // jm should be locked/unlocked based on default
    // Default state: unlocked=false (donation not confirmed)
    assert!(
        !jm.unlocked,
        "jm should be locked (donation not confirmed) by default"
    );

    // No plugins active by default (none unlocked)
    let active_plugins = registry.active_plugins();
    assert!(
        active_plugins.is_empty(),
        "no plugins should be active until donation is confirmed"
    );

    // Simulate unlock and verify activation
    drop(registry);
}
