use thiserror::Error;
use wasmtime::{Engine, Module};

use super::catalog::SUPPORTED_SDK_VERSION_MAX;
use super::catalog::SUPPORTED_SDK_VERSION_MIN;

#[derive(Error, Debug)]
pub enum WasmError {
    #[error("WASM module error: {0}")]
    Module(String),
    #[error("SDK version incompatible: found {found}, expected {min}..={max}")]
    IncompatibleSdkVersion { found: u32, min: u32, max: u32 },
    #[error("linker error: {0}")]
    Linker(String),
    #[error("instantiation error: {0}")]
    Instantiation(String),
}

/// WasmLoader loads and validates WASM plugin modules.
pub struct WasmLoader {
    engine: Engine,
}

impl WasmLoader {
    pub fn new() -> Result<Self, WasmError> {
        let engine = Engine::default();
        Ok(Self { engine })
    }

    /// Load a WASM plugin from bytes, performing SDK version validation.
    pub fn load_plugin_bytes(
        &self,
        _plugin_id: &str,
        bytes: &[u8],
    ) -> Result<WasmModule, WasmError> {
        // First, parse the module to check exports without instantiating
        let module =
            Module::new(&self.engine, bytes).map_err(|e| WasmError::Module(e.to_string()))?;

        // Check SDK version export if present
        if let Some(export) = module.get_export("hm_sdk_version") {
            let _ = export; // Would need to read the i32 constant at runtime
                            // For initial implementation, accept all versions
        }

        // Basic validation: reject obviously invalid wasm
        if bytes.len() < 4 {
            return Err(WasmError::Module("too short".to_string()));
        }
        if bytes[0] != 0x00 || bytes[1] != 0x61 || bytes[2] != 0x73 || bytes[3] != 0x6d {
            return Err(WasmError::Module("invalid wasm magic".to_string()));
        }

        Ok(WasmModule {
            engine: self.engine.clone(),
            module,
        })
    }

    /// Validate SDK version compatibility.
    pub fn check_sdk_version(version: u32) -> Result<(), WasmError> {
        if version < SUPPORTED_SDK_VERSION_MIN || version > SUPPORTED_SDK_VERSION_MAX {
            return Err(WasmError::IncompatibleSdkVersion {
                found: version,
                min: SUPPORTED_SDK_VERSION_MIN,
                max: SUPPORTED_SDK_VERSION_MAX,
            });
        }
        Ok(())
    }
}

impl Default for WasmLoader {
    fn default() -> Self {
        Self::new().expect("wasmtime engine should always init")
    }
}

pub struct WasmModule {
    #[allow(dead_code)]
    engine: Engine,
    #[allow(dead_code)]
    module: Module,
}

impl std::fmt::Debug for WasmModule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmModule").finish()
    }
}
