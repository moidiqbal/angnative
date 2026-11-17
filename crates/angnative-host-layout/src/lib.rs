pub mod layout_engine;
/**
 * lib.rs — angnative-host-layout: Taffy-powered Flexbox/Grid layout engine.
 *
 * Layout owner per identity D-4: Rust (taffy) lays out; the platform draws.
 * This crate converts AngNative style properties (§5.6 subset) into Taffy
 * style nodes, runs layout, and produces geometry rectangles for each node.
 *
 * A `TextMeasurer` trait abstracts text measurement so unit tests use a
 * `NaiveMeasurer` while platform shells (CoreText on iOS, StaticLayout on
 * Android) provide real measurement behind the same trait.
 */
pub mod styles;
mod taffy_adapter;
pub mod text_measurer;

pub use layout_engine::{LayoutEngine, LayoutNodeInput, LayoutNodeRect, LayoutResult};
pub use styles::{to_taffy_style, StyleInput};
pub use text_measurer::{
    CachedTextMeasurer, ImageMeasurer, IntrinsicContent, NaiveMeasurer, TextMeasurer, TextSize,
    ZeroImageMeasurer,
};
