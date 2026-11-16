use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique 64-bit identifier for every native view node.
pub type NodeId = u64;

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
    pub fn from_tag(tag: &str) -> Self {
        match tag.to_lowercase().as_str() {
            "native-view" | "view" | "div" => NodeType::View,
            "native-text" | "text" | "span" | "p" => NodeType::Text,
            "native-button" | "button" => NodeType::Button,
            "native-image" | "image" | "img" => NodeType::Image,
            "native-scroll" | "scroll" | "scrollview" => NodeType::ScrollView,
            "native-input" | "input" => NodeType::TextInput,
            custom => NodeType::Custom(custom.to_string()),
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
}
