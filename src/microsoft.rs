use anyhow::Result;
use image::{DynamicImage, GenericImageView};

#[cfg(target_os = "windows")]
use windows::{
    Graphics::Imaging::BitmapDecoder,
    Media::Ocr::OcrEngine as WindowsOcrEngine,
    Storage::Streams::{DataWriter, InMemoryRandomAccessStream},
};

#[cfg(target_os = "windows")]
fn bytes_to_stream(
    buffer: &[u8],
) -> Result<InMemoryRandomAccessStream> {
    let stream = InMemoryRandomAccessStream::new()?;
    let writer = DataWriter::CreateDataWriter(&stream)?;
    writer.WriteBytes(buffer)?;
    writer.StoreAsync()?.get()?;
    writer.FlushAsync()?.get()?;
    stream.Seek(0)?;
    Ok(stream)
}

#[cfg(target_os = "windows")]
pub fn decode_native(bytes: &[u8]) -> Result<DynamicImage> {
    use windows::Graphics::Imaging::BitmapPixelFormat;

    let stream = bytes_to_stream(bytes)?;

    let decoder = BitmapDecoder::CreateAsync(&stream)?.get()?;

    let transform = decoder.BitmapTransform()?;
    let pf = BitmapPixelFormat::Rgba8;
    let bitmap = decoder
        .GetPixelDataAsync(
            pf,
            windows::Graphics::Imaging::BitmapAlphaMode::Premultiplied,
            &transform,
            windows::Graphics::Imaging::ExifOrientationMode::RespectExifOrientation,
            windows::Graphics::Imaging::ColorManagementMode::ColorManageToSRgb,
        )?
        .get()?;

    let pixel_data = bitmap.DetachPixelData()?;
    let width = decoder.PixelWidth()? ;
    let height = decoder.PixelHeight()?;

    let mut pixels = pixel_data.to_vec();

    for chunk in pixels.chunks_exact_mut(4) {
        let a = chunk[3] as u32;
        if a > 0 && a < 255 {
            chunk[0] = ((chunk[0] as u32 * 255) / a).min(255) as u8;
            chunk[1] = ((chunk[1] as u32 * 255) / a).min(255) as u8;
            chunk[2] = ((chunk[2] as u32 * 255) / a).min(255) as u8;
        }
    }

    let rgba = image::RgbaImage::from_raw(width, height, pixels)
        .ok_or_else(|| anyhow::anyhow!("Failed to construct RgbaImage from WIC decoded data"))?;

    Ok(DynamicImage::ImageRgba8(rgba))
}

#[cfg(target_os = "windows")]
pub fn perform_ocr_windows(image: &DynamicImage) -> Result<(String, String, Option<f64>)> {
    use std::io::Cursor;

    let (width, height) = image.dimensions();
    if width == 0 || height == 0 {
        return Ok(("".to_string(), "[]".to_string(), None));
    }

    let mut buffer = Vec::new();
    image
        .write_to(&mut Cursor::new(&mut buffer), image::ImageFormat::Png)
        .map_err(|e| anyhow::anyhow!("Failed to write image to buffer: {}", e))?;

    let stream = bytes_to_stream(&buffer)?;

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