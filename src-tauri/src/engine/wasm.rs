#![allow(dead_code)]
use std::sync::{Arc, Mutex};
use wasmer::{imports, Function, FunctionEnv, FunctionEnvMut, Instance, Module, Store};

/// WASM Plugin Interface for VIBE DAW
///
/// Memory Layout (Linear Memory):
/// - [0..buffer_size*8] : Input buffer (f64 samples, interleaved L/R)
/// - [buffer_size*8..buffer_size*16] : Output buffer (f64 samples, interleaved L/R)
/// - [buffer_size*16..] : Plugin state/scratch space
pub struct WasmPluginEnv {
    pub sample_rate: f64,
    pub buffer_size: usize,
}

pub struct WasmPlugin {
    instance: Instance,
    store: Store,
    env: Arc<Mutex<WasmPluginEnv>>,
}

impl WasmPlugin {
    pub fn new(wasm_bytes: &[u8], sample_rate: f64, buffer_size: usize) -> Result<Self, String> {
        let mut store = Store::default();
        let module = Module::new(&store, wasm_bytes).map_err(|e| e.to_string())?;

        let env = Arc::new(Mutex::new(WasmPluginEnv {
            sample_rate,
            buffer_size,
        }));

        let env_clone = Arc::clone(&env);
        let function_env = FunctionEnv::new(&mut store, env_clone);

        // Host functions exposed to WASM
        let get_sample_rate = Function::new_typed_with_env(
            &mut store,
            &function_env,
            |env: FunctionEnvMut<Arc<Mutex<WasmPluginEnv>>>| -> f64 {
                env.data().lock().unwrap().sample_rate
            },
        );

        let import_object = imports! {
            "env" => {
                "get_sample_rate" => get_sample_rate,
            }
        };

        // Note: In a real implementation, we would inspect the exports here
        // to see if "get_parameter_info" exists.

        let instance =
            Instance::new(&mut store, &module, &import_object).map_err(|e| e.to_string())?;

        Ok(WasmPlugin {
            instance,
            store,
            env,
        })
    }

    /// Process audio through the WASM plugin
    /// input/output are planar f64 buffers (left, right)
    pub fn process(
        &mut self,
        input_l: &[f64],
        input_r: &[f64],
        output_l: &mut [f64],
        output_r: &mut [f64],
    ) -> Result<(), String> {
        let memory = self
            .instance
            .exports
            .get_memory("memory")
            .map_err(|e| e.to_string())?;

        let buffer_size = self.env.lock().unwrap().buffer_size;

        // Write input to WASM memory (interleaved)
        {
            let view = memory.view(&self.store);
            for i in 0..buffer_size {
                let offset = i * 2;
                view.write((offset * 8) as u64, &input_l[i].to_le_bytes())
                    .map_err(|e| e.to_string())?;
                view.write(((offset + 1) * 8) as u64, &input_r[i].to_le_bytes())
                    .map_err(|e| e.to_string())?;
            }
        }

        // Call WASM process function
        let process_fn = self
            .instance
            .exports
            .get_function("process")
            .map_err(|e| e.to_string())?;

        process_fn
            .call(&mut self.store, &[])
            .map_err(|e| e.to_string())?;

        // Read output from WASM memory (interleaved)
        {
            let view = memory.view(&self.store);
            let output_offset = buffer_size * 2 * 8; // After input buffer
            for i in 0..buffer_size {
                let offset = output_offset + (i * 2 * 8);

                let mut l_bytes = [0u8; 8];
                let mut r_bytes = [0u8; 8];

                view.read(offset as u64, &mut l_bytes)
                    .map_err(|e| e.to_string())?;
                view.read((offset + 8) as u64, &mut r_bytes)
                    .map_err(|e| e.to_string())?;

                output_l[i] = f64::from_le_bytes(l_bytes);
                output_r[i] = f64::from_le_bytes(r_bytes);
            }
        }

        Ok(())
    }

