mod language;
#[cfg(target_os = "macos")]
mod apple;
#[cfg(target_os = "windows")]
mod microsoft;

use anyhow::Result;
use clap::Parser;
use language::Language;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "uni-ocr", about = "Native OCR CLI tool")]
struct Cli {
    #[arg(help = "Path to image file")]
    image: PathBuf,

    #[arg(short, long, default_value = "en", help = "Language code")]
    language: Vec<String>,

    #[arg(short, long, help = "Output as JSON")]
    json: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let image = image::open(&cli.image)
        .map_err(|e| anyhow::anyhow!("Failed to open image '{}': {}", cli.image.display(), e))?;

    let languages: Vec<Language> = cli
        .language
        .iter()
        .filter_map(|code| Language::from_code(code))
        .collect();

    let languages = if languages.is_empty() {
        vec![Language::English]
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