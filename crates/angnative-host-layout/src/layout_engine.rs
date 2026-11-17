//! AngNative tree validation and layout orchestration.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use taffy::geometry::Size as TaffySize;

use crate::styles::{to_taffy_style, StyleInput};
use crate::taffy_adapter::{AdapterNode, MeasureContext, TaffyAdapter};
use crate::text_measurer::{
    ImageMeasurer, IntrinsicContent, NaiveMeasurer, TextMeasurer, TextSize, ZeroImageMeasurer,
};

/// Computed geometry for one stable AngNative node identifier.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutNodeRect {
    pub node_id: u64,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Result of one layout pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutResult {
    /// Geometry per reachable node, sorted by stable AngNative node id.
    pub nodes: Vec<LayoutNodeRect>,
    /// The root node's measured border-box size.
    pub content_size: TaffySize<f32>,
}

/// Input node: stable id, style map and ordered child ids.
///
/// `node_id` is retained for source compatibility and must equal the map key.
pub struct LayoutNodeInput {
    pub node_id: u64,
    pub styles: StyleInput,
    pub children: Vec<u64>,
}

/// Layout engine with explicit text and image measurement hosts.
pub struct LayoutEngine<M: TextMeasurer = NaiveMeasurer, I: ImageMeasurer = ZeroImageMeasurer> {
    text_measurer: M,
    image_measurer: I,
    adapter: TaffyAdapter,
}

impl<M: TextMeasurer> LayoutEngine<M, ZeroImageMeasurer> {
    /// Construct an engine preserving the existing text-measurer contract.
    pub fn new(text_measurer: M) -> Self {
        Self {
            text_measurer,
            image_measurer: ZeroImageMeasurer,
            adapter: TaffyAdapter::default(),
        }
    }

    /// Add a host image measurement callback.
    pub fn with_image_measurer<I: ImageMeasurer>(self, image_measurer: I) -> LayoutEngine<M, I> {
        LayoutEngine {
            text_measurer: self.text_measurer,
            image_measurer,
            adapter: self.adapter,
        }
    }
}

impl<M: TextMeasurer, I: ImageMeasurer> LayoutEngine<M, I> {
    /// Run a layout pass without intrinsic content metadata.
    pub fn layout(
        &mut self,
        root_id: u64,
        nodes: &HashMap<u64, LayoutNodeInput>,
        available_width: f32,
        available_height: f32,
    ) -> Result<LayoutResult, String> {
        self.layout_with_intrinsic_content(
            root_id,
            nodes,
            available_width,
            available_height,
            &HashMap::new(),
        )
    }

    /// Run layout with host-provided text or image intrinsic-content requests.
    ///
    /// The `intrinsic_content` map is keyed by the same stable IDs used by
    /// `LayoutNodeInput`. Missing image callbacks return a zero intrinsic size.
    pub fn layout_with_intrinsic_content(
        &mut self,
        root_id: u64,
        nodes: &HashMap<u64, LayoutNodeInput>,
        available_width: f32,
        available_height: f32,
        intrinsic_content: &HashMap<u64, IntrinsicContent>,
    ) -> Result<LayoutResult, String> {
        validate_available_space(available_width, "available_width")?;
        validate_available_space(available_height, "available_height")?;

        let order = reachable_postorder(root_id, nodes)?;
        let reachable = order.iter().copied().collect::<HashSet<_>>();
        for node_id in intrinsic_content.keys() {
            if !reachable.contains(node_id) {
                return Err(format!(
                    "Intrinsic content references unreachable node {node_id}"
                ));
            }
        }

        let mut adapter_nodes = Vec::with_capacity(order.len());
        for node_id in order {
            let input = nodes.get(&node_id).expect("validated reachable node");
            let style = to_taffy_style(&input.styles)?;
            let font_size = parse_font_size(&input.styles)?;
            let font_weight = input
                .styles
                .get("fontWeight")
                .cloned()
                .unwrap_or_else(|| "normal".to_owned());
            adapter_nodes.push(AdapterNode {
                angnative_id: node_id,
                style,
                children: input.children.clone(),
                measure: MeasureContext {
                    content: intrinsic_content.get(&node_id).cloned(),
                    font_size,
                    font_weight,
                },
            });
        }

        let text_measurer = &self.text_measurer;
        let image_measurer = &self.image_measurer;
        let rects = self.adapter.compute(
            &adapter_nodes,
            root_id,
            available_width,
            available_height,
            |context, width, height| match &context.content {
                Some(IntrinsicContent::Text(text)) => {
                    let measured = text_measurer.measure_text(
                        text,
                        context.font_size,
                        &context.font_weight,
                        width,
                    );
                    safe_size(measured)
                }
                Some(IntrinsicContent::Image(source)) => {
                    safe_size(image_measurer.measure_image(source, width, height))
                }
                None => TaffySize::ZERO,
            },
        )?;

        let mut output = rects
            .into_iter()
            .map(|rect| LayoutNodeRect {
                node_id: rect.node_id,
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height,
            })
            .collect::<Vec<_>>();
        output.sort_by_key(|rect| rect.node_id);
        let root = output
            .iter()
            .find(|rect| rect.node_id == root_id)
            .ok_or_else(|| format!("Layout result omitted root node {root_id}"))?;
        let content_size = TaffySize {
            width: root.width,
            height: root.height,
        };

        Ok(LayoutResult {
            nodes: output,
            content_size,
        })
    }
}

