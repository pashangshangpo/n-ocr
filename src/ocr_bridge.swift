import Foundation
import Vision
import CoreGraphics
import ImageIO

@_cdecl("perform_ocr")
func performOcr(
    imageData: UnsafePointer<UInt8>,
    width: UInt32,
    height: UInt32,
    languages: UnsafePointer<UnsafePointer<CChar>?>?,
    languageCount: UInt32
) -> OcrResult {
    let w = Int(width)
    let h = Int(height)
    let bytesPerRow = w

    let data = Data(bytes: imageData, count: w * h)
    guard let provider = CGDataProvider(data: data as CFData) else {
        return makeEmptyResult()
    }

    let colorSpace = CGColorSpaceCreateDeviceGray()
    let bitmapInfo = CGBitmapInfo(rawValue: 0)

    guard let cgImage = CGImage(
        width: w,
        height: h,
        bitsPerComponent: 8,
        bitsPerPixel: 8,
        bytesPerRow: bytesPerRow,
        space: colorSpace,
        bitmapInfo: bitmapInfo,
        provider: provider,
        decode: nil,
        shouldInterpolate: false,
        intent: .defaultIntent
    ) else {
        return makeEmptyResult()
    }

    var resultText = ""
    var resultJson: [[String: String]] = []
    var totalConfidence: Double = 0

    let request = VNRecognizeTextRequest()
    request.usesLanguageCorrection = false
    request.recognitionLevel = VNRequestTextRecognitionLevel.accurate

    if #available(macOS 13.0, *) {
        request.automaticallyDetectsLanguage = true
    }

    if languageCount > 0, let langs = languages {
        var langArray: [String] = []
        for i in 0..<Int(languageCount) {
            if let ptr = langs[i] {
                langArray.append(String(cString: ptr))
            }
        }
        if !langArray.isEmpty {
            request.recognitionLanguages = langArray
        }
    }

    let handler = VNImageRequestHandler(cgImage: cgImage, options: [:])
    do {
        try handler.perform([request])
    } catch {
        return makeEmptyResult()
    }

    guard let observations = request.results else {
        return makeEmptyResult()
    }

    for obs in observations {
        guard let candidate = obs.topCandidates(1).first else { continue }

        let text: String = candidate.string
        let confidence: Double = Double(candidate.confidence)
        let box: CGRect = obs.boundingBox

        resultText += text + "\n"
        totalConfidence += confidence

        var entry: [String: String] = [:]
        entry["text"] = text
        entry["conf"] = String(confidence)
        entry["left"] = String(Double(box.origin.x))
        entry["top"] = String(Double(box.origin.y))
        entry["width"] = String(Double(box.size.width))
        entry["height"] = String(Double(box.size.height))
        resultJson.append(entry)
    }

    let textC = strdup(resultText) ?? strdup("")!

    var jsonString = "[]"
    if let jsonData = try? JSONSerialization.data(withJSONObject: resultJson) {
        if let s = String(data: jsonData, encoding: .utf8) {
            jsonString = s
        }
    }
    let jsonC = strdup(jsonString) ?? strdup("[]")!

    return OcrResult(text: textC, json: jsonC, confidence: totalConfidence)
}

@_cdecl("free_ocr_result")
func freeOcrResult(_ result: OcrResult) {
    free(UnsafeMutablePointer(mutating: result.text))
    free(UnsafeMutablePointer(mutating: result.json))
}

@_cdecl("decode_image_native")
func decodeImageNative(fileData: UnsafePointer<UInt8>, fileLen: UInt32) -> DecodedImage {
    let data = Data(bytes: fileData, count: Int(fileLen))

    guard let source = CGImageSourceCreateWithData(data as CFData, nil),
          let cgImage = CGImageSourceCreateImageAtIndex(source, 0, nil) else {
        return DecodedImage(data: nil, width: 0, height: 0, success: 0)
    }

    let w = cgImage.width
    let h = cgImage.height
    let bytesPerRow = w * 4
    let totalBytes = bytesPerRow * h

    let buf = UnsafeMutablePointer<UInt8>.allocate(capacity: totalBytes)

    let colorSpace = CGColorSpaceCreateDeviceRGB()
    let bitmapInfo = CGBitmapInfo(rawValue: CGImageAlphaInfo.premultipliedLast.rawValue)

    guard let ctx = CGContext(
        data: buf,
        width: w,
        height: h,
        bitsPerComponent: 8,
        bytesPerRow: bytesPerRow,
        space: colorSpace,
        bitmapInfo: bitmapInfo.rawValue
    ) else {
        buf.deallocate()
        return DecodedImage(data: nil, width: 0, height: 0, success: 0)
    }

    ctx.draw(cgImage, in: CGRect(x: 0, y: 0, width: w, height: h))

    for i in 0..<(w * h) {
        let offset = i * 4
        let r = buf[offset]
        let g = buf[offset + 1]
        let b = buf[offset + 2]
        let a = buf[offset + 3]
        if a > 0 && a < 255 {
            let af = Float(a) / 255.0
            buf[offset]     = UInt8(min(Float(r) / af, 255))
            buf[offset + 1] = UInt8(min(Float(g) / af, 255))
            buf[offset + 2] = UInt8(min(Float(b) / af, 255))
        }
    }

    return DecodedImage(data: buf, width: UInt32(w), height: UInt32(h), success: 1)
}

@_cdecl("free_decoded_image")
func freeDecodedImage(_ img: DecodedImage) {
    if let ptr = img.data {
        ptr.deallocate()
    }
}

func makeEmptyResult() -> OcrResult {
    return OcrResult(text: strdup("")!, json: strdup("[]")!, confidence: 0)
}