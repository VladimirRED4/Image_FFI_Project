use serde::Deserialize;
use std::ffi::CStr;
use std::os::raw::c_char;

#[derive(Debug, Deserialize)]
#[serde(default)]
#[derive(Default)]
struct MirrorParams {
    horizontal: bool,
    vertical: bool,
}

/// Применяет зеркальное отражение к изображению.
///
/// # Safety
///
/// Эта функция является unsafe, потому что:
/// * `rgba_data` должен указывать на валидную память размером `width * height * 4` байт
/// * `width` и `height` должны соответствовать реальному размеру изображения
/// * `params` должен быть валидным указателем на C-строку (или null)
/// * Вызывающий должен гарантировать, что память не освободится во время выполнения функции
#[no_mangle]
pub unsafe extern "C" fn process_image(
    width: u32,
    height: u32,
    rgba_data: *mut u8,
    params: *const c_char,
) {
    // Проверяем параметры на корректность
    if width == 0 || height == 0 {
        eprintln!("Warning: zero width or height");
        return;
    }

    // Проверяем, что указатель не нулевой
    if rgba_data.is_null() {
        eprintln!("Error: rgba_data is null");
        return;
    }

    // Проверяем переполнение при вычислении размера буфера
    let w = match usize::try_from(width) {
        Ok(w) => w,
        Err(_) => {
            eprintln!("Error: width too large for target platform");
            return;
        }
    };

    let h = match usize::try_from(height) {
        Ok(h) => h,
        Err(_) => {
            eprintln!("Error: height too large for target platform");
            return;
        }
    };

    // Используем checked_mul для предотвращения переполнения
    let total_pixels = match w.checked_mul(h) {
        Some(p) => p,
        None => {
            eprintln!("Error: width * height overflow");
            return;
        }
    };

    let buffer_size = match total_pixels.checked_mul(4) {
        Some(size) => size,
        None => {
            eprintln!("Error: buffer size overflow");
            return;
        }
    };

    // Дополнительная проверка: буфер не должен быть слишком большим
    // (например, больше 1 ГБ для тестов)
    if buffer_size > 1024 * 1024 * 1024 {
        eprintln!("Warning: buffer size too large ({} bytes)", buffer_size);
        return;
    }

    let params_str = if params.is_null() {
        ""
    } else {
        // Safety: вызывающий гарантирует, что params - валидная C-строка
        match CStr::from_ptr(params).to_str() {
            Ok(s) => s,
            Err(_) => {
                eprintln!("Error: invalid UTF-8 in params");
                return;
            }
        }
    };

    let mirror_params: MirrorParams = match serde_json::from_str(params_str) {
        Ok(p) => p,
        Err(_) => {
            eprintln!("Warning: failed to parse params, using defaults");
            MirrorParams::default()
        }
    };

    // Safety: вызывающий гарантирует, что rgba_data указывает на память нужного размера
    // и что размер соответствует вычисленному buffer_size
    let data = std::slice::from_raw_parts_mut(rgba_data, buffer_size);

    if mirror_params.horizontal {
        mirror_horizontal(data, w, h);
    }

    if mirror_params.vertical {
        mirror_vertical(data, w, h);
    }
}

fn mirror_horizontal(data: &mut [u8], width: usize, height: usize) {
    for y in 0..height {
        for x in 0..width / 2 {
            // Проверяем индексы на переполнение
            let left_idx = match (y * width + x).checked_mul(4) {
                Some(idx) => idx,
                None => continue,
            };

            let right_idx = match (y * width + (width - 1 - x)).checked_mul(4) {
                Some(idx) => idx,
                None => continue,
            };

            // Проверяем, что индексы в пределах буфера
            if left_idx + 3 >= data.len() || right_idx + 3 >= data.len() {
                continue;
            }

            for i in 0..4 {
                data.swap(left_idx + i, right_idx + i);
            }
        }
    }
}

