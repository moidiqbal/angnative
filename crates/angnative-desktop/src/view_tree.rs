/**
 * view_tree.rs — retained local view tree for the desktop shell.
 *
 * The host (OpEmitter) emits an ordered stream of `PlatformOp`s. This module
 * consumes that stream into a local, inspectable tree of view nodes carrying
 * geometry (x/y/width/height), text and props. Rendering is still done as
 * rectangles, but geometry now comes from this retained tree via layout
 * `SetFrame` ops instead of being discarded.
 */

use std::collections::HashMap;

use angnative_host_view::ops::{PlatformOp, ViewProps};

/// A single retained native view node.
#[derive(Debug, Clone, PartialEq)]
pub struct LocalViewNode {
    pub id: u64,
    pub view_type: String,
    pub parent: Option<u64>,
    pub children: Vec<u64>,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub props: ViewProps,
    pub text: Option<String>,
}

impl LocalViewNode {
    fn new(id: u64, view_type: String) -> Self {
        Self {
            id,
            view_type,
            parent: None,
            children: Vec::new(),
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            props: ViewProps::new(),
            text: None,
        }
    }
}

/// Retained tree of native views, updated incrementally by `PlatformOp`s.
#[derive(Debug, Clone, Default)]
pub struct LocalViewTree {
    nodes: HashMap<u64, LocalViewNode>,
    /// Stable creation order, used for deterministic rendering/traversal.
    order: Vec<u64>,
}

impl LocalViewTree {
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply a batch of ordered platform ops to the retained tree.
    ///
    /// This is intentionally tolerant: ops for unknown ids create placeholder
    /// nodes so that a frame or prop arriving before a Create is not lost.
    pub fn apply_ops(&mut self, ops: &[PlatformOp]) {
        for op in ops {
            match op {
                PlatformOp::Create { node_id, view_type } => {
                    if !self.nodes.contains_key(node_id) {
                        self.nodes.insert(
                            *node_id,
                            LocalViewNode::new(*node_id, view_type.clone()),
                        );
                        self.order.push(*node_id);
                    } else {
                        // Recreated view: reset its visual state but keep order.
                        let node = self.nodes.get_mut(node_id).expect("checked above");
                        node.view_type = view_type.clone();
                        node.props = ViewProps::new();
                        node.text = None;
                    }
                }
                PlatformOp::SetFrame {
                    node_id,
                    x,
                    y,
                    width,
                    height,
                } => {
                    let node = self.node_mut_or_create(*node_id);
                    node.x = *x;
                    node.y = *y;
                    node.width = *width;
                    node.height = *height;
                }
                PlatformOp::SetProps { node_id, props } => {
                    let node = self.node_mut_or_create(*node_id);
                    for (key, value) in &props.styles {
                        node.props.styles.insert(key.clone(), value.clone());
                    }
                    for (key, value) in &props.properties {
                        node.props.properties.insert(key.clone(), value.clone());
                    }
                    if let Some(text) = &props.text {
                        node.text = Some(text.clone());
                        node.props.text = Some(text.clone());
                    }
                }
                PlatformOp::SetText { node_id, text } => {
                    let node = self.node_mut_or_create(*node_id);
                    node.text = Some(text.clone());
                    node.props.text = Some(text.clone());
                }
                PlatformOp::Append { parent_id, child_id } => {
                    let index = self
                        .nodes
                        .get(parent_id)
                        .map(|p| p.children.len())
                        .unwrap_or(0);
                    self.insert_child(*parent_id, *child_id, index);
                }
                PlatformOp::Insert {
                    parent_id,
                    child_id,
                    index,
                } => {
                    self.insert_child(*parent_id, *child_id, *index);
                }
                PlatformOp::Remove { parent_id, child_id } => {
                    if let Some(parent) = self.nodes.get_mut(parent_id) {
                        parent.children.retain(|id| id != child_id);
                    }
                    if let Some(child) = self.nodes.get_mut(child_id) {
                        if child.parent == Some(*parent_id) {
                            child.parent = None;
                        }
                    }
                }
                PlatformOp::Destroy { node_id } => {
                    if let Some(node) = self.nodes.remove(node_id) {
                        if let Some(parent_id) = node.parent {
                            if let Some(parent) = self.nodes.get_mut(&parent_id) {
                                parent.children.retain(|id| id != node_id);
                            }
                        }
                        self.order.retain(|id| id != node_id);
                    }
                }
            }
        }
    }

    fn node_mut_or_create(&mut self, id: u64) -> &mut LocalViewNode {
        if !self.nodes.contains_key(&id) {
            self.nodes
                .insert(id, LocalViewNode::new(id, "View".to_string()));
            self.order.push(id);
        }
        self.nodes.get_mut(&id).expect("inserted above")
    }

    fn insert_child(&mut self, parent_id: u64, child_id: u64, index: usize) {
        // Ensure both nodes exist before wiring them together.
        self.node_mut_or_create(parent_id);
        self.node_mut_or_create(child_id);

        // Detach from the previous parent first (re-parenting).
        let old_parent = self.nodes.get(&child_id).and_then(|n| n.parent);
        if let Some(old) = old_parent {
            if let Some(node) = self.nodes.get_mut(&old) {
                node.children.retain(|id| id != &child_id);
            }
        }

        let at = self
            .nodes
            .get(&parent_id)
            .map(|p| p.children.len().min(index))
            .unwrap_or(0);
        if let Some(parent) = self.nodes.get_mut(&parent_id) {
            parent.children.insert(at, child_id);
        }
        if let Some(child) = self.nodes.get_mut(&child_id) {
            child.parent = Some(parent_id);
        }
    }

