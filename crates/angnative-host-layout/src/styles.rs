//! Translate the supported AngNative CSS subset to Taffy 0.16.0 styles.

use crate::taffy_adapter::style::{
    AlignContent, AlignItems, BoxSizing, Dimension, Direction, Display, FlexDirection, FlexWrap,
    JustifyContent, LengthPercentage, LengthPercentageAuto, Position, Rect, Size, Style,
};

/// Canonicalized properties from the host command protocol.
pub type StyleInput = std::collections::HashMap<String, String>;

fn parse_finite_number(value: &str) -> Option<f32> {
    let number = value.trim().parse::<f32>().ok()?;
    number.is_finite().then_some(number)
}

fn parse_numeric_length(value: &str) -> Option<(f32, bool)> {
    let value = value.trim();
    if let Some(number) = value.strip_suffix('%') {
        let number = parse_finite_number(number)?;
        return Some((number / 100.0, true));
    }
    if let Some(number) = value.strip_suffix("px") {
        return Some((parse_finite_number(number)?, false));
    }
    Some((parse_finite_number(value)?, false))
}

fn parse_dimension(value: &str) -> Option<Dimension> {
    if value.trim().eq_ignore_ascii_case("auto") {
        return Some(Dimension::auto());
    }
    let (number, percent) = parse_numeric_length(value)?;
    (number >= 0.0).then_some(if percent {
        Dimension::percent(number)
    } else {
        Dimension::length(number)
    })
}

fn parse_length_percentage_auto(value: &str, allow_negative: bool) -> Option<LengthPercentageAuto> {
    if value.trim().eq_ignore_ascii_case("auto") {
        return Some(LengthPercentageAuto::auto());
    }
    let (number, percent) = parse_numeric_length(value)?;
    if !allow_negative && number < 0.0 {
        return None;
    }
    Some(if percent {
        LengthPercentageAuto::percent(number)
    } else {
        LengthPercentageAuto::length(number)
    })
}

fn parse_length_percentage(value: &str) -> Option<LengthPercentage> {
    let (number, percent) = parse_numeric_length(value)?;
    if number < 0.0 {
        return None;
    }
    Some(if percent {
        LengthPercentage::percent(number)
    } else {
        LengthPercentage::length(number)
    })
}

fn set_rect<T: Copy>(rect: &mut Rect<T>, side: &str, value: T) {
    match side {
        "top" => rect.top = value,
        "right" => rect.right = value,
        "bottom" => rect.bottom = value,
        "left" => rect.left = value,
        _ => unreachable!("validated rectangle side"),
    }
}

fn parse_rect_shorthand<T: Copy>(
    value: &str,
    mut parse: impl FnMut(&str) -> Option<T>,
) -> Option<Rect<T>> {
    let values = value.split_ascii_whitespace().collect::<Vec<_>>();
    let parsed = values
        .iter()
        .map(|value| parse(value))
        .collect::<Option<Vec<_>>>()?;
    match parsed.as_slice() {
        [all] => Some(Rect {
            left: *all,
            right: *all,
            top: *all,
            bottom: *all,
        }),
        [vertical, horizontal] => Some(Rect {
            left: *horizontal,
            right: *horizontal,
            top: *vertical,
            bottom: *vertical,
        }),
        [top, horizontal, bottom] => Some(Rect {
            left: *horizontal,
            right: *horizontal,
            top: *top,
            bottom: *bottom,
        }),
        [top, right, bottom, left] => Some(Rect {
            left: *left,
            right: *right,
            top: *top,
            bottom: *bottom,
        }),
        _ => None,
    }
}

fn justify_content(value: &str) -> Option<JustifyContent> {
    Some(match value {
        "normal" => JustifyContent::NORMAL,
        "start" => JustifyContent::START,
        "end" => JustifyContent::END,
        "flex-start" => JustifyContent::FLEX_START,
        "flex-end" => JustifyContent::FLEX_END,
        "center" => JustifyContent::CENTER,
        "space-between" => JustifyContent::SPACE_BETWEEN,
        "space-around" => JustifyContent::SPACE_AROUND,
        "space-evenly" => JustifyContent::SPACE_EVENLY,
        _ => return None,
    })
}

