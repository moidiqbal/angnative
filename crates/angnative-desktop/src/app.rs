/**
 * app.rs — Demo application for AngNative desktop.
 *
 * Builds a command batch representing a counter app (view, text, buttons)
 * and drives the desktop shell through command batches + layout.
 */

use angnative_desktop::{AngNativeDesktop, DesktopBridge};
use angnative_host_core::command::{Command, CommandBatch};
use angnative_host_layout::{LayoutNodeInput, LayoutEngine};
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
    pub async fn new() -> Self {
        // Create event loop and window (will be set by main)
        // Note: In a real app, the event loop is passed from main.
        // Here we use a placeholder that gets replaced.
        panic!("DemoApp::new() should not be called directly. Use DemoApp::new_with_event_loop().");
    }

    pub async fn new_with_event_loop(
        event_loop: &winit::event_loop::EventLoop<()>,
    ) -> Result<Self, String> {
        let desktop = AngNativeDesktop::new(event_loop).await?;

        // Build the demo command batch
        let batch = Self::demo_batch();

        // Process through the Rust host
        desktop.bridge_mut().process_batch(&batch)?;

        // Build layout input from the command batch
        let nodes = Self::layout_inputs();

        // Run layout
        desktop.bridge_mut().run_layout(
            &nodes,
            1, // root node id
            800.0, // available width
            600.0, // available height
        )?;

        // Apply layout as SetFrame ops
        let layout = desktop.bridge.last_layout().unwrap();
        desktop.bridge_mut().apply_layout(layout)?;

        Ok(Self {
            desktop,
            counter_value: 0,
        })
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
            protocolVersion: 1,
            frameId: 0,
            timestampMs: 0,
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

    /// Render the current frame.
    pub fn render(&mut self) -> Result<(), String> {
        self.desktop.render()
    }
}
