/**
 * app.rs — Demo application for AngNative desktop.
 *
 * Builds a command batch representing a counter app (view, text, buttons)
 * and drives the desktop shell through command batches + layout.
 */

use crate::AngNativeDesktop;
use angnative_host_core::command::{Command, CommandBatch};
use angnative_host_layout::LayoutNodeInput;
use std::collections::HashMap;

/**
 * DemoApp — demo counter application running on AngNative desktop.
 *
 * Builds the following UI tree:
 *   na-view (root, flex column)
 *     na-text (title: "AngNative Desktop")
 *     na-text (subtitle: "Angular running natively — no WebView.")
 *     na-view (button row, flex row, gap 8px)
 *       na-button (−)
 *       na-button (reset)
 *       na-button (+)
 *     na-text (counter: "Count: 0")
 *
 * Commands are sent through the Rust host (MutationEngine → OpEmitter),
 * layout is computed by Taffy, and results are rendered via wgpu.
 */
pub struct DemoApp {
    desktop: AngNativeDesktop,
    counter_value: i32,
}

impl DemoApp {
    /// Construct the demo app on top of a real winit event loop.
    ///
    /// Creates the desktop window and GPU renderer, pushes the demo command
    /// batch through the Rust host, runs Taffy layout, and applies the emitted
    /// ops to the retained view tree.
    pub async fn new(
        event_loop: &winit::event_loop::EventLoop<()>,
    ) -> Result<Self, String> {
        let mut desktop = AngNativeDesktop::new(event_loop).await?;

        // Build the demo command batch and process it through the Rust host.
        let batch = Self::demo_batch();
        desktop.process_batch(&batch)?;

        // Build layout input from the command batch, run layout, then apply
        // the resulting SetFrame ops to the retained view tree.
        let nodes = Self::layout_inputs();
        let layout = desktop.run_layout(&nodes, 1, 800.0, 600.0)?;
        desktop.apply_layout(&layout)?;

        Ok(Self {
            desktop,
            counter_value: 0,
        })
    }

    /// Backwards-compatible alias for [`DemoApp::new`].
    pub async fn new_with_event_loop(
        event_loop: &winit::event_loop::EventLoop<()>,
    ) -> Result<Self, String> {
        Self::new(event_loop).await
    }

