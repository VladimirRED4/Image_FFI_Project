use serde::Deserialize;
use std::ffi::CStr;
use std::os::raw::c_char;

#[derive(Debug, Deserialize)]
#[serde(default)]
struct BlurParams {
    radius: usize,
    iterations: usize,
}

impl Default for BlurParams {
    fn default() -> Self {
        BlurParams {
            radius: 3,
            iterations: 1,
        }
    }
}

/// Применяет размытие к изображению.
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
    if buffer_size > 1024 * 1024 * 1024 {
        eprintln!("Warning: buffer size too large ({} bytes)", buffer_size);
        return;
    }

    let params_str = if params.is_null() {
        ""
    } else {
        match CStr::from_ptr(params).to_str() {
            Ok(s) => s,
            Err(_) => {
                eprintln!("Error: invalid UTF-8 in params");
                return;
            }
        }
    };

    let blur_params: BlurParams = match serde_json::from_str(params_str) {
        Ok(p) => p,
        Err(_) => {
            eprintln!("Warning: failed to parse params, using defaults");
            BlurParams::default()
        }
    };

    // Для очень маленьких изображений ничего не делаем
    if w < 3 || h < 3 {
        return;
    }

    // Корректируем радиус: если радиус 0, используем 1
    let radius = if blur_params.radius == 0 {
        1
    } else {
        blur_params.radius.min(w.min(h) / 5).max(1)
    };

    let iterations = blur_params.iterations.min(3);

    let data = std::slice::from_raw_parts_mut(rgba_data, buffer_size);
    let mut temp = vec![0u8; buffer_size];

    for _ in 0..iterations {
        for y in 0..h {
            for x in 0..w {
                let (mut r, mut g, mut b, mut a, mut cnt) = (0u64, 0u64, 0u64, 0u64, 0);

                let y0 = y.saturating_sub(radius);
                let y1 = (y + radius).min(h - 1);
                let x0 = x.saturating_sub(radius);
                let x1 = (x + radius).min(w - 1);

                for ny in y0..=y1 {
                    for nx in x0..=x1 {
                        let idx = match (ny * w + nx).checked_mul(4) {
                            Some(i) => i,
                            None => continue,
                        };

                        if idx + 3 >= data.len() {
                            continue;
                        }

                        r += data[idx] as u64;
                        g += data[idx + 1] as u64;
                        b += data[idx + 2] as u64;
                        a += data[idx + 3] as u64;
                        cnt += 1;
                    }
                }

                let idx = match (y * w + x).checked_mul(4) {
                    Some(i) => i,
                    None => continue,
                };

                if idx + 3 >= temp.len() || cnt == 0 {
                    continue;
                }

                temp[idx] = (r / cnt) as u8;
                temp[idx + 1] = (g / cnt) as u8;
                temp[idx + 2] = (b / cnt) as u8;
                temp[idx + 3] = (a / cnt) as u8;
            }
        }
        data.copy_from_slice(&temp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    #[test]
    fn test_process_image_modifies_data() {
        let width: u32 = 20;
        let height: u32 = 20;
        let buffer_size = (width * height * 4) as usize;
        let mut data = vec![0u8; buffer_size];

        // Создаем изображение с четкой границей
        for y in 0..height as usize {
            for x in 0..width as usize {
                let idx = (y * width as usize + x) * 4;
                if x < (width as usize / 2) {
                    data[idx] = 255;
                    data[idx + 1] = 255;
                    data[idx + 2] = 255;
                    data[idx + 3] = 255;
                } else {
                    data[idx] = 0;
                    data[idx + 1] = 0;
                    data[idx + 2] = 0;
                    data[idx + 3] = 255;
                }
            }
        }

        let original = data.clone();
        let params = CString::new("{\"radius\":3,\"iterations\":1}").unwrap();

        unsafe {
            process_image(width, height, data.as_mut_ptr(), params.as_ptr());
        }

        let changed = data.iter().zip(original.iter()).any(|(a, b)| a != b);
        assert!(changed, "Image data should be modified by blur");
    }

    #[test]
    fn test_process_image_default_params() {
        let width: u32 = 10;
        let height: u32 = 10;
        let buffer_size = (width * height * 4) as usize;
        let mut data = vec![100u8; buffer_size];
        let params = CString::new("").unwrap();

        unsafe {
            process_image(width, height, data.as_mut_ptr(), params.as_ptr());
        }
    }

    #[test]
    fn test_zero_dimensions() {
        let mut data = vec![0u8; 4];
        let params = CString::new("{\"radius\":3}").unwrap();

        unsafe {
            process_image(0, 10, data.as_mut_ptr(), params.as_ptr());
            process_image(10, 0, data.as_mut_ptr(), params.as_ptr());
            process_image(0, 0, data.as_mut_ptr(), params.as_ptr());
        }
    }

    #[test]
    fn test_null_data_pointer() {
        let params = CString::new("{\"radius\":3}").unwrap();

        unsafe {
            process_image(10, 10, std::ptr::null_mut(), params.as_ptr());
        }
    }

    #[test]
    fn test_small_buffer() {
        let width: u32 = 15;
        let height: u32 = 15;
        let buffer_size = (width * height * 4) as usize;
        let mut data = vec![100u8; buffer_size];

        // Создаем шахматную доску для гарантированного размытия
        for y in 0..height as usize {
            for x in 0..width as usize {
                let idx = (y * width as usize + x) * 4;
                let value = if (x + y) % 2 == 0 { 255 } else { 0 };
                data[idx] = value;
                data[idx + 1] = value;
                data[idx + 2] = value;
                data[idx + 3] = 255;
            }
        }

        let original = data.clone();
        let params = CString::new("{\"radius\":2,\"iterations\":1}").unwrap();

        unsafe {
            process_image(width, height, data.as_mut_ptr(), params.as_ptr());
        }

        // Проверяем, что пиксели на границах изменились
        let mut changed = false;
        for y in 1..height as usize - 1 {
            for x in 1..width as usize - 1 {
                let idx = (y * width as usize + x) * 4;
                if data[idx] != original[idx] {
                    changed = true;
                    break;
                }
            }
        }

        assert!(changed, "Data should be modified by blur");
    }

    #[test]
    fn test_extreme_dimensions() {
        let mut data = vec![0u8; 4];
        let params = CString::new("{\"radius\":3}").unwrap();

        unsafe {
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

    #[test]
    fn test_extreme_radius() {
        let width: u32 = 30;
        let height: u32 = 30;
        let buffer_size = (width * height * 4) as usize;
        let mut data = vec![0u8; buffer_size];

        // Создаем градиент
        for y in 0..height as usize {
            for x in 0..width as usize {
                let idx = (y * width as usize + x) * 4;
                let value = (x * 255 / width as usize) as u8;
                data[idx] = value;
                data[idx + 1] = value;
                data[idx + 2] = value;
                data[idx + 3] = 255;
            }
        }

        let original = data.clone();
        let params = CString::new("{\"radius\":1000,\"iterations\":10}").unwrap();

        unsafe {
            process_image(width, height, data.as_mut_ptr(), params.as_ptr());
        }

        // Проверяем, что размытие сгладило градиент
        let changed = data.iter().zip(original.iter()).any(|(a, b)| a != b);
        assert!(changed, "Data should be modified by extreme radius");
    }

    #[test]
    fn test_zero_radius_behavior() {
        let width: u32 = 20;
        let height: u32 = 20;
        let buffer_size = (width * height * 4) as usize;

        // Создаем тестовое изображение
        let mut data_zero = vec![0u8; buffer_size];
        let mut data_one = vec![0u8; buffer_size];

        for y in 0..height as usize {
            for x in 0..width as usize {
                let idx = (y * width as usize + x) * 4;
                let value = if x < width as usize / 2 { 255 } else { 0 };
                data_zero[idx] = value;
                data_zero[idx + 1] = value;
                data_zero[idx + 2] = value;
                data_zero[idx + 3] = 255;

                data_one[idx] = value;
                data_one[idx + 1] = value;
                data_one[idx + 2] = value;
                data_one[idx + 3] = 255;
            }
        }

        // Тест с радиусом 0 (должен обрабатываться как радиус 1)
        let params_zero = CString::new("{\"radius\":0}").unwrap();
        unsafe {
            process_image(width, height, data_zero.as_mut_ptr(), params_zero.as_ptr());
        }

        // Тест с радиусом 1
        let params_one = CString::new("{\"radius\":1}").unwrap();
        unsafe {
            process_image(width, height, data_one.as_mut_ptr(), params_one.as_ptr());
        }

        // Оба должны изменить данные
        assert!(
            data_zero.iter().any(|&x| x != 0 && x != 255),
            "Data should change with radius 0"
        );
        assert!(
            data_one.iter().any(|&x| x != 0 && x != 255),
            "Data should change with radius 1"
        );

        // Оба должны иметь промежуточные значения
        assert!(
            data_zero.iter().any(|&x| x > 0 && x < 255),
            "Radius 0 should create intermediate values"
        );
        assert!(
            data_one.iter().any(|&x| x > 0 && x < 255),
            "Radius 1 should create intermediate values"
        );
    }
}
