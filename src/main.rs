mod language;
#[cfg(target_os = "macos")]
mod apple;
#[cfg(target_os = "windows")]
mod microsoft;

use anyhow::{Context, Result};
use clap::Parser;
use language::Language;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "nocr", about = "")]
struct Cli {
    #[arg(help = "")]
    image: Option<PathBuf>,

    #[arg(long, help = "Image URL")]
    url: Option<String>,

    #[arg(short, long, default_value = "zh", help = "Language code")]
    language: Vec<String>,

    #[arg(short, long, help = "Output as JSON")]
    json: bool,
}

fn load_image_from_bytes(bytes: &[u8]) -> Result<image::DynamicImage> {
    if let Ok(img) = image::load_from_memory(bytes) {
        return Ok(img);
    }

    #[cfg(target_os = "macos")]
    {
        return apple::decode_native(bytes);
    }

    #[cfg(target_os = "windows")]
    {
        return microsoft::decode_native(bytes);
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err(anyhow::anyhow!("Unsupported image format"))
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let image = if let Some(url) = &cli.url {
        let bytes = minreq::get(url)
            .send()
            .with_context(|| format!("Failed to fetch image from URL '{}'", url))?
            .into_bytes();
        load_image_from_bytes(&bytes)
            .context("Failed to decode image from URL")?
    } else if let Some(path) = &cli.image {
        let bytes = std::fs::read(path)
            .with_context(|| format!("Failed to read file '{}'", path.display()))?;
        load_image_from_bytes(&bytes)
            .with_context(|| format!("Failed to decode image '{}'", path.display()))?
    } else {
        return Err(anyhow::anyhow!("Either an image path or --url must be provided"));
    };

    let languages: Vec<Language> = cli
        .language
        .iter()
        .filter_map(|code| Language::from_code(code))
        .collect();

    let languages = if languages.is_empty() {
        vec![Language::Chinese]
    } else {
        languages
    };

    let (text, json_output, confidence) = perform_ocr(&image, &languages)?;

    if cli.json {
        let output = serde_json::json!({
            "text": text,
            "elements": serde_json::from_str::<serde_json::Value>(&json_output).unwrap_or(serde_json::Value::Array(vec![])),
            "confidence": confidence,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        print!("{}", text);
    }

    Ok(())
}

fn perform_ocr(
    image: &image::DynamicImage,
    languages: &[Language],
) -> Result<(String, String, Option<f64>)> {
    #[cfg(target_os = "macos")]
    {
        Ok(apple::perform_ocr_apple(image, languages))
    }

    #[cfg(target_os = "windows")]
    {
        microsoft::perform_ocr_windows(image)
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = (image, languages);
        Err(anyhow::anyhow!("No OCR engine available for this platform"))
    }
}