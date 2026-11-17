//! Host supplied intrinsic measurement callbacks and a small thread-safe cache.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex};

/// Measured intrinsic content size in logical layout units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextSize {
    pub width: f32,
    pub height: f32,
}

/// Content that requires host measurement rather than Taffy's zero-size default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntrinsicContent {
    Text(String),
    Image(String),
}

/// Measure text at a given font configuration and width constraint.
pub trait TextMeasurer: Send + Sync {
    fn measure_text(
        &self,
        text: &str,
        font_size: f32,
        font_weight: &str,
        available_width: Option<f32>,
    ) -> TextSize;
}

/// Host callback for image intrinsic dimensions, including aspect-ratio-aware
/// sizing under the constraints supplied by Taffy.
pub trait ImageMeasurer: Send + Sync {
    fn measure_image(
        &self,
        source: &str,
        available_width: Option<f32>,
        available_height: Option<f32>,
    ) -> TextSize;
}

/// Fallback used when an app has not supplied an image-measurement callback.
#[derive(Debug, Default, Clone, Copy)]
pub struct ZeroImageMeasurer;

impl ImageMeasurer for ZeroImageMeasurer {
    fn measure_image(&self, _source: &str, _width: Option<f32>, _height: Option<f32>) -> TextSize {
        TextSize {
            width: 0.0,
            height: 0.0,
        }
    }
}

/// Deterministic approximation for tests and early host development.
#[derive(Debug, Default, Clone, Copy)]
pub struct NaiveMeasurer;

impl TextMeasurer for NaiveMeasurer {
    fn measure_text(
        &self,
        text: &str,
        font_size: f32,
        _weight: &str,
        width: Option<f32>,
    ) -> TextSize {
        let font_size = if font_size.is_finite() && font_size > 0.0 {
            font_size
        } else {
            16.0
        };
        let line_height = font_size * 1.2;
        let natural_width = text.chars().count() as f32 * font_size * 0.5;
        match width.filter(|width| width.is_finite() && *width >= 0.0) {
            Some(width) if natural_width > width && width > 0.0 => {
                let lines = (natural_width / width).ceil().max(1.0);
                TextSize {
                    width,
                    height: line_height * lines,
                }
            }
            _ => TextSize {
                width: natural_width,
                height: line_height,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TextMeasureKey {
    text: String,
    font_size_bits: u32,
    font_weight: String,
    available_width_bits: Option<u32>,
}

/// Thread-safe text cache. Float keys are stored by exact IEEE bit pattern so
/// invalid `Eq`/`Hash` implementations over `f32` are avoided.
pub struct CachedTextMeasurer<M: TextMeasurer> {
    inner: Arc<M>,
    cache: Mutex<HashMap<TextMeasureKey, TextSize>>,
}

impl<M: TextMeasurer> CachedTextMeasurer<M> {
    pub fn new(inner: M) -> Self {
        Self {
            inner: Arc::new(inner),
            cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn cached_entries(&self) -> usize {
        self.cache.lock().map_or(0, |cache| cache.len())
    }
}

impl<M: TextMeasurer> TextMeasurer for CachedTextMeasurer<M> {
    fn measure_text(
        &self,
        text: &str,
        font_size: f32,
        weight: &str,
        width: Option<f32>,
    ) -> TextSize {
        let key = TextMeasureKey {
            text: text.to_owned(),
            font_size_bits: font_size.to_bits(),
            font_weight: weight.to_owned(),
            available_width_bits: width.map(f32::to_bits),
        };
        if let Ok(cache) = self.cache.lock() {
            if let Some(size) = cache.get(&key) {
                return *size;
            }
        }

        let size = self.inner.measure_text(text, font_size, weight, width);
        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(key, size);
        }
        size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn naive_measurer_scales_with_font_size_and_wraps() {
        let measurer = NaiveMeasurer;
        let small = measurer.measure_text("hi", 10.0, "normal", None);
        let large = measurer.measure_text("hi", 20.0, "normal", None);
        assert!((large.width - small.width * 2.0).abs() < 0.01);
        let wrapped = measurer.measure_text("abcdefgh", 10.0, "normal", Some(20.0));
        assert_eq!(wrapped.width, 20.0);
        assert!(wrapped.height > 12.0);
    }

    struct CountingMeasurer(AtomicUsize);
    impl TextMeasurer for CountingMeasurer {
        fn measure_text(&self, text: &str, size: f32, _: &str, _: Option<f32>) -> TextSize {
            self.0.fetch_add(1, Ordering::SeqCst);
            TextSize {
                width: text.len() as f32 * size,
                height: size,
            }
        }
    }

    #[test]
    fn cache_reuses_exact_measurement_inputs() {
        let inner = CountingMeasurer(AtomicUsize::new(0));
        let cached = CachedTextMeasurer::new(inner);
        let first = cached.measure_text("abc", 12.0, "600", Some(40.0));
        let second = cached.measure_text("abc", 12.0, "600", Some(40.0));
        assert_eq!(first, second);
        assert_eq!(cached.cached_entries(), 1);
        assert_eq!(cached.inner.0.load(Ordering::SeqCst), 1);
    }
}
