//! Version boundary for Taffy 0.15.1.
//!
//! Keep Taffy types and tree operations in this module so an upstream upgrade
//! does not leak through the host layout algorithm.

use std::collections::HashMap;

use taffy::compute::compute_leaf_layout;
use taffy::prelude::{AvailableSpace, Size, Style};
use taffy::tree::{LayoutInput, LayoutOutput, NodeId, TaffyTree};

use crate::text_measurer::IntrinsicContent;

pub(crate) type TaffyStyle = Style;

#[derive(Clone)]
pub(crate) struct MeasureContext {
    pub content: Option<IntrinsicContent>,
    pub font_size: f32,
    pub font_weight: String,
}

pub(crate) struct AdapterNode {
    pub angnative_id: u64,
    pub style: TaffyStyle,
    pub children: Vec<u64>,
    pub measure: MeasureContext,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct AdapterRect {
    pub node_id: u64,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

pub(crate) struct TaffyAdapter {
    tree: TaffyTree<MeasureContext>,
    ids: HashMap<u64, NodeId>,
}

impl Default for TaffyAdapter {
    fn default() -> Self {
        Self {
            tree: TaffyTree::new(),
            ids: HashMap::new(),
        }
    }
}

impl TaffyAdapter {
    pub fn compute(
        &mut self,
        nodes: &[AdapterNode],
        root_id: u64,
        available_width: f32,
        available_height: f32,
        mut measure: impl FnMut(&MeasureContext, Option<f32>, Option<f32>) -> Size<f32>,
    ) -> Result<Vec<AdapterRect>, String> {
        // Each input snapshot is authoritative. Clearing first bounds memory,
        // drops obsolete node contexts and guarantees style/topology updates
        // invalidate every cached Taffy layout result.
        self.tree.clear();
        self.ids.clear();

        for node in nodes {
            let id = self
                .tree
                .new_leaf_with_context(node.style.clone(), node.measure.clone())
                .map_err(|error| format!("Taffy node creation failed: {error}"))?;
            if self.ids.insert(node.angnative_id, id).is_some() {
                return Err(format!("Duplicate AngNative node id {}", node.angnative_id));
            }
        }

        for node in nodes {
            let parent = self.node_id(node.angnative_id)?;
            let children = node
                .children
                .iter()
                .map(|id| self.node_id(*id))
                .collect::<Result<Vec<_>, _>>()?;
            self.tree
                .set_children(parent, &children)
                .map_err(|error| format!("Taffy child update failed: {error}"))?;
        }

        let root = self.node_id(root_id)?;
        self.tree
            .compute_layout_with_measure(
                root,
                Size {
                    width: AvailableSpace::Definite(available_width),
                    height: AvailableSpace::Definite(available_height),
                },
                |inputs: LayoutInput,
                 _node_id,
                 context: Option<&mut MeasureContext>,
                 style: &Style|
                 -> LayoutOutput {
                    compute_leaf_layout(
                        inputs,
                        style,
                        |_, _| 0.0,
                        |known, available| {
                            let measured = context
                                .as_deref()
                                .map(|context| {
                                    measure(
                                        context,
                                        known.width.or_else(|| available.width.into_option()),
                                        known.height.or_else(|| available.height.into_option()),
                                    )
                                })
                                .unwrap_or(Size::ZERO);
                            Size {
                                width: known.width.unwrap_or(measured.width),
                                height: known.height.unwrap_or(measured.height),
                            }
                        },
                    )
                },
            )
            .map_err(|error| format!("Taffy layout computation failed: {error}"))?;

        nodes
            .iter()
            .map(|node| {
                let id = self.node_id(node.angnative_id)?;
                let layout = self
                    .tree
                    .layout(id)
                    .map_err(|error| format!("Taffy layout lookup failed: {error}"))?;
                Ok(AdapterRect {
                    node_id: node.angnative_id,
                    x: layout.location.x,
                    y: layout.location.y,
                    width: layout.size.width,
                    height: layout.size.height,
                })
            })
            .collect()
    }

    fn node_id(&self, angnative_id: u64) -> Result<NodeId, String> {
        self.ids
            .get(&angnative_id)
            .copied()
            .ok_or_else(|| format!("Missing Taffy node mapping for AngNative node {angnative_id}"))
    }
}

pub(crate) mod style {
    pub use taffy::prelude::{
        AlignContent, AlignItems, BoxSizing, Dimension, Display, FlexDirection, FlexWrap,
        JustifyContent, LengthPercentage, LengthPercentageAuto, Position, Rect, Size, Style,
    };
    pub use taffy::style::Direction;
}
