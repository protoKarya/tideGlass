// SPDX-License-Identifier: AGPL-3.0-or-later

//! Capability-based primal discovery — no hardcoded names, ports, or URLs.

use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

/// A capability advertised by a primal via JSON-RPC `capabilities.list`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    /// JSON-RPC method name (domain.operation format).
    pub method: Arc<str>,
    /// Semantic version of this capability implementation.
    pub version: Arc<str>,
    /// Human-readable description of what the capability provides.
    pub description: Arc<str>,
}

/// Self-knowledge: what a primal advertises about itself and its capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrimalIdentity {
    /// Unique primal name reported at registration time.
    pub name: Arc<str>,
    /// Primal software version string.
    pub version: Arc<str>,
    /// Capabilities this primal implements.
    pub capabilities: Vec<Capability>,
}

/// Runtime capability registry — discovers primals by capability, not by name.
pub trait CapabilityRegistry: Send + Sync {
    /// Finds a primal that provides the given JSON-RPC method.
    fn find_provider(&self, method: &str) -> Option<PrimalIdentity>;

    /// Lists all known capabilities across registered primals.
    fn list_capabilities(&self) -> Vec<Capability>;

    /// Registers a primal's capabilities (from a `capabilities.list` response).
    fn register(&mut self, identity: PrimalIdentity);
}

/// In-memory capability registry suitable for tests and single-process deployments.
#[derive(Debug, Default)]
pub struct InMemoryCapabilityRegistry {
    primals: Vec<PrimalIdentity>,
    method_index: HashMap<Arc<str>, usize>,
}

impl InMemoryCapabilityRegistry {
    /// Creates an empty in-memory registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

impl CapabilityRegistry for InMemoryCapabilityRegistry {
    fn find_provider(&self, method: &str) -> Option<PrimalIdentity> {
        self.method_index
            .get(method)
            .and_then(|&index| self.primals.get(index))
            .cloned()
    }

    fn list_capabilities(&self) -> Vec<Capability> {
        self.primals
            .iter()
            .flat_map(|identity| identity.capabilities.clone())
            .collect()
    }

    fn register(&mut self, identity: PrimalIdentity) {
        let index = self.primals.len();
        for capability in &identity.capabilities {
            self.method_index
                .insert(Arc::clone(&capability.method), index);
        }
        self.primals.push(identity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_capability(method: &str) -> Capability {
        Capability {
            method: Arc::from(method),
            version: Arc::from("1.0.0"),
            description: Arc::from("test capability"),
        }
    }

    fn sample_identity(name: &str, methods: &[&str]) -> PrimalIdentity {
        PrimalIdentity {
            name: Arc::from(name),
            version: Arc::from("0.1.0"),
            capabilities: methods
                .iter()
                .map(|method| sample_capability(method))
                .collect(),
        }
    }

    #[test]
    fn register_and_find_provider_by_method() {
        let mut registry = InMemoryCapabilityRegistry::new();
        let identity = sample_identity("science-node", &["science.rges_screen"]);
        registry.register(identity);

        let found = registry
            .find_provider("science.rges_screen")
            .expect("provider registered");
        assert_eq!(found.name.as_ref(), "science-node");
    }

    #[test]
    fn find_provider_returns_none_for_unknown_method() {
        let registry = InMemoryCapabilityRegistry::new();
        assert!(registry.find_provider("science.unknown").is_none());
    }

    #[test]
    fn list_capabilities_aggregates_all_primals() {
        let mut registry = InMemoryCapabilityRegistry::new();
        registry.register(sample_identity("a", &["science.rges_screen"]));
        registry.register(sample_identity("b", &["science.nf_score", "health.check"]));

        let caps = registry.list_capabilities();
        assert_eq!(caps.len(), 3);
    }

    #[test]
    fn later_registration_overwrites_method_index() {
        let mut registry = InMemoryCapabilityRegistry::new();
        registry.register(sample_identity("first", &["science.rges_screen"]));
        registry.register(sample_identity("second", &["science.rges_screen"]));

        let found = registry
            .find_provider("science.rges_screen")
            .expect("provider");
        assert_eq!(found.name.as_ref(), "second");
    }

    #[test]
    fn capability_round_trip_json() {
        let cap = sample_capability("health.liveness");
        let json = serde_json::to_string(&cap).expect("serialize capability");
        let back: Capability = serde_json::from_str(&json).expect("deserialize capability");
        assert_eq!(back.method.as_ref(), "health.liveness");
    }
}