fn mirror_vertical(data: &mut [u8], width: usize, height: usize) {
    for y in 0..height / 2 {
        for x in 0..width {
            let top_idx = match (y * width + x).checked_mul(4) {
                Some(idx) => idx,
                None => continue,
            };

            let bottom_idx = match ((height - 1 - y) * width + x).checked_mul(4) {
                Some(idx) => idx,
                None => continue,
            };

            if top_idx + 3 >= data.len() || bottom_idx + 3 >= data.len() {
                continue;
            }

            for i in 0..4 {
                data.swap(top_idx + i, bottom_idx + i);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_mirror_horizontal() {
        let width: u32 = 5;
        let height: u32 = 3;
        let buffer_size = (width * height * 4) as usize;
        let mut data = vec![0u8; buffer_size];

        for y in 0..height as usize {
            for x in 0..width as usize {
                let idx = (y * width as usize + x) * 4;
                data[idx] = x as u8;
                data[idx + 1] = x as u8;
                data[idx + 2] = x as u8;
                data[idx + 3] = 255;
            }
        }

        let params = CString::new("{\"horizontal\":true,\"vertical\":false}").unwrap();

        unsafe {
            process_image(width, height, data.as_mut_ptr(), params.as_ptr());
        }

        for y in 0..height as usize {
            for x in 0..width as usize {
                let idx = (y * width as usize + x) * 4;
                let expected_x = (width as usize - 1 - x) as u8;
                assert_eq!(data[idx], expected_x);
            }
        }
    }

    #[test]
    fn test_mirror_vertical() {
        let width: u32 = 3;
        let height: u32 = 5;
        let buffer_size = (width * height * 4) as usize;
        let mut data = vec![0u8; buffer_size];

        for y in 0..height as usize {
            for x in 0..width as usize {
                let idx = (y * width as usize + x) * 4;
                data[idx] = y as u8;
                data[idx + 1] = y as u8;
                data[idx + 2] = y as u8;
                data[idx + 3] = 255;
            }
        }

        let params = CString::new("{\"horizontal\":false,\"vertical\":true}").unwrap();

        unsafe {
            process_image(width, height, data.as_mut_ptr(), params.as_ptr());
        }

        for y in 0..height as usize {
            for x in 0..width as usize {
                let idx = (y * width as usize + x) * 4;
                let expected_y = (height as usize - 1 - y) as u8;
                assert_eq!(data[idx], expected_y);
            }
        }
    }

    #[test]
    fn test_mirror_both() {
        let width: u32 = 3;
        let height: u32 = 3;
        let buffer_size = (width * height * 4) as usize;
        let mut data = vec![0u8; buffer_size];
        let mut expected = vec![0u8; buffer_size];

        for y in 0..height as usize {
            for x in 0..width as usize {
                let idx = (y * width as usize + x) * 4;
                let value = (y * width as usize + x) as u8;
                data[idx] = value;
                data[idx + 1] = value;
                data[idx + 2] = value;
                data[idx + 3] = 255;

                let new_x = width as usize - 1 - x;
                let new_y = height as usize - 1 - y;
                let expected_idx = (new_y * width as usize + new_x) * 4;
                expected[expected_idx] = value;
                expected[expected_idx + 1] = value;
                expected[expected_idx + 2] = value;
                expected[expected_idx + 3] = 255;
            }
        }

        let params = CString::new("{\"horizontal\":true,\"vertical\":true}").unwrap();

        unsafe {
            process_image(width, height, data.as_mut_ptr(), params.as_ptr());
        }

        for i in 0..data.len() {
            assert_eq!(data[i], expected[i]);
        }
    }

    #[test]
    fn test_mirror_twice() {
        let width: u32 = 4;
        let height: u32 = 4;
        let buffer_size = (width * height * 4) as usize;
        let mut data = vec![0u8; buffer_size];

        for y in 0..height as usize {
            for x in 0..width as usize {
                let idx = (y * width as usize + x) * 4;
                let value = (y * width as usize + x) as u8;
                data[idx] = value;
                data[idx + 1] = value;
                data[idx + 2] = value;
                data[idx + 3] = 255;
            }
        }

        let original = data.clone();
        let params = CString::new("{\"horizontal\":true,\"vertical\":true}").unwrap();

        unsafe {
            process_image(width, height, data.as_mut_ptr(), params.as_ptr());
            process_image(width, height, data.as_mut_ptr(), params.as_ptr());
        }

        for i in 0..data.len() {
            assert_eq!(data[i], original[i]);
        }
    }

    #[test]
    fn test_zero_dimensions() {
        let mut data = vec![0u8; 4];
        let params = CString::new("{\"horizontal\":true}").unwrap();

        unsafe {
            process_image(0, 10, data.as_mut_ptr(), params.as_ptr());
            process_image(10, 0, data.as_mut_ptr(), params.as_ptr());
            process_image(0, 0, data.as_mut_ptr(), params.as_ptr());
        }
    }

    #[test]
    fn test_null_data_pointer() {
        let params = CString::new("{\"horizontal\":true}").unwrap();

        unsafe {
            process_image(10, 10, std::ptr::null_mut(), params.as_ptr());
        }
    }

    #[test]
    fn test_small_buffer() {
        let width: u32 = 1;
        let height: u32 = 1;
        let buffer_size = (width * height * 4) as usize;
        let mut data = vec![100u8; buffer_size];
        let params = CString::new("{\"horizontal\":true}").unwrap();

        unsafe {
            process_image(width, height, data.as_mut_ptr(), params.as_ptr());
            assert_eq!(data[0], 100);
        }
    }

    #[test]
    fn test_extreme_dimensions() {
        // Создаем минимальный корректный буфер
        let mut data = vec![0u8; 4];
        let params = CString::new("{\"horizontal\":true}").unwrap();

        unsafe {
            // Эти вызовы должны вернуться на этапе проверки переполнения
            // и не должны пытаться создать слайс
            process_image(u32::MAX, 1, data.as_mut_ptr(), params.as_ptr());
            process_image(1, u32::MAX, data.as_mut_ptr(), params.as_ptr());
            process_image(100_000, 100_000, data.as_mut_ptr(), params.as_ptr());
        }
    }

    #[test]
    fn test_null_params() {
        let width: u32 = 10;
        let height: u32 = 10;
        let buffer_size = (width * height * 4) as usize;
        let mut data = vec![0u8; buffer_size];

        unsafe {
            process_image(width, height, data.as_mut_ptr(), std::ptr::null());
        }
    }

    #[test]
    fn test_invalid_utf8_params() {
        let width: u32 = 10;
        let height: u32 = 10;
        let buffer_size = (width * height * 4) as usize;
        let mut data = vec![0u8; buffer_size];

        let invalid_utf8 = vec![0xFF, 0xFF, 0xFF, 0xFF];
        let params = CString::new(invalid_utf8).unwrap();

        unsafe {
            process_image(width, height, data.as_mut_ptr(), params.as_ptr());
        }
    }

    #[test]
    fn test_invalid_json_params() {
        let width: u32 = 10;
        let height: u32 = 10;
        let buffer_size = (width * height * 4) as usize;
        let mut data = vec![0u8; buffer_size];

        let params = CString::new("this is not json").unwrap();

        unsafe {
            process_image(width, height, data.as_mut_ptr(), params.as_ptr());
        }
    }
}
