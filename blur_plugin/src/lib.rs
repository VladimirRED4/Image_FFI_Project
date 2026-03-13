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

#[no_mangle]
pub unsafe extern "C" fn process_image(
    width: u32,
    height: u32,
    rgba_data: *mut u8,
    params: *const c_char,
) {
    let params_str = CStr::from_ptr(params).to_str().unwrap_or("");
    let blur_params = serde_json::from_str::<BlurParams>(params_str).unwrap_or_default();

    let w = width as usize;
    let h = height as usize;

    // Для очень маленьких изображений ничего не делаем
    if w < 3 || h < 3 {
        return;
    }

    // Ограничиваем параметры для производительности
    let radius = blur_params.radius.min(w.min(h) / 5).max(1);
    let iterations = blur_params.iterations.min(3);

    let data = std::slice::from_raw_parts_mut(rgba_data, w * h * 4);
    let mut temp = vec![0u8; w * h * 4];

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
                        let idx = (ny * w + nx) * 4;
                        r += data[idx] as u64;
                        g += data[idx + 1] as u64;
                        b += data[idx + 2] as u64;
                        a += data[idx + 3] as u64;
                        cnt += 1;
                    }
                }

                let idx = (y * w + x) * 4;
                if cnt > 0 {
                    temp[idx] = (r / cnt) as u8;
                    temp[idx + 1] = (g / cnt) as u8;
                    temp[idx + 2] = (b / cnt) as u8;
                    temp[idx + 3] = (a / cnt) as u8;
                }
            }
        }
        data.copy_from_slice(&temp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    // Тест с изображением, которое точно изменится
    #[test]
    fn test_process_image_modifies_data() {
        let width: u32 = 20;
        let height: u32 = 20;

        let mut data = vec![0u8; (width * height * 4) as usize];

        // Левая половина - белая, правая - черная
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

    // Тест с пустыми параметрами
    #[test]
    fn test_process_image_default_params() {
        let width: u32 = 10;
        let height: u32 = 10;
        let mut data = vec![100u8; (width * height * 4) as usize];
        let params = CString::new("").unwrap();

        unsafe {
            process_image(width, height, data.as_mut_ptr(), params.as_ptr());
        }
    }

    // Тест с минимальным изображением
    #[test]
    fn test_process_image_minimal_size() {
        let test_sizes = vec![
            (1, 1),
            (2, 2),
            (3, 3),
        ];

        for (w, h) in test_sizes {
            let width: u32 = w;
            let height: u32 = h;
            let mut data = vec![100u8; (width * height * 4) as usize];
            let params = CString::new("{\"radius\":5,\"iterations\":2}").unwrap();

            unsafe {
                process_image(width, height, data.as_mut_ptr(), params.as_ptr());
            }
        }
    }

    // Тест с разными значениями радиуса
    #[test]
    fn test_process_image_different_radii() {
        let width: u32 = 30;
        let height: u32 = 30;
        let mut data = vec![100u8; (width * height * 4) as usize];

        // Создаем паттерн, который точно изменится при размытии
        for y in 0..height as usize {
            for x in 0..width as usize {
                let idx = (y * width as usize + x) * 4;
                // Шахматная доска
                if (x + y) % 2 == 0 {
                    data[idx] = 255;
                    data[idx + 1] = 255;
                    data[idx + 2] = 255;
                } else {
                    data[idx] = 0;
                    data[idx + 1] = 0;
                    data[idx + 2] = 0;
                }
                data[idx + 3] = 255;
            }
        }

        let radii = vec![1, 2, 3];

        for radius in radii {
            let mut test_data = data.clone();
            let params = CString::new(format!("{{\"radius\":{},\"iterations\":1}}", radius)).unwrap();

            unsafe {
                process_image(width, height, test_data.as_mut_ptr(), params.as_ptr());
            }

            // Проверяем, что данные изменились (шахматная доска должна размыться)
            let changed = test_data.iter().zip(data.iter()).any(|(a, b)| a != b);
            assert!(changed, "Data should change with radius {}", radius);
        }
    }

    // Тест на создание градиента
    #[test]
    fn test_process_image_creates_gradient() {
        let width: u32 = 20;
        let height: u32 = 20; // Используем квадратное изображение

        let mut data = vec![0u8; (width * height * 4) as usize];

        // Создаем черно-белую границу по вертикали
        for y in 0..height as usize {
            for x in 0..width as usize {
                let idx = (y * width as usize + x) * 4;
                if x < (width as usize / 2) {
                    data[idx] = 255;
                    data[idx + 1] = 255;
                    data[idx + 2] = 255;
                } else {
                    data[idx] = 0;
                    data[idx + 1] = 0;
                    data[idx + 2] = 0;
                }
                data[idx + 3] = 255;
            }
        }

        let params = CString::new("{\"radius\":3,\"iterations\":1}").unwrap();

        unsafe {
            process_image(width, height, data.as_mut_ptr(), params.as_ptr());
        }

        // Проверяем пиксели рядом с границей в середине изображения
        let mid_y = (height as usize / 2) * width as usize;

        // Пиксели слева от границы должны стать темнее
        let left_idx = (mid_y + (width as usize / 2 - 1)) * 4;
        assert!(data[left_idx] < 255,
                "Left of boundary should be darker: {} < 255", data[left_idx]);

        // Пиксели справа от границы должны стать светлее
        let right_idx = (mid_y + (width as usize / 2)) * 4;
        assert!(data[right_idx] > 0,
                "Right of boundary should be lighter: {} > 0", data[right_idx]);

        // Пиксели дальше от границы должны меняться меньше
        let far_left_idx = (mid_y + 2) * 4;
        let far_right_idx = (mid_y + (width as usize - 3)) * 4;

        assert!(data[far_left_idx] >= data[left_idx],
                "Far left should be brighter than near left");
        assert!(data[far_right_idx] <= data[right_idx],
                "Far right should be darker than near right");
    }
}