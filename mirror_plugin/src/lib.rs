use serde::Deserialize;
use std::ffi::CStr;
use std::os::raw::c_char;

#[derive(Debug, Deserialize)]
#[serde(default)]
struct MirrorParams {
    horizontal: bool,
    vertical: bool,
}

impl Default for MirrorParams {
    fn default() -> Self {
        MirrorParams {
            horizontal: false,
            vertical: false,
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn process_image(
    width: u32,
    height: u32,
    rgba_data: *mut u8,
    params: *const c_char,
) {
    // Преобразуем параметры из C-строки
    let params_str = if params.is_null() {
        ""
    } else {
        CStr::from_ptr(params).to_str().unwrap_or("")
    };

    // Парсим JSON параметры
    let mirror_params: MirrorParams = serde_json::from_str(params_str).unwrap_or_default();

    let width = width as usize;
    let height = height as usize;
    let data = std::slice::from_raw_parts_mut(rgba_data, width * height * 4);

    // Применяем зеркальное отражение
    if mirror_params.horizontal {
        mirror_horizontal(data, width, height);
    }

    if mirror_params.vertical {
        mirror_vertical(data, width, height);
    }
}

fn mirror_horizontal(data: &mut [u8], width: usize, height: usize) {
    for y in 0..height {
        for x in 0..width / 2 {
            let left_idx = (y * width + x) * 4;
            let right_idx = (y * width + (width - 1 - x)) * 4;

            // Меняем местами пиксели
            for i in 0..4 {
                data.swap(left_idx + i, right_idx + i);
            }
        }
    }
}

fn mirror_vertical(data: &mut [u8], width: usize, height: usize) {
    for y in 0..height / 2 {
        for x in 0..width {
            let top_idx = (y * width + x) * 4;
            let bottom_idx = ((height - 1 - y) * width + x) * 4;

            // Меняем местами пиксели
            for i in 0..4 {
                data.swap(top_idx + i, bottom_idx + i);
            }
        }
    }
}