impl Default for LayoutEngine<NaiveMeasurer, ZeroImageMeasurer> {
    fn default() -> Self {
        Self::new(NaiveMeasurer)
    }
}

fn safe_size(size: TextSize) -> TaffySize<f32> {
    TaffySize {
        width: if size.width.is_finite() {
            size.width.max(0.0)
        } else {
            0.0
        },
        height: if size.height.is_finite() {
            size.height.max(0.0)
        } else {
            0.0
        },
    }
}

fn validate_available_space(value: f32, label: &str) -> Result<(), String> {
    if value.is_finite() && value >= 0.0 {
        Ok(())
    } else {
        Err(format!(
            "{label} must be finite and non-negative, got {value}"
        ))
    }
}

fn parse_font_size(styles: &StyleInput) -> Result<f32, String> {
    let Some(value) = styles.get("fontSize") else {
        return Ok(16.0);
    };
    let value = value.trim().strip_suffix("px").unwrap_or(value.trim());
    let size = value
        .parse::<f32>()
        .map_err(|_| format!("Invalid fontSize: {value}"))?;
    if size.is_finite() && size > 0.0 {
        Ok(size)
    } else {
        Err(format!("Invalid fontSize: {value}"))
    }
}

fn reachable_postorder(
    root_id: u64,
    nodes: &HashMap<u64, LayoutNodeInput>,
) -> Result<Vec<u64>, String> {
    if !nodes.contains_key(&root_id) {
        return Err(format!("Root node {root_id} not found in layout input"));
    }
    let mut parents = HashMap::<u64, u64>::new();
    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    let mut order = Vec::new();

    fn visit(
        id: u64,
        nodes: &HashMap<u64, LayoutNodeInput>,
        parents: &mut HashMap<u64, u64>,
        visiting: &mut HashSet<u64>,
        visited: &mut HashSet<u64>,
        order: &mut Vec<u64>,
    ) -> Result<(), String> {
        if visiting.contains(&id) {
            return Err(format!("Cycle detected at node {id}"));
        }
        if visited.contains(&id) {
            return Ok(());
        }
        let input = nodes
            .get(&id)
            .ok_or_else(|| format!("Child node {id} is missing"))?;
        if input.node_id != id {
            return Err(format!(
                "Node map key {id} does not match input node_id {}",
                input.node_id
            ));
        }
        visiting.insert(id);
        let mut local_children = HashSet::new();
        for child_id in &input.children {
            if !local_children.insert(*child_id) {
                return Err(format!("Node {id} lists child {child_id} more than once"));
            }
            if let Some(previous_parent) = parents.insert(*child_id, id) {
                return Err(format!(
                    "Node {child_id} has multiple parents ({previous_parent} and {id})"
                ));
            }
            visit(*child_id, nodes, parents, visiting, visited, order)?;
        }
        visiting.remove(&id);
        visited.insert(id);
        order.push(id);
        Ok(())
    }

    visit(
        root_id,
        nodes,
        &mut parents,
        &mut visiting,
        &mut visited,
        &mut order,
    )?;
    if let Some(parent) = parents.get(&root_id) {
        return Err(format!(
            "Root node {root_id} cannot be a child of node {parent}"
        ));
    }
    Ok(order)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn style(pairs: &[(&str, &str)]) -> StyleInput {
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect()
    }

    fn node(id: u64, styles: StyleInput, children: &[u64]) -> LayoutNodeInput {
        LayoutNodeInput {
            node_id: id,
            styles,
            children: children.to_vec(),
        }
    }

    fn get(result: &LayoutResult, id: u64) -> &LayoutNodeRect {
        result.nodes.iter().find(|rect| rect.node_id == id).unwrap()
    }

    fn run(
        engine: &mut LayoutEngine,
        nodes: &HashMap<u64, LayoutNodeInput>,
        width: f32,
        height: f32,
    ) -> LayoutResult {
        engine.layout(1, nodes, width, height).unwrap()
    }

    #[test]
    fn nested_flex_layout_preserves_order_and_parent_relative_frames() {
        let mut nodes = HashMap::new();
        nodes.insert(
            1,
            node(
                1,
                style(&[
                    ("width", "300px"),
                    ("height", "100px"),
                    ("flexDirection", "row"),
                ]),
                &[2, 3],
            ),
        );
        nodes.insert(
            2,
            node(
                2,
                style(&[
                    ("width", "100px"),
                    ("height", "80px"),
                    ("flexDirection", "column"),
                ]),
                &[4],
            ),
        );
        nodes.insert(
            3,
            node(3, style(&[("width", "100px"), ("height", "80px")]), &[]),
        );
        nodes.insert(
            4,
            node(4, style(&[("width", "20px"), ("height", "20px")]), &[]),
        );
        let result = run(&mut LayoutEngine::default(), &nodes, 300.0, 100.0);
        assert!(get(&result, 3).x > get(&result, 2).x);
        assert_eq!(get(&result, 4).x, 0.0);
    }

    #[test]
    fn percentage_auto_and_min_max_sizing_work() {
        let mut nodes = HashMap::new();
        nodes.insert(
            1,
            node(1, style(&[("width", "200px"), ("height", "100px")]), &[2]),
        );
        nodes.insert(
            2,
            node(
                2,
                style(&[
                    ("width", "50%"),
                    ("height", "auto"),
                    ("minHeight", "30px"),
                    ("maxWidth", "120px"),
                ]),
                &[],
            ),
        );
        let result = run(&mut LayoutEngine::default(), &nodes, 200.0, 100.0);
        assert!((get(&result, 2).width - 100.0).abs() <= 1.0);
        assert!(get(&result, 2).height >= 30.0);
    }

    #[test]
    fn padding_margin_and_gaps_affect_child_frames() {
        let mut nodes = HashMap::new();
        nodes.insert(
            1,
            node(
                1,
                style(&[
                    ("width", "300px"),
                    ("height", "100px"),
                    ("flexDirection", "row"),
                    ("gap", "10px"),
                    ("padding", "5px"),
                ]),
                &[2, 3],
            ),
        );
        nodes.insert(
            2,
            node(
                2,
                style(&[("width", "50px"), ("height", "20px"), ("marginLeft", "3px")]),
                &[],
            ),
        );
        nodes.insert(
            3,
            node(3, style(&[("width", "50px"), ("height", "20px")]), &[]),
        );
        let result = run(&mut LayoutEngine::default(), &nodes, 300.0, 100.0);
        assert!((get(&result, 2).x - 8.0).abs() <= 1.0);
        assert!(
            (get(&result, 3).x - (get(&result, 2).x + get(&result, 2).width + 10.0)).abs() <= 1.0
        );
    }

    #[test]
    fn flex_grow_shrink_and_wrap_are_applied() {
        let mut grow_nodes = HashMap::new();
        grow_nodes.insert(
            1,
            node(
                1,
                style(&[
                    ("width", "300px"),
                    ("height", "60px"),
                    ("flexDirection", "row"),
                ]),
                &[2, 3],
            ),
        );
        grow_nodes.insert(
            2,
            node(2, style(&[("width", "50px"), ("flexGrow", "1")]), &[]),
        );
        grow_nodes.insert(
            3,
            node(3, style(&[("width", "50px"), ("flexGrow", "1")]), &[]),
        );
        let result = run(&mut LayoutEngine::default(), &grow_nodes, 300.0, 60.0);
        assert!((get(&result, 2).width - 150.0).abs() <= 1.0);

        let mut wrap_nodes = HashMap::new();
        wrap_nodes.insert(
            1,
            node(
                1,
                style(&[
                    ("width", "100px"),
                    ("height", "100px"),
                    ("flexDirection", "row"),
                    ("flexWrap", "wrap"),
                ]),
                &[2, 3],
            ),
        );
        wrap_nodes.insert(
            2,
            node(
                2,
                style(&[("width", "70px"), ("height", "20px"), ("flexShrink", "0")]),
                &[],
            ),
        );
        wrap_nodes.insert(
            3,
            node(
                3,
                style(&[("width", "70px"), ("height", "20px"), ("flexShrink", "0")]),
                &[],
            ),
        );
        let wrapped = run(&mut LayoutEngine::default(), &wrap_nodes, 100.0, 100.0);
        assert!(get(&wrapped, 3).y > get(&wrapped, 2).y);
    }

    #[test]
    fn absolute_positioning_uses_relative_containing_block() {
        let mut nodes = HashMap::new();
        nodes.insert(
            1,
            node(
                1,
                style(&[
                    ("width", "200px"),
                    ("height", "100px"),
                    ("position", "relative"),
                ]),
                &[2],
            ),
        );
        nodes.insert(
            2,
            node(
                2,
                style(&[
                    ("position", "absolute"),
                    ("left", "25px"),
                    ("top", "10px"),
                    ("width", "20px"),
                    ("height", "10px"),
                ]),
                &[],
            ),
        );
        let result = run(&mut LayoutEngine::default(), &nodes, 200.0, 100.0);
        assert_eq!(get(&result, 2).x, 25.0);
        assert_eq!(get(&result, 2).y, 10.0);
    }

    struct RecordingTextMeasurer(AtomicUsize);
    impl TextMeasurer for RecordingTextMeasurer {
        fn measure_text(&self, text: &str, size: f32, _: &str, width: Option<f32>) -> TextSize {
            self.0.fetch_add(1, Ordering::SeqCst);
            TextSize {
                width: width.unwrap_or(text.len() as f32 * size),
                height: size * 2.0,
            }
        }
    }

    struct RecordingImageMeasurer;
    impl ImageMeasurer for RecordingImageMeasurer {
        fn measure_image(&self, source: &str, width: Option<f32>, _: Option<f32>) -> TextSize {
            assert_eq!(source, "hero.png");
            TextSize {
                width: width.unwrap_or(80.0),
                height: 40.0,
            }
        }
    }

    #[test]
    fn text_and_image_measurement_callbacks_are_used() {
        let mut nodes = HashMap::new();
        nodes.insert(
            1,
            node(
                1,
                style(&[("fontSize", "18px"), ("fontWeight", "600")]),
                &[],
            ),
        );
        let text_content = HashMap::from([(1, IntrinsicContent::Text("measured".to_owned()))]);
        let text_measurer = RecordingTextMeasurer(AtomicUsize::new(0));
        let mut engine = LayoutEngine::new(text_measurer);
        let text = engine
            .layout_with_intrinsic_content(1, &nodes, 100.0, 100.0, &text_content)
            .unwrap();
        assert_eq!(get(&text, 1).height, 36.0);
        assert!(engine.text_measurer.0.load(Ordering::SeqCst) > 0);

        let image_content = HashMap::from([(1, IntrinsicContent::Image("hero.png".to_owned()))]);
        let mut image_engine = LayoutEngine::default().with_image_measurer(RecordingImageMeasurer);
        let image = image_engine
            .layout_with_intrinsic_content(1, &nodes, 100.0, 100.0, &image_content)
            .unwrap();
        assert_eq!(get(&image, 1).height, 40.0);
    }

    #[test]
    fn rtl_direction_reverses_row_visual_order() {
        let mut nodes = HashMap::new();
        nodes.insert(
            1,
            node(
                1,
                style(&[
                    ("width", "200px"),
                    ("height", "50px"),
                    ("flexDirection", "row"),
                    ("direction", "rtl"),
                ]),
                &[2, 3],
            ),
        );
        nodes.insert(
            2,
            node(2, style(&[("width", "40px"), ("height", "20px")]), &[]),
        );
        nodes.insert(
            3,
            node(3, style(&[("width", "40px"), ("height", "20px")]), &[]),
        );
        let result = run(&mut LayoutEngine::default(), &nodes, 200.0, 50.0);
        assert!(get(&result, 2).x > get(&result, 3).x);
    }

    #[test]
    fn same_engine_handles_insertion_removal_style_update_and_relayout() {
        let mut engine = LayoutEngine::default();
        let mut nodes = HashMap::new();
        nodes.insert(
            1,
            node(
                1,
                style(&[
                    ("width", "100px"),
                    ("height", "60px"),
                    ("flexDirection", "row"),
                ]),
                &[2],
            ),
        );
        nodes.insert(
            2,
            node(2, style(&[("width", "20px"), ("height", "20px")]), &[]),
        );
        assert_eq!(get(&run(&mut engine, &nodes, 100.0, 60.0), 2).width, 20.0);

        nodes.insert(
            1,
            node(
                1,
                style(&[
                    ("width", "100px"),
                    ("height", "60px"),
                    ("flexDirection", "row"),
                ]),
                &[2, 3],
            ),
        );
        nodes.insert(
            3,
            node(3, style(&[("width", "30px"), ("height", "20px")]), &[]),
        );
        assert_eq!(run(&mut engine, &nodes, 100.0, 60.0).nodes.len(), 3);

        nodes.insert(
            2,
            node(2, style(&[("width", "45px"), ("height", "20px")]), &[]),
        );
        assert_eq!(get(&run(&mut engine, &nodes, 100.0, 60.0), 2).width, 45.0);
        nodes.insert(
            1,
            node(1, style(&[("width", "100px"), ("height", "60px")]), &[3]),
        );
        assert_eq!(run(&mut engine, &nodes, 100.0, 60.0).nodes.len(), 2);
    }

    #[test]
    fn rejects_invalid_graphs_and_bad_constraints() {
        let mut engine = LayoutEngine::default();
        let mut cycle = HashMap::new();
        cycle.insert(1, node(1, StyleInput::default(), &[2]));
        cycle.insert(2, node(2, StyleInput::default(), &[1]));
        assert!(engine
            .layout(1, &cycle, 10.0, 10.0)
            .unwrap_err()
            .contains("Cycle"));
        assert!(engine.layout(1, &cycle, f32::NAN, 10.0).is_err());
    }

    #[test]
    fn zero_size_and_constrained_containers_are_stable() {
        let mut nodes = HashMap::new();
        nodes.insert(
            1,
            node(
                1,
                style(&[
                    ("width", "0px"),
                    ("height", "0px"),
                    ("minWidth", "0px"),
                    ("maxHeight", "0px"),
                ]),
                &[2],
            ),
        );
        nodes.insert(
            2,
            node(2, style(&[("width", "80px"), ("height", "40px")]), &[]),
        );
        let result = run(&mut LayoutEngine::default(), &nodes, 0.0, 0.0);
        assert_eq!(get(&result, 1).width, 0.0);
        assert_eq!(get(&result, 1).height, 0.0);
        assert!(get(&result, 2).width.is_finite());
    }
}