fn align_items(value: &str) -> Option<AlignItems> {
    Some(match value {
        "normal" => AlignItems::NORMAL,
        "stretch" => AlignItems::STRETCH,
        "start" => AlignItems::START,
        "end" => AlignItems::END,
        "flex-start" => AlignItems::FLEX_START,
        "flex-end" => AlignItems::FLEX_END,
        "center" => AlignItems::CENTER,
        "baseline" => AlignItems::BASELINE,
        "self-start" => AlignItems::SELF_START,
        "self-end" => AlignItems::SELF_END,
        _ => return None,
    })
}

fn align_content(value: &str) -> Option<AlignContent> {
    Some(match value {
        "normal" => AlignContent::NORMAL,
        "stretch" => AlignContent::STRETCH,
        "start" => AlignContent::START,
        "end" => AlignContent::END,
        "flex-start" => AlignContent::FLEX_START,
        "flex-end" => AlignContent::FLEX_END,
        "center" => AlignContent::CENTER,
        "space-between" => AlignContent::SPACE_BETWEEN,
        "space-around" => AlignContent::SPACE_AROUND,
        "space-evenly" => AlignContent::SPACE_EVENLY,
        _ => return None,
    })
}

fn invalid(key: &str, value: &str) -> String {
    format!("Invalid {key}: {value}")
}

