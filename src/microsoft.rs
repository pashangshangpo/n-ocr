use anyhow::Result;
use image::{DynamicImage, GenericImageView};

#[cfg(target_os = "windows")]
pub fn perform_ocr_windows(image: &DynamicImage) -> Result<(String, String, Option<f64>)> {
    use std::io::Cursor;
    use windows::{
        Graphics::Imaging::BitmapDecoder,
        Media::Ocr::OcrEngine as WindowsOcrEngine,
        Storage::Streams::{DataWriter, InMemoryRandomAccessStream},
    };

    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        return Ok(("".to_string(), "[]".to_string(), None));
    }

    let mut buffer = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
        .map_err(|e| anyhow::anyhow!("Failed to write image to buffer: {}", e))?;

    let stream = InMemoryRandomAccessStream::new()?;
    let writer = DataWriter::CreateDataWriter(&stream)?;
    writer.WriteBytes(&buffer)?;
    writer.StoreAsync()?.get()?;
    writer.FlushAsync()?.get()?;
    stream.Seek(0)?;

    let decoder =
        BitmapDecoder::CreateWithIdAsync(BitmapDecoder::PngDecoderId()?, &stream)?.get()?;

    let bitmap = decoder.GetSoftwareBitmapAsync()?.get()?;

    let engine = WindowsOcrEngine::TryCreateFromUserProfileLanguages()?;
    let result = engine.RecognizeAsync(&bitmap)?.get()?;

    let mut text_lines: Vec<String> = Vec::new();
    let mut json_elements: Vec<serde_json::Value> = Vec::new();

    let lines = result.Lines()?;
    for i in 0..lines.Size()? {
        let line = lines.GetAt(i)?;
        let line_text = line.Text()?.to_string();
        text_lines.push(line_text.clone());
        json_elements.push(serde_json::json!({
            "text": line_text,
            "confidence": "1.0",
        }));
    }

    let text = text_lines.join("\n");
    let json_output = serde_json::to_string(&json_elements)?;

    Ok((text, json_output, Some(1.0)))
}