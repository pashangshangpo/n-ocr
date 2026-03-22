import Foundation
import Vision
import CoreGraphics

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

    guard let provider = CGDataProvider(data: Data(bytes: imageData, count: w * h) as CFData),
          let cgImage = CGImage(
              width: w,
              height: h,
              bitsPerComponent: 8,
              bitsPerPixel: 8,
              bytesPerRow: bytesPerRow,
              space: CGColorSpaceCreateDeviceGray(),
              bitmapInfo: CGBitmapInfo(rawValue: 0),
              provider: provider,
              decode: nil,
              shouldInterpolate: false,
              intent: .defaultIntent
          ) else {
        return makeEmptyResult()
    }

    let semaphore = DispatchSemaphore(value: 0)
    var resultText = ""
    var resultJson: [[String: String]] = []
    var totalConfidence: Double = 0

    let request = VNRecognizeTextRequest { req, error in
        defer { semaphore.signal() }

        guard error == nil,
              let observations = req.results as? [VNRecognizedTextObservation] else {
            return
        }

        for obs in observations {
            guard let candidate = obs.topCandidates(1).first else { continue }

            let text = candidate.string
            let confidence = Double(candidate.confidence)
            let box = obs.boundingBox

            resultText += text + "\n"
            totalConfidence += confidence

            resultJson.append([
                "text": text,
                "conf": String(confidence),
                "left": String(box.origin.x),
                "top": String(box.origin.y),
                "width": String(box.size.width),
                "height": String(box.size.height),
            ])
        }
    }

    if #available(macOS 13.0, *) {
        request.automaticallyDetectsLanguage = true
    }

    request.usesLanguageCorrection = false
    request.recognitionLevel = .accurate

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

    semaphore.wait()

    let textC = strdup(resultText) ?? strdup("")!
    let jsonData = (try? JSONSerialization.data(withJSONObject: resultJson)) ?? Data("[]".utf8)
    let jsonString = String(data: jsonData, encoding: .utf8) ?? "[]"
    let jsonC = strdup(jsonString) ?? strdup("[]")!

    return OcrResult(text: textC, json: jsonC, confidence: totalConfidence)
}

@_cdecl("free_ocr_result")
func freeOcrResult(_ result: OcrResult) {
    free(UnsafeMutablePointer(mutating: result.text))
    free(UnsafeMutablePointer(mutating: result.json))
}

func makeEmptyResult() -> OcrResult {
    return OcrResult(text: strdup("")!, json: strdup("[]")!, confidence: 0)
}