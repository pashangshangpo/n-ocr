#ifndef OCR_BRIDGE_H
#define OCR_BRIDGE_H

#include <stdint.h>

typedef struct {
    const char *text;
    const char *json;
    double confidence;
} OcrResult;

OcrResult perform_ocr(const uint8_t *image_data, uint32_t width, uint32_t height,
                       const char *const *languages, uint32_t language_count);

void free_ocr_result(OcrResult result);

#endif