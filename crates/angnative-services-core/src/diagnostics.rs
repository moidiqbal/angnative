/**
 * diagnostics.rs — Performance diagnostics channel (spec §9).
 *
 * One stream of diagnostic events consumed by dev overlay, CLI, and CI
 * reporter. Diagnostic types:
 *   - NATIVE_NO_BACKEND
 *   - NATIVE_CAPABILITY_UNAVAILABLE
 *   - NATIVE_CAPABILITY_DEGRADED
 *   - NATIVE_MODULE_UNAVAILABLE
 *   - NATIVE_STYLE_UNSUPPORTED
 *   - NATIVE_COMMIT_FAILED
 */

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/** Severity level of a diagnostic event. */
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticsLevel {
    Info,
    Warn,
    Error,
    Fatal,
}

/** Specific diagnostic event types from spec §9. */
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticsEventType {
    NativeNoBackend,
    NativeCapabilityUnavailable,
    NativeCapabilityDegraded,
    NativeModuleUnavailable,
    NativeStyleUnsupported,
    NativeCommitFailed,
    Custom(String),
}

/** A single diagnostic event. */
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticsEvent {
    pub id: String,
    pub event_type: DiagnosticsEventType,
    pub level: DiagnosticsLevel,
    pub message: String,
    pub timestamp_ms: u64,
    pub context: serde_json::Value,
    pub platform: Option<String>,
}

/** A channel that buffers diagnostic events for consumption by dev overlay,
 * CLI, and CI reporter. */
pub struct DiagnosticsChannel {
    events: VecDeque<DiagnosticsEvent>,
    max_buffer: usize,
    subscriber_count: u32,
}

impl Default for DiagnosticsChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl DiagnosticsChannel {
    pub fn new() -> Self {
        Self {
            events: VecDeque::new(),
            max_buffer: 1000,
            subscriber_count: 0,
        }
    }

    /** Set the maximum number of events to buffer. Oldest events are dropped. */
    pub fn set_max_buffer(&mut self, max: usize) {
        self.max_buffer = max;
        while self.events.len() > self.max_buffer {
            self.events.pop_front();
        }
    }

    /** Publish a diagnostic event to the channel. */
    pub fn publish(&mut self, event: DiagnosticsEvent) {
        if self.events.len() >= self.max_buffer {
            self.events.pop_front();
        }
        self.events.push_back(event);
    }

    /** Publish a NATIVE_NO_BACKEND event. */
    pub fn native_no_backend(&mut self, module: &str, platform: Option<&str>) {
        self.publish(DiagnosticsEvent {
            id: format!("evt-{}", current_timestamp_ms()),
            event_type: DiagnosticsEventType::NativeNoBackend,
            level: DiagnosticsLevel::Error,
            message: format!("No native backend available for module '{}'", module),
            timestamp_ms: current_timestamp_ms(),
            context: serde_json::json!({"module": module}),
            platform: platform.map(|s| s.to_string()),
        });
    }

    /** Publish a NATIVE_CAPABILITY_UNAVAILABLE event. */
    pub fn native_capability_unavailable(&mut self, capability: &str, reason: &str) {
        self.publish(DiagnosticsEvent {
            id: format!("evt-{}", current_timestamp_ms()),
            event_type: DiagnosticsEventType::NativeCapabilityUnavailable,
            level: DiagnosticsLevel::Warn,
            message: format!("Capability '{}' unavailable: {}", capability, reason),
            timestamp_ms: current_timestamp_ms(),
            context: serde_json::json!({"capability": capability, "reason": reason}),
            platform: None,
        });
    }

    /** Publish a NATIVE_CAPABILITY_DEGRADED event. */
    pub fn native_capability_degraded(&mut self, capability: &str, fallback: Option<&str>) {
        self.publish(DiagnosticsEvent {
            id: format!("evt-{}", current_timestamp_ms()),
            event_type: DiagnosticsEventType::NativeCapabilityDegraded,
            level: DiagnosticsLevel::Warn,
            message: format!(
                "Capability '{}' degraded{}",
                capability,
                fallback.map(|f| format!(", using fallback '{}'", f)).unwrap_or_default()
            ),
            timestamp_ms: current_timestamp_ms(),
            context: serde_json::json!({"capability": capability, "fallback": fallback}),
            platform: None,
        });
    }

    /** Publish a NATIVE_MODULE_UNAVAILABLE event. */
    pub fn native_module_unavailable(&mut self, module: &str) {
        self.publish(DiagnosticsEvent {
            id: format!("evt-{}", current_timestamp_ms()),
            event_type: DiagnosticsEventType::NativeModuleUnavailable,
            level: DiagnosticsLevel::Error,
            message: format!("Native module '{}' unavailable", module),
            timestamp_ms: current_timestamp_ms(),
            context: serde_json::json!({"module": module}),
            platform: None,
        });
    }

    /** Publish a NATIVE_STYLE_UNSUPPORTED event. */
    pub fn native_style_unsupported(&mut self, style: &str, element: &str) {
        self.publish(DiagnosticsEvent {
            id: format!("evt-{}", current_timestamp_ms()),
            event_type: DiagnosticsEventType::NativeStyleUnsupported,
            level: DiagnosticsLevel::Warn,
            message: format!("Style '{}' unsupported for element '{}'", style, element),
            timestamp_ms: current_timestamp_ms(),
            context: serde_json::json!({"style": style, "element": element}),
            platform: None,
        });
    }

