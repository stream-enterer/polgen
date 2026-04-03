// SPLIT: Split from emFontCache.h — bitmap font implementation extracted
//! Text measurement utilities (character/line counting).
//!
//! The actual glyph rendering is handled by `em_font`, which uses the
//! Eagle Mode grayscale font atlas. Text measurement is now inline in
//! `emPainter::GetTextSize` (byte-level C++ port).
