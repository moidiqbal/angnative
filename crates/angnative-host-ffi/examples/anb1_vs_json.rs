//! Host-level ANB1 vs JSON measurement (evidence harness, not a test).
//!
//! Run from the workspace root:
//!   cargo run -p angnative-host-ffi --release --example anb1_vs_json -- \
//!     docs/benchmarks/anb1-vs-json/host-2026-11-28.json
//!
//! Both inputs are precomputed once. Every measured iteration gets a fresh
//! `Host`, and only the `process_batch_json` / `process_batch_binary` call is
//! timed. The decode, stage-then-commit pipeline and ops serialization are all
//! inside the timed region; Host construction and input generation are not.
//! This is a host-only measurement — no device or throughput superiority claim.

use angnative_host_core::binary::encode_binary_batch;
use angnative_host_core::command::{Command, CommandBatch};
use angnative_host_ffi::bridge::Host;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::process::Command as ProcessCommand;
use std::time::Instant;

const NODES: usize = 120;
const WARMUP: usize = 24;
const MEASURED: usize = 160;

fn make_batch() -> CommandBatch {
    let mut batch = CommandBatch::new(1, 1_700_000_000_000);
    for id in 1..=NODES as u64 {
        let node_type = if id % 4 == 0 { "Text" } else { "View" };
        batch.push(Command::CreateNode {
            id,
            node_type: node_type.to_string(),
        });
    }
    for id in 1..=NODES as u64 {
        batch.push(Command::SetStyle {
            id,
            key: "color".to_string(),
            value: "#3b82f6".to_string(),
        });
        batch.push(Command::SetProperty {
            id,
            key: "opacity".to_string(),
            value: json!(0.9),
        });
        if id % 4 == 0 {
            batch.push(Command::SetTextContent {
                id,
                text: format!("row {id} — benchmark payload"),
            });
        }
    }
    for id in 2..=NODES as u64 {
        batch.push(Command::InsertChild {
            parent_id: 1,
            child_id: id,
            index: (id - 2) as usize,
        });
    }
    batch
}

fn git_output(args: &[&str]) -> String {
    ProcessCommand::new("git")
        .args(args)
        .output()
        .ok()
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .unwrap_or_default()
}

fn utc_unix_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn tool_version(binary: &str, args: &[&str]) -> String {
    ProcessCommand::new(binary)
        .args(args)
        .output()
        .ok()
        .map(|out| {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            stdout.lines().chain(stderr.lines()).next().unwrap_or("").trim().to_string()
        })
        .unwrap_or_default()
}

fn stats(mut values: Vec<f64>) -> Value {
    let count = values.len();
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mean = values.iter().sum::<f64>() / count as f64;
    json!({
        "samples": count,
        "min_us": values[0],
        "p50_us": values[count / 2],
        "p95_us": values[count.saturating_mul(95) / 100],
        "max_us": values[count - 1],
        "mean_us": mean,
        "raw_us": values,
    })
}

fn main() -> Result<(), String> {
    let output = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from("docs/benchmarks/anb1-vs-json/host-2026-11-28.json")
        });

    let batch = make_batch();
    let json = serde_json::to_string(&batch).map_err(|e| e.to_string())?;
    let anb1 = encode_binary_batch(&batch)?;

    // Correctness gate: both wire formats must drive the identical pipeline to
    // the identical ops output before any timing is recorded.
    let json_ops = Host::new()
        .process_batch_json(&json)
        .map_err(|e| format!("JSON batch failed: {e}"))?;
    let anb1_ops = Host::new()
        .process_batch_binary(&anb1)
        .map_err(|e| format!("ANB1 batch failed: {e}"))?;
    if json_ops != anb1_ops {
        return Err("JSON and ANB1 produced different ops; refusing to benchmark".to_string());
    }
    let ops_bytes = json_ops.len();

    for _ in 0..WARMUP {
        let mut host = Host::new();
        std::hint::black_box(
            host.process_batch_json(&json)
                .map_err(|e| e.to_string())?,
        );
        let mut host = Host::new();
        std::hint::black_box(
            host.process_batch_binary(&anb1)
                .map_err(|e| e.to_string())?,
        );
    }

    let mut json_samples = Vec::with_capacity(MEASURED);
    let mut anb1_samples = Vec::with_capacity(MEASURED);
    for index in 0..MEASURED {
        // Alternate order so any systematic drift affects both formats.
        let json_first = index % 2 == 0;
        let run_json = |samples: &mut Vec<f64>| -> Result<(), String> {
            let mut host = Host::new();
            let start = Instant::now();
            let ops = host.process_batch_json(&json).map_err(|e| e.to_string())?;
            let elapsed = start.elapsed();
            std::hint::black_box(ops);
            samples.push(elapsed.as_secs_f64() * 1_000_000.0);
            Ok(())
        };
        let run_anb1 = |samples: &mut Vec<f64>| -> Result<(), String> {
            let mut host = Host::new();
            let start = Instant::now();
            let ops = host.process_batch_binary(&anb1).map_err(|e| e.to_string())?;
            let elapsed = start.elapsed();
            std::hint::black_box(ops);
            samples.push(elapsed.as_secs_f64() * 1_000_000.0);
            Ok(())
        };
        if json_first {
            run_json(&mut json_samples)?;
            run_anb1(&mut anb1_samples)?;
        } else {
            run_anb1(&mut anb1_samples)?;
            run_json(&mut json_samples)?;
        }
    }

    let evidence = json!({
        "title": "Host-level ANB1 vs JSON batch processing (no superiority claim)",
        "captured_at_unix_ms": utc_unix_ms(),
        "environment": {
            "git_head": git_output(&["rev-parse", "HEAD"]),
            "rustc": tool_version("rustc", &["--version"]),
            "cargo": tool_version("cargo", &["--version"]),
            "target": format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS),
            "command": "cargo run -p angnative-host-ffi --release --example anb1_vs_json -- <output>",
            "profile": "workspace release (opt-level=z, lto=fat, codegen-units=1, strip=true)",
        },
        "method": {
            "inputs_precomputed_once": true,
            "fresh_host_per_iteration": true,
            "timed_region": "process_batch_json / process_batch_binary only (decode + stage-then-commit + ops serialization)",
            "excluded_from_timing": ["batch generation", "Host::new()", "encoding setup"],
            "warmup_iterations": WARMUP,
            "measured_iterations": MEASURED,
            "alternating_order": true,
            "identical_ops_verified_before_timing": true,
            "view_factory": "none (no native views materialized)",
            "units": "microseconds measured with std::time::Instant",
        },
        "input": {
            "commands": batch.commands.len(),
            "json_bytes": json.len(),
            "anb1_bytes": anb1.len(),
            "ops_json_bytes": ops_bytes,
        },
        "json": stats(json_samples),
        "anb1": stats(anb1_samples),
        "claim_boundary": {
            "host_only": true,
            "device_jni_arm": "not verified here; reported separately",
            "superiority_claim": "none — samples describe this host, input and profile only",
            "end_to_end_zero_copy": "not claimed; decode produces an owned batch",
        },
    });

    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let pretty = serde_json::to_string_pretty(&evidence).map_err(|e| e.to_string())?;
    fs::write(&output, pretty + "\n").map_err(|e| e.to_string())?;
    println!(
        "wrote {}: json p50 {:.3} us / anb1 p50 {:.3} us (no superiority claim)",
        output.display(),
        evidence["json"]["p50_us"].as_f64().unwrap(),
        evidence["anb1"]["p50_us"].as_f64().unwrap(),
    );
    Ok(())
}