    /// Look up a node by id.
    pub fn node(&self, id: u64) -> Option<&LocalViewNode> {
        self.nodes.get(&id)
    }

    /// Iterate nodes in stable creation order.
    pub fn nodes(&self) -> impl Iterator<Item = &LocalViewNode> {
        self.order.iter().filter_map(move |id| self.nodes.get(id))
    }

    /// Number of retained nodes.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn create_insert_frame_text_and_props_build_retained_tree() {
        let mut tree = LocalViewTree::new();
        tree.apply_ops(&[
            PlatformOp::Create {
                node_id: 1,
                view_type: "View".to_string(),
            },
            PlatformOp::Create {
                node_id: 2,
                view_type: "Text".to_string(),
            },
            PlatformOp::Insert {
                parent_id: 1,
                child_id: 2,
                index: 0,
            },
            PlatformOp::SetFrame {
                node_id: 2,
                x: 24.0,
                y: 48.0,
                width: 120.0,
                height: 20.0,
            },
            PlatformOp::SetText {
                node_id: 2,
                text: "AngNative Desktop".to_string(),
            },
            PlatformOp::SetProps {
                node_id: 2,
                props: ViewProps {
                    text: None,
                    styles: HashMap::from([("fontSize".to_string(), "20px".to_string())]),
                    properties: HashMap::from([("disabled".to_string(), json!(true))]),
                },
            },
        ]);

        assert_eq!(tree.len(), 2);
        let root = tree.node(1).expect("root");
        assert_eq!(root.view_type, "View");
        assert_eq!(root.children, vec![2]);

        let text = tree.node(2).expect("text");
        assert_eq!(text.parent, Some(1));
        assert_eq!((text.x, text.y, text.width, text.height), (24.0, 48.0, 120.0, 20.0));
        assert_eq!(text.text.as_deref(), Some("AngNative Desktop"));
        assert_eq!(
            text.props.styles.get("fontSize").map(String::as_str),
            Some("20px")
        );
        assert_eq!(text.props.properties.get("disabled"), Some(&json!(true)));

        // Stable order is preserved for rendering.
        let ids: Vec<u64> = tree.nodes().map(|n| n.id).collect();
        assert_eq!(ids, vec![1, 2]);
    }

    #[test]
    fn insert_honors_index_and_reparenting_detaches_old_parent() {
        let mut tree = LocalViewTree::new();
        tree.apply_ops(&[
            PlatformOp::Create {
                node_id: 1,
                view_type: "View".to_string(),
            },
            PlatformOp::Create {
                node_id: 2,
                view_type: "View".to_string(),
            },
            PlatformOp::Create {
                node_id: 3,
                view_type: "View".to_string(),
            },
            PlatformOp::Insert {
                parent_id: 1,
                child_id: 2,
                index: 0,
            },
            PlatformOp::Insert {
                parent_id: 1,
                child_id: 3,
                index: 0,
            },
        ]);
        assert_eq!(tree.node(1).unwrap().children, vec![3, 2]);

        // Reparent 2 under 3: it must be removed from root's children.
        tree.apply_ops(&[PlatformOp::Append {
            parent_id: 3,
            child_id: 2,
        }]);
        assert_eq!(tree.node(1).unwrap().children, vec![3]);
        assert_eq!(tree.node(3).unwrap().children, vec![2]);
        assert_eq!(tree.node(2).unwrap().parent, Some(3));
    }

    #[test]
    fn remove_and_destroy_prune_tree() {
        let mut tree = LocalViewTree::new();
        tree.apply_ops(&[
            PlatformOp::Create {
                node_id: 1,
                view_type: "View".to_string(),
            },
            PlatformOp::Create {
                node_id: 2,
                view_type: "View".to_string(),
            },
            PlatformOp::Append {
                parent_id: 1,
                child_id: 2,
            },
        ]);
        assert_eq!(tree.node(2).unwrap().parent, Some(1));

        tree.apply_ops(&[PlatformOp::Remove {
            parent_id: 1,
            child_id: 2,
        }]);
        assert!(tree.node(1).unwrap().children.is_empty());
        assert_eq!(tree.node(2).unwrap().parent, None);

        tree.apply_ops(&[PlatformOp::Destroy { node_id: 2 }]);
        assert!(tree.node(2).is_none());
        assert_eq!(tree.len(), 1);
    }

    #[test]
    fn frame_before_create_is_not_lost() {
        let mut tree = LocalViewTree::new();
        tree.apply_ops(&[PlatformOp::SetFrame {
            node_id: 9,
            x: 1.0,
            y: 2.0,
            width: 3.0,
            height: 4.0,
        }]);
        let node = tree.node(9).expect("placeholder node created");
        assert_eq!((node.x, node.y, node.width, node.height), (1.0, 2.0, 3.0, 4.0));
    }
}
