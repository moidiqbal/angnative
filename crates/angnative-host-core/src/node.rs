/**
 * node.rs — Retained native element tree node for angnative-host-core.
 *
 * Mirrors the TypeScript `NativeElement` (`packages/core/src/native-element.ts`)
 * and the existing `crates/angnative-core/src/node.rs` `ViewNode`. This is the
 * Rust side of the TS↔Rust boundary: the TS fake backend and the Rust host
 * both manage trees of these nodes, and the conformance suite (Task R4)
 * asserts they produce identical state from identical command batches.
 */
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique 64-bit identifier for every native view node.
pub type NodeId = u64;

/// Node kind, mirroring `crates/angnative-core/src/node.rs::NodeType` and the
/// TypeScript `NativeNodeType`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeType {
    View,
    Text,
    Button,
    Image,
    ScrollView,
    TextInput,
    Custom(String),
}

impl NodeType {
    /// Parse a tag string into a `NodeType`, mirroring
    /// `NodeType::from_tag` in `crates/angnative-core/src/node.rs`.
    /// Legacy `native-*` aliases are accepted during migration; canonical
    /// tags use the `na-` prefix (identity D-9).
    pub fn from_tag(tag: &str) -> Self {
        match tag.to_lowercase().as_str() {
            "na-view" | "native-view" | "view" | "div" => NodeType::View,
            "na-text" | "native-text" | "text" | "span" | "p" => NodeType::Text,
            "na-button" | "native-button" | "button" => NodeType::Button,
            "na-image" | "native-image" | "image" | "img" => NodeType::Image,
            "na-scroll-view" | "native-scroll" | "scroll" | "scrollview" => NodeType::ScrollView,
            "na-text-input" | "native-input" | "input" => NodeType::TextInput,
            custom => NodeType::Custom(custom.to_string()),
        }
    }

    /// Parse a wire `nodeType` string from the command protocol. Tagged customs
    /// (`Custom:<tag>`) preserve their suffix; plain `Custom` remains an
    /// untagged generic host node.
    pub fn from_wire(name: &str) -> Option<Self> {
        match name {
            "View" => Some(NodeType::View),
            "Text" => Some(NodeType::Text),
            "Button" => Some(NodeType::Button),
            "Image" => Some(NodeType::Image),
            "ScrollView" => Some(NodeType::ScrollView),
            "TextInput" => Some(NodeType::TextInput),
            "Custom" => Some(NodeType::Custom(String::new())),
            tagged if tagged.starts_with("Custom:") => Some(NodeType::Custom(
                tagged["Custom:".len()..].to_string(),
            )),
            _ => None,
        }
    }

    /// Suffix of a tagged custom node, if any.
    pub fn custom_tag(&self) -> Option<&str> {
        match self {
            NodeType::Custom(tag) if !tag.is_empty() => Some(tag.as_str()),
            _ => None,
        }
    }

    /// Exact `nodeType` string used on the wire (matches the TypeScript
    /// `NativeNodeType` values, including tagged customs).
    pub fn wire_name(&self) -> String {
        match self {
            NodeType::Custom(tag) if tag.is_empty() => "Custom".to_string(),
            NodeType::Custom(tag) => format!("Custom:{tag}"),
            other => format!("{other:?}"),
        }
    }

    /// Canonical `na-` tag name for this node type.
    pub fn canonical_tag(&self) -> &'static str {
        match self {
            NodeType::View => "na-view",
            NodeType::Text => "na-text",
            NodeType::Button => "na-button",
            NodeType::Image => "na-image",
            NodeType::ScrollView => "na-scroll-view",
            NodeType::TextInput => "na-text-input",
            NodeType::Custom(_) => "na-custom",
        }
    }
}

/// Represents an active native node in the shadow tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewNode {
    pub id: NodeId,
    pub node_type: NodeType,
    pub parent_id: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub properties: HashMap<String, serde_json::Value>,
    pub styles: HashMap<String, String>,
    pub text_content: Option<String>,
}

impl ViewNode {
    pub fn new(id: NodeId, node_type: NodeType) -> Self {
        Self {
            id,
            node_type,
            parent_id: None,
            children: Vec::new(),
            properties: HashMap::new(),
            styles: HashMap::new(),
            text_content: None,
        }
    }

    pub fn set_style(&mut self, key: String, value: String) {
        self.styles.insert(key, value);
    }

    pub fn remove_style(&mut self, key: &str) {
        self.styles.remove(key);
    }

    pub fn set_property(&mut self, key: String, value: serde_json::Value) {
        self.properties.insert(key, value);
    }

    pub fn remove_property(&mut self, key: &str) {
        self.properties.remove(key);
    }

    pub fn add_child(&mut self, child_id: NodeId) {
        if !self.children.contains(&child_id) {
            self.children.push(child_id);
        }
    }

    pub fn remove_child_id(&mut self, child_id: NodeId) {
        self.children.retain(|&id| id != child_id);
    }

    pub fn insert_child_at(&mut self, index: usize, child_id: NodeId) {
        let idx = index.min(self.children.len());
        self.children.insert(idx, child_id);
    }

    pub fn set_text_content(&mut self, text: String) {
        self.text_content = Some(text);
    }
}

#[cfg(test)]
mod tests {
    use super::NodeType;

    #[test]
    fn tagged_custom_wire_names_preserve_suffix() {
        for tag in [
            "na-list",
            "na-switch",
            "na-activity-indicator",
            "my-widget",
        ] {
            let wire = format!("Custom:{tag}");
            let node_type = NodeType::from_wire(&wire).unwrap();
            assert_eq!(node_type, NodeType::Custom(tag.to_string()));
            assert_eq!(node_type.custom_tag(), Some(tag));
            assert_eq!(node_type.wire_name(), wire);
        }
    }

    #[test]
    fn plain_custom_is_untagged_and_unknown_types_are_rejected() {
        assert_eq!(
            NodeType::from_wire("Custom"),
            Some(NodeType::Custom(String::new()))
        );
        let plain = NodeType::from_wire("Custom").unwrap();
        assert_eq!(plain.custom_tag(), None);
        assert_eq!(plain.wire_name(), "Custom");

        assert_eq!(NodeType::from_wire("View"), Some(NodeType::View));
        assert_eq!(NodeType::from_wire("Bogus"), None);
    }
}
