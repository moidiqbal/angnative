/**
 * conformance_spec.rs — Rust conformance suite runner (Task R4 / keystone).
 *
 * Runs the SAME JSON fixtures as the TypeScript fake backend
 * (`packages/testing/fixtures/`) through the Rust host
 * (`angnative-host-core` MutationEngine + `angnative-host-view` OpEmitter)
 * and asserts identical results.
 *
 * This is the keystone of the AngNative plan (TaskList §4): without a shared,
 * mutation-tested suite, swapping the backend is a rewrite with no detector.
 * With it, the Rust host is a drop-in that must pass a suite that already
 * has teeth.
 *
 * The fixtures and expected results are identical to the TS side — only the
 * implementation language differs. If TS is green and Rust is red (or vice
 * versa), there's a bug in one of them.
 */
use angnative_host_core::command::{Command, CommandBatch};
use angnative_host_core::mutation::MutationEngine;
use angnative_host_view::op_emitter::OpEmitter;
use serde_json::Value;
use std::fs;

/// Path to the shared JSON fixtures (relative to crate root when running
/// `cargo test` in `crates/angnative-host-view/`).
const FIXTURES_DIR: &str = "../../packages/testing/fixtures";

/// A loaded conformance fixture.
struct Fixture {
    name: String,
    commands: Vec<Command>,
    expect: Value,
}

/// Load all fixtures listed in manifest.json.
fn load_fixtures() -> Vec<Fixture> {
    let manifest_path = format!("{FIXTURES_DIR}/manifest.json");
    let manifest: Value = serde_json::from_str(
        &fs::read_to_string(&manifest_path)
            .unwrap_or_else(|_| panic!("Failed to read manifest at {manifest_path}")),
    )
    .unwrap();

    let names: Vec<String> = manifest
        .as_array()
        .expect("manifest.json must be an array")
        .iter()
        .map(|v| {
            v.as_str()
                .expect("manifest entry must be a string")
                .to_string()
        })
        .collect();

    let mut fixtures = Vec::new();
    for name in &names {
        let path = format!("{FIXTURES_DIR}/{name}.json");
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("Failed to read fixture at {path}"));
        let fixture: Value = serde_json::from_str(&content)
            .unwrap_or_else(|_| panic!("Failed to parse fixture at {path}"));

        let commands: Vec<Command> = serde_json::from_value(fixture["commands"].clone())
            .unwrap_or_else(|error| panic!("Fixture {name} has invalid commands: {error}"));

        fixtures.push(Fixture {
            name: name.clone(),
            commands,
            expect: fixture["expect"].clone(),
        });
    }
    fixtures
}

/// Run commands through a fresh MutationEngine and return the snapshot.
fn run_engine(commands: &[Command]) -> MutationEngine {
    let mut engine = MutationEngine::new();
    for cmd in commands {
        engine.run_command(cmd.clone()).unwrap_or_else(|e| {
            panic!("MutationEngine failed on command {cmd:?}: {e}");
        });
    }
    engine
}

/// Compare node state against fixture expectations.
fn assert_nodes_match(engine: &MutationEngine, expect: &Value, fixture_name: &str) {
    let nodes = expect["nodes"].as_object();
    if nodes.is_none() {
        return;
    }
    let nodes = nodes.unwrap();

    for (node_id_str, expected) in nodes {
        let node_id: u64 = node_id_str.parse().unwrap_or_else(|_| {
            panic!("Fixture {fixture_name}: invalid node id '{node_id_str}'");
        });

        let actual = engine.get_node(node_id).unwrap_or_else(|| {
            panic!("Fixture {fixture_name}: node {node_id} exists in expect but not in tree");
        });

        // nodeType (exact wire string, e.g. `Custom:na-list`)
        if let Some(expected_node_type) = expected["nodeType"].as_str() {
            assert_eq!(
                actual.node_type, expected_node_type,
                "Fixture {fixture_name}: node {node_id} nodeType mismatch"
            );
        }

        // children
        if let Some(expected_children) = expected["children"].as_array() {
            let expected_children: Vec<u64> = expected_children
                .iter()
                .map(|v| v.as_u64().unwrap())
                .collect();
            assert_eq!(
                actual.children, expected_children,
                "Fixture {fixture_name}: node {node_id} children mismatch"
            );
        }

        // parentId
        if let Some(expected_parent) = expected["parentId"].as_u64() {
            assert_eq!(
                actual.parent_id,
                Some(expected_parent),
                "Fixture {fixture_name}: node {node_id} parentId mismatch"
            );
        }

        // textContent
        if let Some(expected_text) = expected["textContent"].as_str() {
            assert_eq!(
                actual.text_content.as_deref(),
                Some(expected_text),
                "Fixture {fixture_name}: node {node_id} textContent mismatch"
            );
        } else if expected["textContent"].is_null() {
            assert!(
                actual.text_content.is_none(),
                "Fixture {fixture_name}: node {node_id} textContent should be None"
            );
        }

        // styles
        if let Some(expected_styles) = expected["styles"].as_object() {
            for (key, expected_value) in expected_styles {
                let actual_value = actual.styles.get(key).cloned();
                let expected_value = expected_value.as_str().unwrap();
                assert_eq!(
                    actual_value.as_deref(),
                    Some(expected_value),
                    "Fixture {fixture_name}: node {node_id} style {key} mismatch"
                );
            }
        }

        // properties
        if let Some(expected_props) = expected["properties"].as_object() {
            for (key, expected_value) in expected_props {
                let actual_value = actual.properties.get(key);
                assert!(
                    actual_value.is_some(),
                    "Fixture {fixture_name}: node {node_id} property '{key}' missing"
                );
                assert_eq!(
                    actual_value,
                    Some(expected_value),
                    "Fixture {fixture_name}: node {node_id} property '{key}' mismatch"
                );
            }
        }
    }
}

