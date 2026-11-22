/**
 * gpu_free_bridge_view_tree_integration.rs — P2-09 desktop bridge/tree test.
 *
 * Exercises the public `DesktopBridge` + `LocalViewTree` path with raw
 * commands and layout inputs only. No `AngNativeDesktop`, `DesktopRenderer`,
 * winit window, wgpu device/surface or other GPU/windowing resource is built.
 *
 * This GPU-free desktop integration test verifies the retained bridge/tree
 * state only; real window/GPU rendering is NOT verified by this test.
 */
use angnative_desktop::{DesktopBridge, LocalViewTree};
use angnative_host_core::command::{Command, CommandBatch};
use angnative_host_layout::LayoutNodeInput;
use angnative_host_view::ops::PlatformOp;
use serde_json::json;
use std::collections::HashMap;

/// Tolerance used by the layout crate's own geometry assertions.
const GEOMETRY_TOLERANCE: f32 = 1.0;

fn command_batch() -> CommandBatch {
    let mut batch = CommandBatch::new(7, 1234);

    batch.push(Command::CreateNode {
        id: 1,
        node_type: "View".to_string(),
    });
    batch.push(Command::SetStyle {
        id: 1,
        key: "flexDirection".to_string(),
        value: "column".to_string(),
    });
    batch.push(Command::SetProperty {
        id: 1,
        key: "testId".to_string(),
        value: json!("gpu-free-root"),
    });

    batch.push(Command::CreateNode {
        id: 2,
        node_type: "Text".to_string(),
    });
    batch.push(Command::SetTextContent {
        id: 2,
        text: "GPU-free bridge".to_string(),
    });
    batch.push(Command::SetStyle {
        id: 2,
        key: "color".to_string(),
        value: "#ffffff".to_string(),
    });
    batch.push(Command::InsertChild {
        parent_id: 1,
        child_id: 2,
        index: 0,
    });

    batch.push(Command::CreateNode {
        id: 3,
        node_type: "View".to_string(),
    });
    batch.push(Command::SetProperty {
        id: 3,
        key: "accessibilityRole".to_string(),
        value: json!("group"),
    });
    batch.push(Command::InsertChild {
        parent_id: 1,
        child_id: 3,
        index: 1,
    });

    batch.push(Command::CreateNode {
        id: 4,
        node_type: "Button".to_string(),
    });
    batch.push(Command::SetTextContent {
        id: 4,
        text: "Run".to_string(),
    });
    batch.push(Command::SetStyle {
        id: 4,
        key: "backgroundColor".to_string(),
        value: "#3355ff".to_string(),
    });
    batch.push(Command::SetProperty {
        id: 4,
        key: "disabled".to_string(),
        value: json!(false),
    });
    batch.push(Command::InsertChild {
        parent_id: 3,
        child_id: 4,
        index: 0,
    });

    batch
}

fn style(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect()
}

fn layout_inputs() -> HashMap<u64, LayoutNodeInput> {
    let mut nodes = HashMap::new();
    nodes.insert(
        1,
        LayoutNodeInput {
            node_id: 1,
            styles: style(&[
                ("width", "300px"),
                ("height", "120px"),
                ("flexDirection", "column"),
                ("padding", "10px"),
                ("gap", "8px"),
            ]),
            children: vec![2, 3],
        },
    );
    nodes.insert(
        2,
        LayoutNodeInput {
            node_id: 2,
            styles: style(&[("width", "120px"), ("height", "20px")]),
            children: Vec::new(),
        },
    );
    nodes.insert(
        3,
        LayoutNodeInput {
            node_id: 3,
            styles: style(&[
                ("width", "160px"),
                ("height", "40px"),
                ("flexDirection", "row"),
            ]),
            children: vec![4],
        },
    );
    nodes.insert(
        4,
        LayoutNodeInput {
            node_id: 4,
            styles: style(&[("width", "60px"), ("height", "24px")]),
            children: Vec::new(),
        },
    );
    nodes
}

fn assert_close(actual: f32, expected: f32, label: &str) {
    assert!(
        (actual - expected).abs() <= GEOMETRY_TOLERANCE,
        "{label}: expected {expected}, got {actual}"
    );
}

