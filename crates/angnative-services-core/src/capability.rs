/**
 * capability.rs — Capability system (spec §5.4, tasks C1–C5).
 *
 * Provides a capability registry interface and per-platform capability
 * declarations (data only — resolution is identity in M1). Includes a
 * resolver with fallback chain and degradation diagnostics.
 *
 * "Not yet" is allowed; "silently absent" is not — every capability gap is
 * explicit and written down via the diagnostics channel.
 */

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/** A capability identifier, e.g. "camera", "geolocation", "offline-sync". */
pub type CapabilityId = String;

/** A capability declaration from a plugin (data only). */
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityDeclaration {
    pub id: CapabilityId,
    pub version: String,
    /** Human-readable description. */
    pub description: String,
    /** Platforms this capability is declared for (empty = all). */
    pub platforms: Vec<String>,
    /** Minimum OS version required, if any. */
    pub min_os_version: Option<String>,
}

/** Runtime status of a capability. */
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CapabilityStatus {
    Available,
    Degraded,
    Unavailable,
    NotDeclared,
}

/** A diagnostic produced when a capability cannot be fully satisfied. */
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DegradationDiagnostic {
    pub capability_id: CapabilityId,
    pub reason: String,
    pub suggested_fallback: Option<String>,
}

/** The capability registry interface. Plugins declare capabilities here. */
pub trait CapabilityRegistry: Send + Sync {
    /** Declare a capability provided by this plugin. */
    fn declare(&mut self, decl: CapabilityDeclaration);

    /** Check if a capability is declared by any registered plugin. */
    fn is_declared(&self, id: &str) -> bool;

    /** Get the declaration for a capability, if declared. */
    fn declaration(&self, id: &str) -> Option<&CapabilityDeclaration>;
}

/** The capability resolver — resolves capabilities with fallback chain. */
pub struct CapabilityResolver {
    declarations: HashMap<CapabilityId, CapabilityDeclaration>,
    /** Fallback chain: capability id -> fallback capability id. */
    fallbacks: HashMap<CapabilityId, Vec<CapabilityId>>,
    /** Diagnostics stream for degradation events. */
    diagnostics: Vec<DegradationDiagnostic>,
}

impl Default for CapabilityResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl CapabilityResolver {
    pub fn new() -> Self {
        Self {
            declarations: HashMap::new(),
            fallbacks: HashMap::new(),
            diagnostics: Vec::new(),
        }
    }

    /** Register a capability declaration. */
    pub fn declare(&mut self, decl: CapabilityDeclaration) {
        self.declarations.insert(decl.id.clone(), decl);
    }

    /** Register a fallback: if `primary` is unavailable, try `fallback`. */
    pub fn add_fallback(&mut self, primary: CapabilityId, fallback: CapabilityId) {
        self.fallbacks.entry(primary).or_default().push(fallback);
    }

    /** Resolve a capability, following the fallback chain.
     *
     * Returns Ok(CapabilityStatus::Available) if the capability (or a
     * fallback) is available, or a DegradationDiagnostic if it must degrade.
     * Never silently absent — every gap produces a diagnostic.
     */
    pub fn resolve(&mut self, id: &str) -> Result<CapabilityStatus, DegradationDiagnostic> {
        if self.declarations.contains_key(id) {
            return Ok(CapabilityStatus::Available);
        }

        // Try fallbacks.
        if let Some(fallbacks) = self.fallbacks.get(id) {
            for fallback in fallbacks {
                if self.declarations.contains_key(fallback) {
                    let diag = DegradationDiagnostic {
                        capability_id: id.to_string(),
                        reason: format!("Primary capability '{}' unavailable, using fallback '{}'", id, fallback),
                        suggested_fallback: Some(fallback.clone()),
                    };
                    self.diagnostics.push(diag.clone());
                    return Ok(CapabilityStatus::Degraded);
                }
            }
        }

        // No capability and no fallback — degrade to unavailable with diagnostic.
        let diag = DegradationDiagnostic {
            capability_id: id.to_string(),
            reason: format!("Capability '{}' is not declared by any plugin", id),
            suggested_fallback: None,
        };
        self.diagnostics.push(diag.clone());
        Err(diag)
    }