    /// Get parameter value from plugin
    pub fn get_parameter(&mut self, index: u32) -> Result<f64, String> {
        let get_param = self
            .instance
            .exports
            .get_function("get_parameter")
            .map_err(|e| e.to_string())?;

        let result = get_param
            .call(&mut self.store, &[index.into()])
            .map_err(|e| e.to_string())?;

        result[0].f64().ok_or("Invalid return type".to_string())
    }

    /// Set parameter value in plugin
    pub fn set_parameter(&mut self, index: u32, value: f64) -> Result<(), String> {
        let set_param = self
            .instance
            .exports
            .get_function("set_parameter")
            .map_err(|e| e.to_string())?;

        set_param
            .call(&mut self.store, &[index.into(), value.into()])
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_wasm_plugin_creation() {
        // Example WASM module (would be loaded from file in production)
        // This is a minimal WAT (WebAssembly Text) that just copies input to output

        let wat = r#"
            (module
                (memory (export "memory") 1)
                (global $param (mut f64) (f64.const 0.5))
                
                (func (export "process")
                    ;; Minimal passthrough: in real life we'd loop
                    ;; For test, we assume host handles memory movement validation
                )

                (func (export "get_parameter") (param i32) (result f64)
                    global.get $param
                )

                (func (export "set_parameter") (param i32) (param f64)
                    local.get 1
                    global.set $param
                )
            )
        "#;

        let wasm_bytes = wat::parse_str(wat).unwrap();
        let plugin = WasmPlugin::new(&wasm_bytes, 48000.0, 512);
        assert!(plugin.is_ok());
    }

    #[test]
    fn test_wasm_parameters() {
        let wat = r#"
            (module
                (memory (export "memory") 1)
                (global $p (mut f64) (f64.const 0.0))
                
                (func (export "process"))

                (func (export "get_parameter") (param i32) (result f64)
                    global.get $p
                )

                (func (export "set_parameter") (param i32) (param f64)
                    local.get 1
                    global.set $p
                )
            )
        "#;
        let wasm_bytes = wat::parse_str(wat).unwrap();
        let mut plugin =
            WasmPlugin::new(&wasm_bytes, 44100.0, 128).expect("Failed to create plugin");

        // Initial default
        assert_eq!(plugin.get_parameter(0).unwrap(), 0.0);

        // Set and Get
        plugin.set_parameter(0, 0.75).expect("Failed to set");
        assert_eq!(plugin.get_parameter(0).unwrap(), 0.75);
    }

    #[test]
    fn test_wasm_processing_passthrough() {
        // This WAT manually copies the first sample from Input to Output
        // Memory Layout:
        // Input: [0..size*16] (Interleaved L/R f64)
        // Output: [size*16..size*32]

        // We will process a buffer of size 1 for simplicity of WAT code
        // Load Input L (byte 0), Store Output L (byte 16)
        // Load Input R (byte 8), Store Output R (byte 24)
        let wat = r#"
            (module
                (memory (export "memory") 1)
                
                (func (export "get_parameter") (param i32) (result f64) (f64.const 0.0))
                (func (export "set_parameter") (param i32) (param f64))

                (func (export "process")
                    ;; Copy L (addr 0) to Output L (addr 16)
                    (f64.store (i32.const 16) (f64.load (i32.const 0)))
                    
                    ;; Copy R (addr 8) to Output R (addr 24)
                    (f64.store (i32.const 24) (f64.load (i32.const 8)))
                )
            )
        "#;

        let wasm_bytes = wat::parse_str(wat).unwrap();
        let mut plugin = WasmPlugin::new(&wasm_bytes, 44100.0, 1).expect("Failed");

        let input_l = vec![0.5];
        let input_r = vec![-0.5];
        let mut output_l = vec![0.0];
        let mut output_r = vec![0.0];

        plugin
            .process(&input_l, &input_r, &mut output_l, &mut output_r)
            .expect("Process failed");

        assert_eq!(output_l[0], 0.5);
        assert_eq!(output_r[0], -0.5);
    }
}
