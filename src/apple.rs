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

extern "C" {
    fn perform_ocr(
        image_data: *const u8,
        width: u32,
        height: u32,
        languages: *const *const std::ffi::c_char,
        language_count: u32,
    ) -> OcrResult;

    fn free_ocr_result(result: OcrResult);
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