/// Compare final tree shape against fixture expectations.
fn assert_final_tree_match(engine: &MutationEngine, expect: &Value, fixture_name: &str) {
    let final_tree = expect["finalTree"].as_object();
    if final_tree.is_none() {
        return;
    }
    let final_tree = final_tree.unwrap();

    for (node_id_str, expected) in final_tree {
        let node_id: u64 = node_id_str.parse().unwrap_or_else(|_| {
            panic!("Fixture {fixture_name}: invalid node id '{node_id_str}'");
        });

        if expected.is_null() {
            // Node should not exist in tree
            assert!(
                engine.get_node(node_id).is_none(),
                "Fixture {fixture_name}: node {node_id} should not exist in tree"
            );
            continue;
        }

        let actual = engine.get_node(node_id).unwrap_or_else(|| {
            panic!("Fixture {fixture_name}: node {node_id} exists in expect but not in tree");
        });

        if let Some(expected_children) = expected["children"].as_array() {
            let expected_children: Vec<u64> = expected_children
                .iter()
                .map(|v| v.as_u64().unwrap())
                .collect();
            assert_eq!(
                actual.children, expected_children,
                "Fixture {fixture_name}: finalTree node {node_id} children mismatch"
            );
        }

        if let Some(expected_text) = expected["textContent"].as_str() {
            assert_eq!(
                actual.text_content.as_deref(),
                Some(expected_text),
                "Fixture {fixture_name}: finalTree node {node_id} textContent mismatch"
            );
        }
    }
}

/// Compare mutation kinds against fixture expectations.
fn assert_mutation_kinds_match(engine: &mut MutationEngine, expect: &Value, fixture_name: &str) {
    let expected_kinds = expect["mutationKinds"].as_array();
    if expected_kinds.is_none() {
        return;
    }
    let expected_kinds: Vec<String> = expected_kinds
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    let actual_kinds = engine.mutation_kinds();
    assert_eq!(
        actual_kinds, expected_kinds,
        "Fixture {fixture_name}: mutation kinds mismatch"
    );
}

#[test]
fn conformance_runs_all_shared_fixtures() {
    let fixtures = load_fixtures();
    assert!(!fixtures.is_empty(), "No fixtures loaded");

    for fixture in &fixtures {
        let mut engine = run_engine(&fixture.commands);

        // All non-error fixtures should run without panic (engine.run_command
        // already unwraps errors, so reaching here means success).
        assert_mutation_kinds_match(&mut engine, &fixture.expect, &fixture.name);
        assert_nodes_match(&engine, &fixture.expect, &fixture.name);
        assert_final_tree_match(&engine, &fixture.expect, &fixture.name);
    }
}

#[test]
fn conformance_create_append_remove() {
    let fixture = load_fixtures()
        .into_iter()
        .find(|f| f.name == "create-append-remove")
        .unwrap();

    let mut engine = run_engine(&fixture.commands);

    // Node 1: children [2]
    let n1 = engine.get_node(1).unwrap();
    assert_eq!(n1.children, vec![2]);

    // Node 2: no children
    let n2 = engine.get_node(2).unwrap();
    assert_eq!(n2.children, Vec::<u64>::new());

    // Node 3: removed
    assert!(engine.get_node(3).is_none());

    // Mutation kinds
    let kinds = engine.mutation_kinds();
    assert_eq!(
        kinds,
        vec![
            "CreateNode",
            "CreateNode",
            "CreateNode",
            "InsertChild",
            "InsertChild",
            "SetTextContent",
            "RemoveChild",
            "RemoveNode",
        ]
    );
}