    /// Build the demo command batch (counter app UI).
    fn demo_batch() -> CommandBatch {
        let mut commands = Vec::new();

        // Root na-view
        commands.push(Command::CreateNode {
            id: 1,
            node_type: "View".to_string(),
        });
        commands.push(Command::SetStyle {
            id: 1,
            key: "flexDirection".to_string(),
            value: "column".to_string(),
        });
        commands.push(Command::SetStyle {
            id: 1,
            key: "padding".to_string(),
            value: "24px".to_string(),
        });
        commands.push(Command::SetStyle {
            id: 1,
            key: "gap".to_string(),
            value: "16px".to_string(),
        });

        // Title text
        commands.push(Command::CreateNode {
            id: 2,
            node_type: "Text".to_string(),
        });
        commands.push(Command::SetTextContent {
            id: 2,
            text: "AngNative Desktop".to_string(),
        });
        commands.push(Command::SetStyle {
            id: 2,
            key: "fontSize".to_string(),
            value: "20px".to_string(),
        });
        commands.push(Command::SetStyle {
            id: 2,
            key: "fontWeight".to_string(),
            value: "bold".to_string(),
        });
        commands.push(Command::InsertChild {
            parent_id: 1,
            child_id: 2,
            index: 0,
        });

        // Subtitle text
        commands.push(Command::CreateNode {
            id: 3,
            node_type: "Text".to_string(),
        });
        commands.push(Command::SetTextContent {
            id: 3,
            text: "Angular running natively — no WebView.".to_string(),
        });
        commands.push(Command::SetStyle {
            id: 3,
            key: "color".to_string(),
            value: "#666".to_string(),
        });
        commands.push(Command::SetStyle {
            id: 3,
            key: "fontSize".to_string(),
            value: "14px".to_string(),
        });
        commands.push(Command::InsertChild {
            parent_id: 1,
            child_id: 3,
            index: 1,
        });

        // Button container
        commands.push(Command::CreateNode {
            id: 4,
            node_type: "View".to_string(),
        });
        commands.push(Command::SetStyle {
            id: 4,
            key: "flexDirection".to_string(),
            value: "row".to_string(),
        });
        commands.push(Command::SetStyle {
            id: 4,
            key: "gap".to_string(),
            value: "8px".to_string(),
        });
        commands.push(Command::SetStyle {
            id: 4,
            key: "marginTop".to_string(),
            value: "16px".to_string(),
        });
        commands.push(Command::InsertChild {
            parent_id: 1,
            child_id: 4,
            index: 2,
        });

        // Decrement button
        commands.push(Command::CreateNode {
            id: 5,
            node_type: "Button".to_string(),
        });
        commands.push(Command::SetTextContent {
            id: 5,
            text: "−".to_string(),
        });
        commands.push(Command::SetStyle {
            id: 5,
            key: "padding".to_string(),
            value: "8px 16px".to_string(),
        });
        commands.push(Command::InsertChild {
            parent_id: 4,
            child_id: 5,
            index: 0,
        });

        // Reset button
        commands.push(Command::CreateNode {
            id: 6,
            node_type: "Button".to_string(),
        });
        commands.push(Command::SetTextContent {
            id: 6,
            text: "reset".to_string(),
        });
        commands.push(Command::SetStyle {
            id: 6,
            key: "padding".to_string(),
            value: "8px 16px".to_string(),
        });
        commands.push(Command::InsertChild {
            parent_id: 4,
            child_id: 6,
            index: 1,
        });

        // Increment button
        commands.push(Command::CreateNode {
            id: 7,
            node_type: "Button".to_string(),
        });
        commands.push(Command::SetTextContent {
            id: 7,
            text: "+".to_string(),
        });
        commands.push(Command::SetStyle {
            id: 7,
            key: "padding".to_string(),
            value: "8px 16px".to_string(),
        });
        commands.push(Command::InsertChild {
            parent_id: 4,
            child_id: 7,
            index: 2,
        });

        // Counter display
        commands.push(Command::CreateNode {
            id: 8,
            node_type: "Text".to_string(),
        });
        commands.push(Command::SetTextContent {
            id: 8,
            text: "Count: 0".to_string(),
        });
        commands.push(Command::SetStyle {
            id: 8,
            key: "fontSize".to_string(),
            value: "18px".to_string(),
        });
        commands.push(Command::SetStyle {
            id: 8,
            key: "fontWeight".to_string(),
            value: "bold".to_string(),
        });
        commands.push(Command::SetStyle {
            id: 8,
            key: "marginTop".to_string(),
            value: "16px".to_string(),
        });
        commands.push(Command::InsertChild {
            parent_id: 1,
            child_id: 8,
            index: 3,
        });

        CommandBatch {
            protocol_version: 1,
            frame_id: 0,
            timestamp_ms: 0,
            commands,
        }
    }

    /// Build layout input for all nodes.
    fn layout_inputs() -> HashMap<u64, LayoutNodeInput> {
        let mut nodes = HashMap::new();

        nodes.insert(
            1,
            LayoutNodeInput {
                node_id: 1,
                styles: HashMap::from([
                    ("flexDirection".to_string(), "column".to_string()),
                    ("padding".to_string(), "24px".to_string()),
                    ("gap".to_string(), "16px".to_string()),
                    ("width".to_string(), "800px".to_string()),
                    ("height".to_string(), "600px".to_string()),
                ]),
                children: vec![2, 3, 4, 8],
            },
        );
        nodes.insert(
            2,
            LayoutNodeInput {
                node_id: 2,
                styles: HashMap::from([
                    ("fontSize".to_string(), "20px".to_string()),
                ]),
                children: vec![],
            },
        );
        nodes.insert(
            3,
            LayoutNodeInput {
                node_id: 3,
                styles: HashMap::from([
                    ("color".to_string(), "#666".to_string()),
                    ("fontSize".to_string(), "14px".to_string()),
                ]),
                children: vec![],
            },
        );
        nodes.insert(
            4,
            LayoutNodeInput {
                node_id: 4,
                styles: HashMap::from([
                    ("flexDirection".to_string(), "row".to_string()),
                    ("gap".to_string(), "8px".to_string()),
                    ("marginTop".to_string(), "16px".to_string()),
                ]),
                children: vec![5, 6, 7],
            },
        );
        nodes.insert(
            5,
            LayoutNodeInput {
                node_id: 5,
                styles: HashMap::from([
                    ("padding".to_string(), "8px 16px".to_string()),
                ]),
                children: vec![],
            },
        );
        nodes.insert(
            6,
            LayoutNodeInput {
                node_id: 6,
                styles: HashMap::from([
                    ("padding".to_string(), "8px 16px".to_string()),
                ]),
                children: vec![],
            },
        );
        nodes.insert(
            7,
            LayoutNodeInput {
                node_id: 7,
                styles: HashMap::from([
                    ("padding".to_string(), "8px 16px".to_string()),
                ]),
                children: vec![],
            },
        );
        nodes.insert(
            8,
            LayoutNodeInput {
                node_id: 8,
                styles: HashMap::from([
                    ("fontSize".to_string(), "18px".to_string()),
                    ("fontWeight".to_string(), "bold".to_string()),
                    ("marginTop".to_string(), "16px".to_string()),
                ]),
                children: vec![],
            },
        );

        nodes
    }

