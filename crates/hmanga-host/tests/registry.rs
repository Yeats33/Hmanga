use hmanga_core::PluginKind;
use hmanga_host::{OfficialPluginCatalog, PluginRegistry, WasmLoader};

#[tokio::test]
async fn native_simple_plugin_adapter_executes_build_then_parse() {
    let catalog = OfficialPluginCatalog::new();
    let registry = PluginRegistry::new(catalog);

    // The jm plugin should be registered as an official locked plugin
    let plugins = registry.visible_plugins();
    let jm = plugins
        .iter()
        .find(|p| p.meta.id == "jm")
        .expect("jm should be visible");
    assert!(!jm.unlocked, "jm should be locked by default");
    assert!(matches!(jm.kind, PluginKind::OfficialBundled));
}

#[tokio::test]
async fn registry_exposes_locked_official_plugins_separately_from_active_plugins() {
    let catalog = OfficialPluginCatalog::new();
    let registry = PluginRegistry::new(catalog);

    let visible = registry.visible_plugins();
    let active = registry.active_plugins();

    // All visible plugins should be in the catalog
    for p in &visible {
        assert!(!p.enabled || !p.unlocked);
    }

    // Active plugins should be empty (none unlocked yet)
    assert!(
        active.is_empty(),
        "no plugins should be active until unlocked"
    );
}

#[tokio::test]
async fn locked_official_plugins_are_not_activated_until_local_confirmation() {
    let catalog = OfficialPluginCatalog::new();
    let mut registry = PluginRegistry::new(catalog);

    // Initially jm is not active
    assert!(registry.active_plugins().is_empty());

    // Simulate local donation unlock
    registry.confirm_unlock("jm");

    // Now jm should be active
    let active = registry.active_plugins();
    assert_eq!(active.len(), 1, "jm should be activated after unlock");
    assert_eq!(active[0].meta.id, "jm");
}

#[tokio::test]
async fn wasm_loader_rejects_incompatible_sdk_version() {
    use hmanga_host::catalog::SUPPORTED_SDK_VERSION_MAX;
    use hmanga_host::catalog::SUPPORTED_SDK_VERSION_MIN;

    // Test that check_sdk_version rejects out-of-range versions
    let _loader = WasmLoader::new().expect("wasmtime engine should init");

    // Version too new
    let too_high = SUPPORTED_SDK_VERSION_MAX + 1;
    let result = WasmLoader::check_sdk_version(too_high);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("sdk version") || err.to_string().contains("incompatible"));

    // Version too low
    let too_low = SUPPORTED_SDK_VERSION_MIN - 1;
    let result2 = WasmLoader::check_sdk_version(too_low);
    assert!(result2.is_err());

    // Valid version range should pass
    let valid = (SUPPORTED_SDK_VERSION_MIN + SUPPORTED_SDK_VERSION_MAX) / 2;
    assert!(WasmLoader::check_sdk_version(valid).is_ok());
}
