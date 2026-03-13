pub mod error;
pub mod plugin_loader;

use crate::error::ProcessorError;
use crate::plugin_loader::Plugin;
use image::{ImageFormat, RgbaImage};
use log::{debug, error, info};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

pub fn process_image(
    input_path: &Path,
    output_path: &Path,
    plugin_name: &str,
    params_path: &Path,
    plugin_path: &Path,
) -> Result<(), ProcessorError> {
    let total_start = Instant::now();

    info!("Process started");

    // Проверка файлов
    if !input_path.exists() {
        error!("Input file not found: {}", input_path.display());
        return Err(ProcessorError::InputFileNotFound(input_path.to_path_buf()));
    }
    if !params_path.exists() {
        error!("Params file not found: {}", params_path.display());
        return Err(ProcessorError::ParamsFileNotFound(
            params_path.to_path_buf(),
        ));
    }

    // Загрузка изображения
    let img = match image::open(input_path) {
        Ok(img) => img.into_rgba8(),
        Err(e) => {
            error!("Failed to load image: {}", e);
            return Err(ProcessorError::ImageLoadError(e.to_string()));
        }
    };

    let (width, height) = img.dimensions();
    info!("Image size: {}x{}", width, height);

    let mut rgba_data = img.into_raw();
    let data_size = rgba_data.len();
    debug!("Raw data size: {} bytes", data_size);

    // Чтение параметров
    let params = match fs::read_to_string(params_path) {
        Ok(p) => p,
        Err(e) => {
            error!("Failed to read params file: {}", e);
            return Err(ProcessorError::ParamsReadError(e.to_string()));
        }
    };

    // Загрузка плагина
    let plugin_load_start = Instant::now();
    let plugin = Plugin::load(plugin_name, plugin_path)?;
    let plugin_load_duration = plugin_load_start.elapsed();
    debug!("Plugin loaded in {:?}", plugin_load_duration);

    // Подготовка параметров
    let params_cstring = match std::ffi::CString::new(params) {
        Ok(cs) => cs,
        Err(e) => {
            error!("Failed to convert params to C string: {}", e);
            return Err(ProcessorError::PluginLoadError(e.to_string()));
        }
    };

    // Выполнение плагина с индикатором прогресса
    info!("Executing plugin: {}", plugin_name);

    // Создаем флаг для остановки индикатора
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    // Запускаем индикатор в отдельном потоке
    let spinner_handle = thread::spawn(move || {
        let dots = [".  ", ".. ", "...", " ..", "  .", "   "];
        let mut i = 0;
        while r.load(Ordering::Relaxed) {
            print!("\rProcessing{}", dots[i % dots.len()]);
            io::stdout().flush().unwrap();
            i += 1;
            thread::sleep(std::time::Duration::from_millis(200));
        }
        print!("\r            \r");
        io::stdout().flush().unwrap();
    });

    let process_start = Instant::now();

    unsafe {
        (plugin.process_fn)(
            width,
            height,
            rgba_data.as_mut_ptr(),
            params_cstring.as_ptr(),
        );
    }

    let process_duration = process_start.elapsed();

    // Останавливаем индикатор
    running.store(false, Ordering::Relaxed);
    let _ = spinner_handle.join();

    info!("Plugin execution time: {:?}", process_duration);

    // Сохранение результата
    let result_img = match RgbaImage::from_raw(width, height, rgba_data) {
        Some(img) => img,
        None => {
            error!("Failed to create image from processed data");
            return Err(ProcessorError::ImageSaveError(
                "Failed to create image from raw data".to_string(),
            ));
        }
    };

    match result_img.save_with_format(output_path, ImageFormat::Png) {
        Ok(_) => {
            let total_duration = total_start.elapsed();
            info!("Image saved to: {}", output_path.display());
            info!("Total processing time: {:?}", total_duration);
            Ok(())
        }
        Err(e) => {
            error!("Failed to save image: {}", e);
            Err(ProcessorError::ImageSaveError(e.to_string()))
        }
    }
}
