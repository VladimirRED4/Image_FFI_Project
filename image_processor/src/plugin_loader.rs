use libloading::{Library, Symbol};
use std::path::Path;
use crate::error::ProcessorError;
use log::{info, debug, error};

pub type ProcessImageFunc = unsafe extern "C" fn(
    width: u32,
    height: u32,
    rgba_data: *mut u8,
    params: *const std::ffi::c_char,
);

pub struct Plugin {
    _lib: Library,
    pub process_fn: ProcessImageFunc,
}

impl Plugin {
    pub fn load(plugin_name: &str, plugin_path: &Path) -> Result<Self, ProcessorError> {
        // Определяем имя библиотеки
        let lib_name = format!(
            "{}{}",
            if cfg!(target_os = "windows") { "" } else { "lib" },
            plugin_name
        ) + match std::env::consts::OS {
            "linux" => ".so",
            "macos" => ".dylib",
            "windows" => ".dll",
            _ => ".so",
        };

        let lib_path = plugin_path.join(lib_name);
        debug!("Loading plugin from: {}", lib_path.display());

        if !lib_path.exists() {
            error!("Plugin not found: {}", lib_path.display());
            return Err(ProcessorError::PluginNotFound(
                lib_path.to_string_lossy().to_string()
            ));
        }

        unsafe {
            // Загружаем библиотеку
            let lib = Library::new(&lib_path)
                .map_err(|e| ProcessorError::PluginLoadError(e.to_string()))?;

            // Получаем функцию
            let func: Symbol<ProcessImageFunc> = lib
                .get(b"process_image")
                .map_err(|e| ProcessorError::PluginLoadError(e.to_string()))?;

            // Копируем указатель на функцию
            let process_fn = *func;

            info!("Plugin loaded: {}", plugin_name);

            // Возвращаем структуру с библиотекой и функцией
            Ok(Plugin {
                _lib: lib,
                process_fn,
            })
        }
    }
}

pub fn init_logging(verbose: bool) {
    use env_logger::Builder;
    use log::LevelFilter;

    let level = if verbose {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    Builder::new()
        .filter(None, level)
        .format_timestamp_secs()
        .init();
}