#[test]
fn bridge_ops_build_retained_tree_and_layout_frames_are_applied_without_gpu() {
    let mut bridge = DesktopBridge::new();
    let mut tree = LocalViewTree::new();

    let batch = command_batch();
    let ops = bridge
        .process_batch(&batch)
        .expect("raw command batch must process without a renderer");
    tree.apply_ops(&ops);

    // OpEmitter emits Create/SetProps/SetText/Insert in command order; only
    // the resulting retained state is asserted here, not the exact op stream.
    assert_eq!(tree.len(), 4);
    assert_eq!(tree.nodes().map(|n| n.id).collect::<Vec<_>>(), vec![1, 2, 3, 4]);

    let root = tree.node(1).expect("root view");
    assert_eq!(root.view_type, "View");
    assert_eq!(root.parent, None);
    assert_eq!(root.children, vec![2, 3]);
    assert_eq!(
        root.props.styles.get("flexDirection").map(String::as_str),
        Some("column")
    );
    assert_eq!(
        root.props.properties.get("testId"),
        Some(&json!("gpu-free-root"))
    );

    let title = tree.node(2).expect("title text");
    assert_eq!(title.view_type, "Text");
    assert_eq!(title.parent, Some(1));
    assert!(title.children.is_empty());
    assert_eq!(title.text.as_deref(), Some("GPU-free bridge"));
    assert_eq!(title.props.text.as_deref(), Some("GPU-free bridge"));
    assert_eq!(
        title.props.styles.get("color").map(String::as_str),
        Some("#ffffff")
    );

    let row = tree.node(3).expect("button row");
    assert_eq!(row.view_type, "View");
    assert_eq!(row.parent, Some(1));
    assert_eq!(row.children, vec![4]);
    assert_eq!(
        row.props.properties.get("accessibilityRole"),
        Some(&json!("group"))
    );

    let button = tree.node(4).expect("action button");
    assert_eq!(button.view_type, "Button");
    assert_eq!(button.parent, Some(3));
    assert!(button.children.is_empty());
    assert_eq!(button.text.as_deref(), Some("Run"));
    assert_eq!(
        button.props.styles.get("backgroundColor").map(String::as_str),
        Some("#3355ff")
    );
    assert_eq!(
        button.props.properties.get("disabled"),
        Some(&json!(false))
    );

    let layout = bridge
        .run_layout(&layout_inputs(), 1, 400.0, 300.0)
        .expect("layout must not require a window");
    assert_eq!(layout.nodes.len(), 4);
    assert_eq!(bridge.last_layout().map(|l| l.nodes.len()), Some(4));

    let frame_ops = bridge
        .apply_layout(&layout)
        .expect("layout results must become platform ops");
    assert_eq!(frame_ops.len(), 4);
    assert!(
        frame_ops
            .iter()
            .all(|op| matches!(op, PlatformOp::SetFrame { .. })),
        "apply_layout should emit only SetFrame ops for these created views"
    );
    tree.apply_ops(&frame_ops);

    // Root frame and nested frames are parent-relative; compare the retained
    // tree to the layout result and to deterministic expected values.
    for rect in &layout.nodes {
        let node = tree.node(rect.node_id).expect("layout node retained");
        assert_close(node.x, rect.x, "x matches layout");
        assert_close(node.y, rect.y, "y matches layout");
        assert_close(node.width, rect.width, "width matches layout");
        assert_close(node.height, rect.height, "height matches layout");
    }

    let root = tree.node(1).expect("root view");
    assert_close(root.x, 0.0, "root.x");
    assert_close(root.y, 0.0, "root.y");
    assert_close(root.width, 300.0, "root.width");
    assert_close(root.height, 120.0, "root.height");

    let title = tree.node(2).expect("title text");
    assert_close(title.x, 10.0, "title.x");
    assert_close(title.y, 10.0, "title.y");
    assert_close(title.width, 120.0, "title.width");
    assert_close(title.height, 20.0, "title.height");

    let row = tree.node(3).expect("button row");
    assert_close(row.x, 10.0, "row.x");
    assert_close(row.y, 38.0, "row.y");
    assert_close(row.width, 160.0, "row.width");
    assert_close(row.height, 40.0, "row.height");

    let button = tree.node(4).expect("action button");
    assert_close(button.x, 0.0, "button.x");
    assert_close(button.y, 0.0, "button.y");
    assert_close(button.width, 60.0, "button.width");
    assert_close(button.height, 24.0, "button.height");

    // Tree structure and visual state survive the layout commit unchanged.
    assert_eq!(tree.len(), 4);
    assert_eq!(tree.node(1).expect("root").children, vec![2, 3]);
    assert_eq!(tree.node(3).expect("row").children, vec![4]);
    assert_eq!(
        tree.node(2).expect("title").text.as_deref(),
        Some("GPU-free bridge")
    );
    assert_eq!(tree.node(4).expect("button").text.as_deref(), Some("Run"));
}