#[test]
fn conformance_set_property_and_style() {
    let fixture = load_fixtures()
        .into_iter()
        .find(|f| f.name == "set-property-and-style")
        .unwrap();

    let engine = run_engine(&fixture.commands);

    let n1 = engine.get_node(1).unwrap();
    assert_eq!(n1.styles.get("flexDirection"), Some(&"row".to_string()));
    assert_eq!(n1.styles.get("width"), Some(&"200px".to_string()));
    assert_eq!(n1.styles.get("color"), Some(&"#ff0000".to_string()));
    assert_eq!(
        n1.properties.get("disabled"),
        Some(&serde_json::json!(true))
    );
    assert_eq!(n1.properties.get("count"), Some(&serde_json::json!(42)));
}

#[test]
fn conformance_set_text_content() {
    let fixture = load_fixtures()
        .into_iter()
        .find(|f| f.name == "set-text-content")
        .unwrap();

    let engine = run_engine(&fixture.commands);

    let n1 = engine.get_node(1).unwrap();
    assert_eq!(n1.text_content.as_deref(), Some("hello world"));

    let n2 = engine.get_node(2).unwrap();
    assert_eq!(n2.text_content.as_deref(), Some("initial value"));
}

#[test]
fn conformance_reparent() {
    let fixture = load_fixtures()
        .into_iter()
        .find(|f| f.name == "reparent")
        .unwrap();

    let engine = run_engine(&fixture.commands);

    // Node 2: children detached (3 was reparented to 1)
    let n2 = engine.get_node(2).unwrap();
    assert_eq!(n2.children, Vec::<u64>::new());

    // Node 1: children [2, 3]
    let n1 = engine.get_node(1).unwrap();
    assert_eq!(n1.children, vec![2, 3]);

    // Node 3: parent is 1
    let n3 = engine.get_node(3).unwrap();
    assert_eq!(n3.parent_id, Some(1));
}

#[test]
fn conformance_list_recycling() {
    let fixture = load_fixtures()
        .into_iter()
        .find(|f| f.name == "list-recycling")
        .unwrap();

    let engine = run_engine(&fixture.commands);

    // Node 1: children [2, 3] (2 was removed and re-inserted)
    let n1 = engine.get_node(1).unwrap();
    assert_eq!(n1.children, vec![2, 3]);
}

#[test]
fn conformance_multiple_listeners_per_event() {
    let fixture = load_fixtures()
        .into_iter()
        .find(|f| f.name == "multiple-listeners-per-event")
        .unwrap();

    let engine = run_engine(&fixture.commands);

    // Node 1: has both listener properties
    let n1 = engine.get_node(1).unwrap();
    assert!(n1.properties.contains_key("listener:press:1"));
    assert!(n1.properties.contains_key("listener:press:2"));
}

#[test]
fn conformance_view_flattening() {
    let fixture = load_fixtures()
        .into_iter()
        .find(|f| f.name == "view-flattening")
        .unwrap();

    let engine = run_engine(&fixture.commands);

    let n1 = engine.get_node(1).unwrap();
    assert_eq!(n1.children, vec![2]);

    let n2 = engine.get_node(2).unwrap();
    assert_eq!(n2.children, vec![3]);

    let n3 = engine.get_node(3).unwrap();
    assert_eq!(n3.children, Vec::<u64>::new());
}

#[test]
fn conformance_capability_degradation() {
    let fixture = load_fixtures()
        .into_iter()
        .find(|f| f.name == "capability-degradation")
        .unwrap();

    let engine = run_engine(&fixture.commands);

    // Should run without error — degradation is diagnosed, not silent
    let n1 = engine.get_node(1).unwrap();
    assert_eq!(
        n1.properties.get("capability:unsupported-feature"),
        Some(&serde_json::json!(true))
    );
}

#[test]
fn conformance_native_primitives_tagged_custom() {
    let fixture = load_fixtures()
        .into_iter()
        .find(|f| f.name == "native-primitives-tagged-custom")
        .unwrap();

    let engine = run_engine(&fixture.commands);
    for (id, wire) in [
        (2u64, "Custom:na-list"),
        (3u64, "Custom:na-switch"),
        (4u64, "Custom:na-activity-indicator"),
        (5u64, "Custom"),
    ] {
        assert_eq!(
            engine.get_node(id).unwrap().node_type,
            wire,
            "node {id} wire node type"
        );
    }
    assert_eq!(engine.get_node(1).unwrap().children, vec![2, 3, 4, 5]);

    // Tagged customs become real native views; the untagged host stays flat.
    let batch = CommandBatch {
        protocol_version: 1,
        frame_id: 0,
        timestamp_ms: 0,
        commands: fixture.commands.clone(),
    };
    let ops = OpEmitter::new().process_batch(&batch).unwrap();
    let create_types: Vec<String> = ops
        .iter()
        .filter_map(|op| match op {
            angnative_host_view::ops::PlatformOp::Create { view_type, .. } => {
                Some(view_type.clone())
            }
            _ => None,
        })
        .collect();
    assert_eq!(
        create_types,
        vec![
            "View",
            "Custom:na-list",
            "Custom:na-switch",
            "Custom:na-activity-indicator",
        ]
    );
}

