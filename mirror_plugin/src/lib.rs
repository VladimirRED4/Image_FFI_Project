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
    let params_str = if params.is_null() {
        ""
    } else {
        // Safety: вызывающий гарантирует, что params - валидная C-строка
        CStr::from_ptr(params).to_str().unwrap_or("")
    };

    let mirror_params: MirrorParams = serde_json::from_str(params_str).unwrap_or_default();

    let w = width as usize;
    let h = height as usize;

    // Safety: вызывающий гарантирует, что rgba_data указывает на память нужного размера
    let data = std::slice::from_raw_parts_mut(rgba_data, w * h * 4);

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
            let left_idx = (y * width + x) * 4;
            let right_idx = (y * width + (width - 1 - x)) * 4;

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
        let mut data = vec![0u8; (width * height * 4) as usize];

        // Заполняем данные: каждому пикселю даем уникальное значение по его x-координате
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

        // Проверяем результат
        for y in 0..height as usize {
            for x in 0..width as usize {
                let idx = (y * width as usize + x) * 4;
                let expected_x = (width as usize - 1 - x) as u8;

                assert_eq!(
                    data[idx], expected_x,
                    "Pixel at ({}, {}) should have value {}, but has {}",
                    x, y, expected_x, data[idx]
                );
            }
        }
    }

    #[test]
    fn test_mirror_vertical() {
        let width: u32 = 3;
        let height: u32 = 5;
        let mut data = vec![0u8; (width * height * 4) as usize];

        // Заполняем данные: каждому пикселю даем уникальное значение по его y-координате
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

        // Проверяем результат
        for y in 0..height as usize {
            for x in 0..width as usize {
                let idx = (y * width as usize + x) * 4;
                let expected_y = (height as usize - 1 - y) as u8;

                assert_eq!(
                    data[idx], expected_y,
                    "Pixel at ({}, {}) should have value {}, but has {}",
                    x, y, expected_y, data[idx]
                );
            }
        }
    }

    #[test]
    fn test_mirror_both() {
        let width: u32 = 3;
        let height: u32 = 3;
        let mut data = vec![0u8; (width * height * 4) as usize];
        let mut expected = vec![0u8; (width * height * 4) as usize];

        // Заполняем данные уникальными значениями
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
            assert_eq!(
                data[i], expected[i],
                "Double mirror should produce diagonal flip at index {}",
                i
            );
        }
    }

    #[test]
    fn test_mirror_twice() {
        let width: u32 = 4;
        let height: u32 = 4;
        let mut data = vec![0u8; (width * height * 4) as usize];

        // Заполняем данные уникальными значениями
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

        // Применяем двойное отражение дважды
        unsafe {
            process_image(width, height, data.as_mut_ptr(), params.as_ptr());
            process_image(width, height, data.as_mut_ptr(), params.as_ptr());
        }

        // После двух применений должно вернуться к исходному
        for i in 0..data.len() {
            assert_eq!(
                data[i], original[i],
                "Double mirror twice should return to original at index {}",
                i
            );
        }
    }
}