    /** Publish a NATIVE_COMMIT_FAILED event. */
    pub fn native_commit_failed(&mut self, commit_id: &str, error: &str) {
        self.publish(DiagnosticsEvent {
            id: format!("evt-{}", current_timestamp_ms()),
            event_type: DiagnosticsEventType::NativeCommitFailed,
            level: DiagnosticsLevel::Error,
            message: format!("Native commit '{}' failed: {}", commit_id, error),
            timestamp_ms: current_timestamp_ms(),
            context: serde_json::json!({"commit_id": commit_id, "error": error}),
            platform: None,
        });
    }

    /** Get all events in the channel. */
    pub fn events(&self) -> &[DiagnosticsEvent] {
        self.events.as_slices().0
    }

    /** Get events of a specific type. */
    pub fn events_by_type(&self, event_type: &DiagnosticsEventType) -> Vec<&DiagnosticsEvent> {
        self.events.iter().filter(|e| &e.event_type == event_type).collect()
    }

    /** Get events at or above a severity level. */
    pub fn events_at_level_or_above(&self, level: DiagnosticsLevel) -> Vec<&DiagnosticsEvent> {
        let level_rank = |l: DiagnosticsLevel| match l {
            DiagnosticsLevel::Info => 0,
            DiagnosticsLevel::Warn => 1,
            DiagnosticsLevel::Error => 2,
            DiagnosticsLevel::Fatal => 3,
        };
        let min = level_rank(level);
        self.events
            .iter()
            .filter(|e| level_rank(e.level) >= min)
            .collect()
    }

    /** Clear all events. */
    pub fn clear(&mut self) {
        self.events.clear();
    }

    /** Number of buffered events. */
    pub fn len(&self) -> usize {
        self.events.len()
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /** Number of subscribers. */
    pub fn subscriber_count(&self) -> u32 {
        self.subscriber_count
    }

    /** Register a subscriber (dev overlay, CLI, CI reporter). */
    pub fn subscribe(&mut self) -> u32 {
        self.subscriber_count += 1;
        self.subscriber_count
    }

    /** Unregister a subscriber. */
    pub fn unsubscribe(&mut self) {
        if self.subscriber_count > 0 {
            self.subscriber_count -= 1;
        }
    }
}

/** Aggregated performance diagnostics for the host. */
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerformanceDiagnostics {
    pub commit_count: u64,
    pub avg_commit_ms: f64,
    pub max_commit_ms: u64,
    pub node_count: u64,
    pub layout_count: u64,
    pub avg_layout_ms: f64,
    pub memory_kb: u64,
    pub timestamp_ms: u64,
}

impl Default for PerformanceDiagnostics {
    fn default() -> Self {
        Self {
            commit_count: 0,
            avg_commit_ms: 0.0,
            max_commit_ms: 0,
            node_count: 0,
            layout_count: 0,
            avg_layout_ms: 0.0,
            memory_kb: 0,
            timestamp_ms: current_timestamp_ms(),
        }
    }
}

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn publish_and_retrieve_events() {
        let mut channel = DiagnosticsChannel::new();
        channel.native_no_backend("camera", Some("android"));

        assert_eq!(channel.len(), 1);
        let events = channel.events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, DiagnosticsEventType::NativeNoBackend);
        assert_eq!(events[0].level, DiagnosticsLevel::Error);
        assert_eq!(events[0].platform, Some("android".to_string()));
    }

    #[test]
    fn filter_by_type() {
        let mut channel = DiagnosticsChannel::new();
        channel.native_no_backend("camera", None);
        channel.native_capability_unavailable("geolocation", "no gps");
        channel.native_capability_degraded("camera", Some("mock"));

        let caps_unavailable = channel.events_by_type(&DiagnosticsEventType::NativeCapabilityUnavailable);
        assert_eq!(caps_unavailable.len(), 1);

        let degraded = channel.events_by_type(&DiagnosticsEventType::NativeCapabilityDegraded);
        assert_eq!(degraded.len(), 1);
    }

    #[test]
    fn filter_by_level() {
        let mut channel = DiagnosticsChannel::new();
        channel.native_no_backend("camera", None); // Error
        channel.native_style_unsupported("border-box", "na-view"); // Warn
        channel.native_capability_degraded("camera", None); // Warn

        let errors = channel.events_at_level_or_above(DiagnosticsLevel::Error);
        assert_eq!(errors.len(), 1);

        let warns_or_above = channel.events_at_level_or_above(DiagnosticsLevel::Warn);
        assert_eq!(warns_or_above.len(), 3);
    }

    #[test]
    fn buffer_evicts_oldest() {
        let mut channel = DiagnosticsChannel::new();
        channel.set_max_buffer(3);
        channel.native_no_backend("a", None);
        channel.native_no_backend("b", None);
        channel.native_no_backend("c", None);
        channel.native_no_backend("d", None); // should evict "a"

        assert_eq!(channel.len(), 3);
        let ids: Vec<&str> = channel.events().iter().map(|e| e.id.as_str()).collect();
        assert!(!ids.iter().any(|id| *id == "evt-0")); // oldest evicted
    }

    #[test]
    fn performance_diagnostics_default() {
        let diag = PerformanceDiagnostics::default();
        assert_eq!(diag.commit_count, 0);
        assert_eq!(diag.memory_kb, 0);
    }

    #[test]
    fn subscriber_count() {
        let mut channel = DiagnosticsChannel::new();
        assert_eq!(channel.subscriber_count(), 0);
        channel.subscribe();
        channel.subscribe();
        assert_eq!(channel.subscriber_count(), 2);
        channel.unsubscribe();
        assert_eq!(channel.subscriber_count(), 1);
    }
}
