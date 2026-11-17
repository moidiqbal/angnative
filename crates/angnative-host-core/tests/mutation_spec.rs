/**
 * mutation_spec.rs — Integration tests for angnative-host-core's MutationEngine,
 * mirroring the TypeScript mutation-teeth specs
 * (`packages/testing/src/mutation-teeth.spec.ts`).
 *
 * These assert correct (green) behavior. The TS mutant specs assert the
 * conformance suite goes RED with these same fixtures.
 */
use angnative_host_core::command::{Command, CommandBatch};
use angnative_host_core::mutation::MutationEngine;
use angnative_host_core::NodeId;

fn create_node(id: NodeId, node_type: &str) -> Command {
    Command::CreateNode {
        id,
        node_type: node_type.to_string(),
    }
}

fn insert_child(parent_id: NodeId, child_id: NodeId, index: usize) -> Command {
    Command::InsertChild {
        parent_id,
        child_id,
        index,
    }
}

fn set_text(id: NodeId, text: &str) -> Command {
    Command::SetTextContent {
        id,
        text: text.to_string(),
    }
}

fn set_property(id: NodeId, key: &str, value: serde_json::Value) -> Command {
    Command::SetProperty {
        id,
        key: key.to_string(),
        value,
    }
}

fn remove_child(parent_id: NodeId, child_id: NodeId) -> Command {
    Command::RemoveChild {
        parent_id,
        child_id,
    }
}

fn remove_node(id: NodeId) -> Command {
    Command::RemoveNode { id }
}

#[test]
fn create_append_and_remove_produces_expected_tree() {
    let mut engine = MutationEngine::new();
    engine.run_command(create_node(1, "View")).unwrap();
    engine.run_command(create_node(2, "View")).unwrap();
    engine.run_command(create_node(3, "Text")).unwrap();
    engine.run_command(insert_child(1, 2, 0)).unwrap();
    engine.run_command(insert_child(1, 3, 1)).unwrap();
    engine.run_command(set_text(3, "hello")).unwrap();
    engine.run_command(remove_child(1, 3)).unwrap();
    engine.run_command(remove_node(3)).unwrap();

    let snapshot = engine.snapshot();

    let n1 = snapshot.nodes.iter().find(|n| n.id == 1).unwrap();
    assert_eq!(n1.children, vec![2]);

    let n2 = snapshot.nodes.iter().find(|n| n.id == 2).unwrap();
    assert_eq!(n2.children, Vec::<NodeId>::new());

    let n3 = snapshot.nodes.iter().find(|n| n.id == 3);
    assert!(n3.is_none());
}

#[test]
fn reparent_detaches_from_old_parent() {
    let mut engine = MutationEngine::new();
    engine.run_command(create_node(1, "View")).unwrap();
    engine.run_command(create_node(2, "View")).unwrap();
    engine.run_command(create_node(3, "View")).unwrap();
    engine.run_command(insert_child(1, 2, 0)).unwrap();
    engine.run_command(insert_child(2, 3, 0)).unwrap();
    engine.run_command(remove_child(2, 3)).unwrap();
    engine.run_command(insert_child(1, 3, 1)).unwrap();

    let snapshot = engine.snapshot();

    let n2 = snapshot.nodes.iter().find(|n| n.id == 2).unwrap();
    assert_eq!(n2.children, Vec::<NodeId>::new());

    let n1 = snapshot.nodes.iter().find(|n| n.id == 1).unwrap();
    assert_eq!(n1.children, vec![2, 3]);

    let n3 = snapshot.nodes.iter().find(|n| n.id == 3).unwrap();
    assert_eq!(n3.parent_id, Some(1));
}

#[test]
fn set_text_content_is_not_a_noop() {
    let mut engine = MutationEngine::new();
    engine.run_command(create_node(1, "Text")).unwrap();
    engine.run_command(create_node(2, "TextInput")).unwrap();
    engine.run_command(set_text(1, "hello world")).unwrap();
    engine.run_command(set_text(2, "initial value")).unwrap();

    let snapshot = engine.snapshot();
    let n1 = snapshot.nodes.iter().find(|n| n.id == 1).unwrap();
    assert_eq!(n1.text_content, Some("hello world".to_string()));

    let n2 = snapshot.nodes.iter().find(|n| n.id == 2).unwrap();
    assert_eq!(n2.text_content, Some("initial value".to_string()));
}

#[test]
fn remove_child_requires_membership() {
    let mut engine = MutationEngine::new();
    engine.run_command(create_node(1, "View")).unwrap();
    engine.run_command(create_node(2, "View")).unwrap();
    engine.run_command(create_node(3, "View")).unwrap();
    engine.run_command(insert_child(1, 2, 0)).unwrap();

    let result = engine.run_command(remove_child(3, 2));
    assert!(result.is_err());
}

#[test]
fn set_property_null_removes_property() {
    let mut engine = MutationEngine::new();
    engine.run_command(create_node(1, "View")).unwrap();
    engine
        .run_command(set_property(1, "foo", serde_json::json!("bar")))
        .unwrap();
    engine
        .run_command(set_property(1, "foo", serde_json::Value::Null))
        .unwrap();

    let snapshot = engine.snapshot();
    let n1 = snapshot.nodes.iter().find(|n| n.id == 1).unwrap();
    assert!(!n1.properties.contains_key("foo"));
}

#[test]
fn insert_child_respects_index() {
    let mut engine = MutationEngine::new();
    engine.run_command(create_node(1, "View")).unwrap();
    engine.run_command(create_node(2, "View")).unwrap();
    engine.run_command(create_node(3, "View")).unwrap();
    engine.run_command(insert_child(1, 2, 0)).unwrap();
    engine.run_command(insert_child(1, 3, 0)).unwrap();

    let snapshot = engine.snapshot();
    let n1 = snapshot.nodes.iter().find(|n| n.id == 1).unwrap();
    assert_eq!(n1.children, vec![3, 2]);
}

#[test]
fn protocol_version_validation_rejects_unsupported() {
    let mut engine = MutationEngine::new();
    let bad_batch = CommandBatch {
        protocol_version: 99,
        frame_id: 0,
        timestamp_ms: 0,
        commands: vec![],
    };
    let result = engine.run_batch(bad_batch);
    assert!(result.is_err());
}

#[test]
fn create_node_rejects_unknown_type() {
    let mut engine = MutationEngine::new();
    let result = engine.run_command(create_node(99, "BogusType"));
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Unknown node type"));
}
