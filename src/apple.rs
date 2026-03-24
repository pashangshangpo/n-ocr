use anyhow::{Context, Result};
use image::DynamicImage;
use image::GenericImageView;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::sync::OnceLock;

use crate::Language;

#[repr(C)]
struct OcrResult {
    text: *const std::ffi::c_char,
    json: *const std::ffi::c_char,
    confidence: f64,
}

#[repr(C)]
struct DecodedImage {
    data: *mut u8,
    width: u32,
    height: u32,
    success: i32,
}

extern "C" {
    fn perform_ocr(
        image_data: *const u8,
        width: u32,
        height: u32,
        languages: *const *const std::ffi::c_char,
        language_count: u32,
    ) -> OcrResult;

    fn free_ocr_result(result: OcrResult);

    fn decode_image_native(
        file_data: *const u8,
        file_len: u32,
    ) -> DecodedImage;

    fn free_decoded_image(img: DecodedImage);
}

static APPLE_LANGUAGE_MAP: OnceLock<HashMap<Language, &'static str>> = OnceLock::new();

fn get_apple_languages(languages: &[Language]) -> Vec<String> {
    let map = APPLE_LANGUAGE_MAP.get_or_init(|| {
        let mut m = HashMap::new();
        m.insert(Language::English, "en-US");
        m.insert(Language::Spanish, "es-ES");
        m.insert(Language::French, "fr-FR");
        m.insert(Language::German, "de-DE");
        m.insert(Language::Italian, "it-IT");
        m.insert(Language::Portuguese, "pt-BR");
        m.insert(Language::Russian, "ru-RU");
        m.insert(Language::Chinese, "zh-Hans");
        m.insert(Language::Korean, "ko-KR");
        m.insert(Language::Japanese, "ja-JP");
        m.insert(Language::Ukrainian, "uk-UA");
        m.insert(Language::Thai, "th-TH");
        m.insert(Language::Arabic, "ar-SA");
        m
    });

    languages
        .iter()
        .filter_map(|lang| map.get(lang).map(|&s| s.to_string()))
        .collect()
}

pub fn decode_native(bytes: &[u8]) -> Result<DynamicImage> {
    unsafe {
        let result = decode_image_native(bytes.as_ptr(), bytes.len() as u32);
        if result.success == 0 || result.data.is_null() {
            return Err(anyhow::anyhow!("Failed to decode image via native decoder"));
        }

        let w = result.width;
        let h = result.height;
        let len = (w * h * 4) as usize;
        let pixels = std::slice::from_raw_parts(result.data, len).to_vec();

        free_decoded_image(result);

        let rgba = image::RgbaImage::from_raw(w, h, pixels)
            .context("Failed to construct RgbaImage from decoded data")?;

        Ok(DynamicImage::ImageRgba8(rgba))
    }
}

pub fn perform_ocr_apple(
    image: &DynamicImage,
    languages: &[Language],
) -> (String, String, Option<f64>) {
    let (width, height) = image.dimensions();
    let gray = image.grayscale().to_luma8();
    let raw_data = gray.as_raw();

    let apple_langs = get_apple_languages(languages);
    let c_strings: Vec<CString> = apple_langs
        .iter()
        .map(|s| CString::new(s.as_str()).unwrap())
        .collect();
    let c_ptrs: Vec<*const std::ffi::c_char> = c_strings.iter().map(|s| s.as_ptr()).collect();

    let lang_ptr = if c_ptrs.is_empty() {
        std::ptr::null()
    } else {
        c_ptrs.as_ptr()
    };

    unsafe {
        let result = perform_ocr(
            raw_data.as_ptr(),
            width,
            height,
            lang_ptr,
            c_ptrs.len() as u32,
        );

        let text = CStr::from_ptr(result.text).to_string_lossy().into_owned();
        let json = CStr::from_ptr(result.json).to_string_lossy().into_owned();
        let confidence = result.confidence;

        free_ocr_result(result);

        (text, json, Some(confidence))
    }
}