/* =========================================================================
 * Error cases — these must throw loudly, never silently (spec §5.3)
 * ========================================================================= */

#[test]
fn error_unknown_node_type_throws() {
    let mut engine = MutationEngine::new();
    let result = engine.run_command(Command::CreateNode {
        id: 99,
        node_type: "BogusType".to_string(),
    });
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Unknown node type"));
}

#[test]
fn error_remove_child_requires_membership() {
    let mut engine = MutationEngine::new();
    engine
        .run_command(Command::CreateNode {
            id: 1,
            node_type: "View".to_string(),
        })
        .unwrap();
    engine
        .run_command(Command::CreateNode {
            id: 2,
            node_type: "View".to_string(),
        })
        .unwrap();
    engine
        .run_command(Command::InsertChild {
            parent_id: 1,
            child_id: 2,
            index: 0,
        })
        .unwrap();

    // Remove child 2 from parent 3 — but 2 belongs to parent 1
    let result = engine.run_command(Command::RemoveChild {
        parent_id: 3,
        child_id: 2,
    });
    assert!(result.is_err());
}

#[test]
fn error_bad_protocol_version_throws() {
    let mut engine = MutationEngine::new();
    let bad_batch = CommandBatch {
        protocol_version: 99,
        frame_id: 0,
        timestamp_ms: 0,
        commands: vec![],
    };
    let result = engine.run_batch(bad_batch);
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Unsupported protocol version"));
}

/* =========================================================================
 * OpEmitter integration — commands produce ordered platform ops
 * ========================================================================= */

#[test]
fn op_emitter_create_insert_produces_ordered_ops() {
    let mut emitter = OpEmitter::new();
    let batch = CommandBatch {
        protocol_version: 1,
        frame_id: 0,
        timestamp_ms: 1000,
        commands: vec![
            Command::CreateNode {
                id: 1,
                node_type: "View".to_string(),
            },
            Command::CreateNode {
                id: 2,
                node_type: "Text".to_string(),
            },
            Command::InsertChild {
                parent_id: 1,
                child_id: 2,
                index: 0,
            },
        ],
    };

    let ops = emitter.process_batch(&batch).unwrap();
    let kinds: Vec<&str> = ops.iter().map(op_kind_str).collect();
    assert_eq!(kinds, vec!["Create", "Create", "Insert"]);

    // Verify Create ops have correct node types
    let create_ops: Vec<&angnative_host_view::ops::PlatformOp> = ops
        .iter()
        .filter(|o| matches!(o, angnative_host_view::ops::PlatformOp::Create { .. }))
        .collect();
    assert_eq!(create_ops.len(), 2);
}

#[test]
fn op_emitter_remove_child_produces_remove_op() {
    let mut emitter = OpEmitter::new();
    emitter
        .process_command(&Command::CreateNode {
            id: 1,
            node_type: "View".to_string(),
        })
        .unwrap();
    emitter
        .process_command(&Command::CreateNode {
            id: 2,
            node_type: "View".to_string(),
        })
        .unwrap();
    emitter
        .process_command(&Command::InsertChild {
            parent_id: 1,
            child_id: 2,
            index: 0,
        })
        .unwrap();

    emitter
        .process_command(&Command::RemoveChild {
            parent_id: 1,
            child_id: 2,
        })
        .unwrap();

    let ops = emitter.ops();
    let remove_ops: Vec<&angnative_host_view::ops::PlatformOp> = ops
        .iter()
        .filter(|o| matches!(o, angnative_host_view::ops::PlatformOp::Remove { .. }))
        .collect();
    assert_eq!(remove_ops.len(), 1);
}

fn op_kind_str(op: &angnative_host_view::ops::PlatformOp) -> &'static str {
    match op {
        angnative_host_view::ops::PlatformOp::Create { .. } => "Create",
        angnative_host_view::ops::PlatformOp::SetFrame { .. } => "SetFrame",
        angnative_host_view::ops::PlatformOp::SetProps { .. } => "SetProps",
        angnative_host_view::ops::PlatformOp::SetText { .. } => "SetText",
        angnative_host_view::ops::PlatformOp::Append { .. } => "Append",
        angnative_host_view::ops::PlatformOp::Insert { .. } => "Insert",
        angnative_host_view::ops::PlatformOp::Remove { .. } => "Remove",
        angnative_host_view::ops::PlatformOp::Destroy { .. } => "Destroy",
    }
}