/// Convert an AngNative style map into the Taffy style used by the adapter.
///
/// Unsupported layout properties fail loudly. Paint-only properties are
/// accepted here and consumed by the host-view layer.
pub fn to_taffy_style(styles: &StyleInput) -> Result<Style, String> {
    let mut style = Style::default();
    for (key, raw_value) in styles {
        let value = raw_value.trim();
        let keyword = value.to_ascii_lowercase();
        match key.as_str() {
            "display" => {
                style.display = match keyword.as_str() {
                    "flex" => Display::Flex,
                    "grid" => Display::Grid,
                    "block" => Display::Block,
                    "none" => Display::None,
                    _ => return Err(invalid(key, value)),
                };
            }
            "direction" => {
                style.direction = match keyword.as_str() {
                    "ltr" => Direction::Ltr,
                    "rtl" => Direction::Rtl,
                    _ => return Err(invalid(key, value)),
                };
            }
            "boxSizing" => {
                style.box_sizing = match keyword.as_str() {
                    "border-box" => BoxSizing::BorderBox,
                    "content-box" => BoxSizing::ContentBox,
                    _ => return Err(invalid(key, value)),
                };
            }
            "width" => {
                style.size.width = parse_dimension(value).ok_or_else(|| invalid(key, value))?
            }
            "height" => {
                style.size.height = parse_dimension(value).ok_or_else(|| invalid(key, value))?
            }
            "minWidth" => {
                style.min_size.width = parse_length_percentage_auto(value, false)
                    .ok_or_else(|| invalid(key, value))?;
            }
            "minHeight" => {
                style.min_size.height = parse_length_percentage_auto(value, false)
                    .ok_or_else(|| invalid(key, value))?;
            }
            "maxWidth" => {
                style.max_size.width = parse_length_percentage_auto(value, false)
                    .ok_or_else(|| invalid(key, value))?;
            }
            "maxHeight" => {
                style.max_size.height = parse_length_percentage_auto(value, false)
                    .ok_or_else(|| invalid(key, value))?;
            }
            "flex" => match keyword.as_str() {
                "none" => {
                    style.flex_grow = 0.0;
                    style.flex_shrink = 0.0;
                    style.flex_basis = Dimension::auto();
                }
                "auto" => {
                    style.flex_grow = 1.0;
                    style.flex_shrink = 1.0;
                    style.flex_basis = Dimension::auto();
                }
                _ => {
                    let number = parse_finite_number(value)
                        .filter(|n| *n >= 0.0)
                        .ok_or_else(|| invalid(key, value))?;
                    style.flex_grow = number;
                    style.flex_shrink = 1.0;
                    style.flex_basis = Dimension::percent(0.0);
                }
            },
            "flexGrow" => {
                style.flex_grow = parse_finite_number(value)
                    .filter(|n| *n >= 0.0)
                    .ok_or_else(|| invalid(key, value))?;
            }
            "flexShrink" => {
                style.flex_shrink = parse_finite_number(value)
                    .filter(|n| *n >= 0.0)
                    .ok_or_else(|| invalid(key, value))?;
            }
            "flexBasis" => {
                style.flex_basis = parse_dimension(value).ok_or_else(|| invalid(key, value))?
            }
            "flexDirection" => {
                style.flex_direction = match keyword.as_str() {
                    "row" => FlexDirection::Row,
                    "row-reverse" => FlexDirection::RowReverse,
                    "column" => FlexDirection::Column,
                    "column-reverse" => FlexDirection::ColumnReverse,
                    _ => return Err(invalid(key, value)),
                };
            }
            "flexWrap" => {
                style.flex_wrap = match keyword.as_str() {
                    "nowrap" => FlexWrap::NoWrap,
                    "wrap" => FlexWrap::Wrap,
                    "wrap-reverse" => FlexWrap::WrapReverse,
                    "balance" => FlexWrap::Balance,
                    "balance-reverse" => FlexWrap::BalanceReverse,
                    _ => return Err(invalid(key, value)),
                };
            }
            "justifyContent" => {
                style.justify_content =
                    justify_content(keyword.as_str()).ok_or_else(|| invalid(key, value))?;
            }
            "alignItems" => {
                style.align_items =
                    align_items(keyword.as_str()).ok_or_else(|| invalid(key, value))?
            }
            "alignSelf" => {
                style.align_self = if keyword == "auto" {
                    None
                } else {
                    Some(align_items(keyword.as_str()).ok_or_else(|| invalid(key, value))?)
                };
            }
            "alignContent" => {
                style.align_content =
                    align_content(keyword.as_str()).ok_or_else(|| invalid(key, value))?;
            }
            "gap" => {
                let dimension =
                    parse_length_percentage(value).ok_or_else(|| invalid(key, value))?;
                style.gap = Size {
                    width: dimension,
                    height: dimension,
                };
            }
            "rowGap" => {
                style.gap.height =
                    parse_length_percentage(value).ok_or_else(|| invalid(key, value))?
            }
            "columnGap" => {
                style.gap.width =
                    parse_length_percentage(value).ok_or_else(|| invalid(key, value))?
            }
            "margin" => {
                style.margin =
                    parse_rect_shorthand(value, |v| parse_length_percentage_auto(v, true))
                        .ok_or_else(|| invalid(key, value))?;
            }
            "marginTop" | "marginRight" | "marginBottom" | "marginLeft" => {
                let side = &key[6..];
                let side = match side.to_ascii_lowercase().as_str() {
                    "top" => "top",
                    "right" => "right",
                    "bottom" => "bottom",
                    "left" => "left",
                    _ => unreachable!(),
                };
                let parsed =
                    parse_length_percentage_auto(value, true).ok_or_else(|| invalid(key, value))?;
                set_rect(&mut style.margin, side, parsed);
            }
            "padding" => {
                style.padding = parse_rect_shorthand(value, parse_length_percentage)
                    .ok_or_else(|| invalid(key, value))?;
            }
            "paddingTop" | "paddingRight" | "paddingBottom" | "paddingLeft" => {
                let side = match &key[7..].to_ascii_lowercase()[..] {
                    "top" => "top",
                    "right" => "right",
                    "bottom" => "bottom",
                    "left" => "left",
                    _ => unreachable!(),
                };
                let parsed = parse_length_percentage(value).ok_or_else(|| invalid(key, value))?;
                set_rect(&mut style.padding, side, parsed);
            }
            "position" => {
                style.position = match keyword.as_str() {
                    "static" => Position::Static,
                    "relative" => Position::Relative,
                    "absolute" => Position::Absolute,
                    "fixed" => Position::Fixed,
                    "sticky" => Position::Sticky,
                    _ => return Err(invalid(key, value)),
                };
            }
            "top" | "right" | "bottom" | "left" => {
                let parsed =
                    parse_length_percentage_auto(value, true).ok_or_else(|| invalid(key, value))?;
                set_rect(&mut style.inset, key, parsed);
            }
            "aspectRatio" => {
                style.aspect_ratio = Some(
                    parse_finite_number(value)
                        .filter(|n| *n > 0.0)
                        .ok_or_else(|| invalid(key, value))?,
                );
            }
            // Text/image properties are consumed by intrinsic measurement;
            // paint-only properties are consumed by the host-view layer.
            "fontSize" | "fontWeight" | "fontFamily" | "lineHeight" | "text" | "src" | "color"
            | "fontStyle" | "textAlign" | "textDecoration" | "letterSpacing" | "borderWidth"
            | "borderColor" | "borderRadius" | "borderStyle" | "backgroundColor" | "opacity"
            | "visibility" => {}
            unknown => return Err(format!("Unknown style property: {unknown}")),
        }
    }
    Ok(style)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn styles(pairs: &[(&str, &str)]) -> StyleInput {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect()
    }

    #[test]
    fn maps_sizing_fields_to_their_taffy_types() {
        let style = to_taffy_style(&styles(&[
            ("width", "50%"),
            ("minWidth", "10px"),
            ("maxHeight", "auto"),
            ("padding", "2px 4px"),
            ("marginLeft", "-3px"),
            ("left", "25%"),
        ]))
        .unwrap();
        assert_eq!(
            style.size.width.expand(),
            taffy::style::ExpandedDimension::Percent(0.5)
        );
        assert_eq!(
            style.min_size.width.expand(),
            taffy::style::ExpandedLengthPercentageAuto::Length(10.0)
        );
        assert_eq!(
            style.padding.left.expand(),
            taffy::style::ExpandedLengthPercentage::Length(4.0)
        );
        assert_eq!(
            style.margin.left.expand(),
            taffy::style::ExpandedLengthPercentageAuto::Length(-3.0)
        );
        assert_eq!(
            style.inset.left.expand(),
            taffy::style::ExpandedLengthPercentageAuto::Percent(0.25)
        );
    }

    #[test]
    fn maps_flex_alignment_and_rtl() {
        let style = to_taffy_style(&styles(&[
            ("flexDirection", "row-reverse"),
            ("flexWrap", "wrap"),
            ("flexGrow", "2"),
            ("flexShrink", "0"),
            ("justifyContent", "space-between"),
            ("alignItems", "center"),
            ("alignSelf", "auto"),
            ("direction", "rtl"),
        ]))
        .unwrap();
        assert_eq!(style.flex_direction, FlexDirection::RowReverse);
        assert_eq!(style.flex_wrap, FlexWrap::Wrap);
        assert_eq!(style.flex_grow, 2.0);
        assert_eq!(style.flex_shrink, 0.0);
        assert_eq!(style.direction, Direction::Rtl);
        assert_eq!(style.align_self, None);
    }

    #[test]
    fn position_offsets_do_not_change_static_positioning() {
        let style = to_taffy_style(&styles(&[("top", "4px")])).unwrap();
        assert_eq!(style.position, Position::Static);
        assert_eq!(
            style.inset.top.expand(),
            taffy::style::ExpandedLengthPercentageAuto::Length(4.0)
        );
    }

    #[test]
    fn rejects_bad_and_non_finite_values() {
        for input in [
            styles(&[("width", "-1px")]),
            styles(&[("padding", "-1px")]),
            styles(&[("flexGrow", "NaN")]),
            styles(&[("bogus", "1px")]),
        ] {
            assert!(to_taffy_style(&input).is_err());
        }
    }

    #[test]
    fn parses_four_value_margin_and_padding_shorthands() {
        let style = to_taffy_style(&styles(&[
            ("margin", "1px 2px 3px 4px"),
            ("padding", "5px 6px 7px 8px"),
        ]))
        .unwrap();
        assert_eq!(
            style.margin.left.expand(),
            taffy::style::ExpandedLengthPercentageAuto::Length(4.0)
        );
        assert_eq!(
            style.margin.bottom.expand(),
            taffy::style::ExpandedLengthPercentageAuto::Length(3.0)
        );
        assert_eq!(
            style.padding.left.expand(),
            taffy::style::ExpandedLengthPercentage::Length(8.0)
        );
        assert_eq!(
            style.padding.bottom.expand(),
            taffy::style::ExpandedLengthPercentage::Length(7.0)
        );
    }

    #[test]
    fn style_input_is_hash_map_compatible() {
        let input: StyleInput = HashMap::from([("width".to_owned(), "10".to_owned())]);
        assert!(to_taffy_style(&input).is_ok());
    }
}