    /** Get all degradation diagnostics recorded so far. */
    pub fn diagnostics(&self) -> &[DegradationDiagnostic] {
        &self.diagnostics
    }

    /** Clear the diagnostics stream. */
    pub fn clear_diagnostics(&mut self) {
        self.diagnostics.clear();
    }

    /** List all declared capability IDs. */
    pub fn declared_capabilities(&self) -> Vec<&str> {
        self.declarations.keys().map(|s| s.as_str()).collect()
    }
}

impl CapabilityRegistry for CapabilityResolver {
    fn declare(&mut self, decl: CapabilityDeclaration) {
        self.declare(decl);
    }

    fn is_declared(&self, id: &str) -> bool {
        self.declarations.contains_key(id)
    }

    fn declaration(&self, id: &str) -> Option<&CapabilityDeclaration> {
        self.declarations.get(id)
    }
}

/** A concrete capability instance provided at runtime. */
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capability {
    pub id: CapabilityId,
    pub status: CapabilityStatus,
    pub config: serde_json::Value,
}

impl Capability {
    pub fn available(id: impl Into<CapabilityId>) -> Self {
        Self {
            id: id.into(),
            status: CapabilityStatus::Available,
            config: serde_json::Value::Null,
        }
    }

    pub fn degraded(id: impl Into<CapabilityId>, reason: String) -> Self {
        Self {
            id: id.into(),
            status: CapabilityStatus::Degraded,
            config: serde_json::json!({"reason": reason}),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_available_capability() {
        let mut resolver = CapabilityResolver::new();
        resolver.declare(CapabilityDeclaration {
            id: "camera".to_string(),
            version: "1.0".to_string(),
            description: "Camera access".to_string(),
            platforms: vec!["android".to_string(), "ios".to_string()],
            min_os_version: None,
        });

        let status = resolver.resolve("camera").unwrap();
        assert_eq!(status, CapabilityStatus::Available);
        assert!(resolver.diagnostics().is_empty());
    }

    #[test]
    fn resolve_degrades_to_fallback() {
        let mut resolver = CapabilityResolver::new();
        resolver.declare(CapabilityDeclaration {
            id: "geolocation".to_string(),
            version: "1.0".to_string(),
            description: "GPS location".to_string(),
            platforms: vec![],
            min_os_version: None,
        });
        // Primary not declared, fallback is.
        resolver.add_fallback("gps".to_string(), "geolocation".to_string());

        let status = resolver.resolve("gps").unwrap();
        assert_eq!(status, CapabilityStatus::Degraded);
        assert_eq!(resolver.diagnostics().len(), 1);
        assert_eq!(resolver.diagnostics()[0].capability_id, "gps");
    }

    #[test]
    fn resolve_unavailable_without_fallback() {
        let mut resolver = CapabilityResolver::new();
        let diag = resolver.resolve("nonexistent").unwrap_err();
        assert_eq!(diag.capability_id, "nonexistent");
        assert_eq!(resolver.diagnostics().len(), 1);
    }

    #[test]
    fn declared_capabilities_list() {
        let mut resolver = CapabilityResolver::new();
        resolver.declare(CapabilityDeclaration {
            id: "camera".to_string(),
            version: "1.0".to_string(),
            description: "Camera".to_string(),
            platforms: vec![],
            min_os_version: None,
        });
        resolver.declare(CapabilityDeclaration {
            id: "geolocation".to_string(),
            version: "1.0".to_string(),
            description: "Geo".to_string(),
            platforms: vec![],
            min_os_version: None,
        });

        let caps = resolver.declared_capabilities();
        assert_eq!(caps.len(), 2);
        assert!(caps.contains(&"camera"));
        assert!(caps.contains(&"geolocation"));
    }
}
