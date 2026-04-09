use hmanga_plugin_sdk::abi::{pack_ptr_len, unpack_ptr_len};
use hmanga_plugin_sdk::prelude::{Capabilities, PluginMetaInfo, SearchSort};

#[test]
fn prelude_reexports_core_plugin_types() {
    let meta = PluginMetaInfo {
        id: "demo".to_string(),
        name: "Demo".to_string(),
        version: "0.1.0".to_string(),
        sdk_version: 1,
        icon: Vec::new(),
        description: "demo plugin".to_string(),
        capabilities: Capabilities {
            search: true,
            login: false,
            favorites: false,
            ranking: false,
            weekly: false,
            tags_browsing: false,
        },
    };

    assert_eq!(meta.id, "demo");
    assert!(matches!(SearchSort::Latest, SearchSort::Latest));
}

#[test]
fn abi_helpers_pack_and_unpack_i64_results() {
    let packed = pack_ptr_len(0x1000, 0x0100);
    let (ptr, len) = unpack_ptr_len(packed);

    assert_eq!(ptr, 0x1000);
    assert_eq!(len, 0x0100);
}