    /// Get a mutable reference to the desktop window.
    pub fn window_mut(&mut self) -> &mut AngNativeDesktop {
        &mut self.desktop
    }

    /// Current demo counter value.
    pub fn counter(&self) -> i32 {
        self.counter_value
    }

    /// Hit-test a click against the retained view tree's Button nodes.
    /// Updates the counter, dispatches a SetTextContent batch to the Rust host,
    /// and returns true when a button was hit.
    pub fn on_click(&mut self, x: f32, y: f32) -> bool {
        let button = self
            .desktop
            .view_tree()
            .nodes()
            .find(|node| {
                node.view_type == "Button"
                    && x >= node.x
                    && x <= node.x + node.width
                    && y >= node.y
                    && y <= node.y + node.height
            })
            .map(|node| node.id);

        let Some(id) = button else { return false };

        match id {
            5 => self.counter_value -= 1,
            6 => self.counter_value = 0,
            7 => self.counter_value += 1,
            _ => {}
        }

        let batch = CommandBatch {
            protocol_version: 1,
            frame_id: self.counter_value as u64,
            timestamp_ms: 0,
            commands: vec![Command::SetTextContent {
                id: 8,
                text: format!("Count: {}", self.counter_value),
            }],
        };
        let _ = self.desktop.process_batch(&batch);
        self.desktop.set_visual_state(self.counter_value, Some(id));
        true
    }

    /// Clear the pressed-button highlight.
    pub fn release_button(&mut self) {
        self.desktop.set_visual_state(self.counter_value, None);
    }

    /// Render the current frame.
    pub fn render(&mut self) -> Result<(), String> {
        self.desktop.render()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use angnative_host_core::command::PROTOCOL_VERSION;

    #[test]
    fn demo_batch_uses_wire_protocol_fields() {
        let batch = DemoApp::demo_batch();
        assert_eq!(batch.protocol_version, PROTOCOL_VERSION);
        assert_eq!(batch.frame_id, 0);
        assert_eq!(batch.timestamp_ms, 0);
        assert!(!batch.commands.is_empty());

        // The demo tree root (id 1) hosts all three top-level children.
        let inserts: Vec<(u64, u64)> = batch
            .commands
            .iter()
            .filter_map(|cmd| match cmd {
                Command::InsertChild {
                    parent_id,
                    child_id,
                    ..
                } if *parent_id == 1 => Some((*parent_id, *child_id)),
                _ => None,
            })
            .collect();
        assert_eq!(inserts, vec![(1, 2), (1, 3), (1, 4), (1, 8)]);
    }

    #[test]
    fn demo_batch_round_trips_through_serde() {
        let batch = DemoApp::demo_batch();
        let json = serde_json::to_string(&batch).expect("serialize");
        let decoded: CommandBatch = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.protocol_version, batch.protocol_version);
        assert_eq!(decoded.frame_id, batch.frame_id);
        assert_eq!(decoded.commands.len(), batch.commands.len());
    }

    #[test]
    fn layout_inputs_cover_every_demo_node() {
        let inputs = DemoApp::layout_inputs();
        assert_eq!(inputs.len(), 8);
        assert!(inputs.contains_key(&1));
        assert_eq!(inputs.get(&1).unwrap().children, vec![2, 3, 4, 8]);
        assert_eq!(inputs.get(&4).unwrap().children, vec![5, 6, 7]);
    }
}
