#ifndef OCR_BRIDGE_H
#define OCR_BRIDGE_H

#include <stdint.h>

typedef struct {
    const char *text;
    const char *json;
    double confidence;
} OcrResult;

typedef struct {
    uint8_t *data;
    uint32_t width;
    uint32_t height;
    int32_t success;
} DecodedImage;

OcrResult perform_ocr(const uint8_t *image_data, uint32_t width, uint32_t height,
                       const char *const *languages, uint32_t language_count);

void free_ocr_result(OcrResult result);

DecodedImage decode_image_native(const uint8_t *file_data, uint32_t file_len);

void free_decoded_image(DecodedImage img);

#endif