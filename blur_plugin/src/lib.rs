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
    let params_str = if params.is_null() {
        ""
    } else {
        // Safety: вызывающий гарантирует, что params - валидная C-строка
        CStr::from_ptr(params).to_str().unwrap_or("")
    };

    let blur_params = serde_json::from_str::<BlurParams>(params_str).unwrap_or_default();

    let w = width as usize;
    let h = height as usize;

    // Для очень маленьких изображений ничего не делаем
    if w < 3 || h < 3 {
        return;
    }

    let radius = blur_params.radius.min(w.min(h) / 5).max(1);
    let iterations = blur_params.iterations.min(3);

    // Safety: вызывающий гарантирует, что rgba_data указывает на память нужного размера
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

    #[test]
    fn test_process_image_modifies_data() {
        let width: u32 = 20;
        let height: u32 = 20;

        let mut data = vec![0u8; (width * height * 4) as usize];

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
        let mut data = vec![100u8; (width * height * 4) as usize];
        let params = CString::new("").unwrap();

        unsafe {
            process_image(width, height, data.as_mut_ptr(), params.as_ptr());
        }
    }
}
