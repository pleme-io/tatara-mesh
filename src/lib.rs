//! tatara-mesh — typed `(defmesh …)` primitive.
//!
//! Sprint **M4.1** of `theory/MESH-EXECUTION-PLAN.md`.
//!
//! # What this crate is
//!
//! The Rust types + validation that mechanically capture the
//! `(defmesh …)` form documented in [`theory/MESH.md` §VI](https://github.com/pleme-io/theory/blob/main/MESH.md).
//! Consumers:
//!
//! - **`arch-synthesizer`** imports `MeshSpec`, runs
//!   `MeshSpec::validate()`, then dispatches to per-backend renderers
//!   (k8s-sidecar, Linkerd, Istio, Cilium ServiceMesh, native-no-mesh).
//! - **`tatara-lisp`** parses `(defmesh …)` author-side surface,
//!   constructs `MeshSpec`, runs validate.
//!
//! # Why a typed primitive
//!
//! Without `MeshSpec`, every Aplicacao on the substrate hand-rolls
//! its own retries / mTLS / observability. With `MeshSpec`, the
//! operator writes one form per Aplicacao and the renderer emits all
//! the K8s manifests, identity registrations, sidecar configs, and
//! observability sinks. *One rule, multiple environments.*
//!
//! # `#[derive(TataraDomain)]` is intentionally absent today
//!
//! The proc-macro lives in `tatara-lisp` and isn't yet stable across
//! the fleet (per `theory/MESH-EXECUTION-PLAN.md` open question 6).
//! When it lands, add `#[derive(TataraDomain)]` + `#[tatara(keyword
//! = "defmesh")]` to `MeshSpec` to enable Lisp authoring. Until then,
//! consumers build `MeshSpec` from YAML or directly in Rust.

#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc, clippy::module_name_repetitions)]

pub mod identity;
pub mod policy;
pub mod validate;

use std::time::Duration;

use serde::{Deserialize, Serialize};

pub use identity::{IdentityKind, IdentitySpec, IssuerKind};
pub use policy::{
    CircuitBreakerSpec, MtlsPosture, ObservabilitySpec, RetrySpec, TimeoutSpec, TraceFormat,
};
pub use validate::ValidateError;

/// `(defmesh …)` — the typed mesh spec that renders to a concrete
/// data + control plane on any of the supported backends.
///
/// Top-level shape mirrors `theory/MESH.md` §VI: each named slot
/// corresponds to one Lisp `:keyword`.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MeshSpec {
    /// Logical name (slug) of this mesh — used as a label key,
    /// resource id prefix, and SPIFFE-ID path component when relevant.
    pub name: String,

    /// Aplicacao this mesh wraps. The renderer reads the named
    /// Aplicacao's `:contratos` and `:entrada` to compose policy +
    /// gateway shape from it.
    pub aplicacao: String,

    /// Identity backend — SPIFFE issuer + trust domain.
    pub identity: IdentitySpec,

    /// Data plane (proxy + L7) configuration.
    pub data_plane: DataPlaneSpec,

    /// mTLS posture across mesh edges.
    #[serde(default)]
    pub mtls: MtlsPosture,

    /// Per-edge defaults — overridable per `:contratos[*]:policy` on
    /// the Aplicacao side.
    #[serde(default)]
    pub defaults: PolicyDefaults,

    /// Observability surface (traces / metrics / logs).
    #[serde(default)]
    pub observability: ObservabilitySpec,

    /// Inherit policy from Aplicacao `:contratos` (vs. providing
    /// explicit per-edge policy here).
    #[serde(default = "default_inherit_policy")]
    pub policy: PolicySource,

    /// Inherit gateway from Aplicacao `:entrada`.
    #[serde(default = "default_inherit_gateway")]
    pub gateway: GatewaySource,

    /// Saguão integration — when true, human edges entering via
    /// `:entrada` flow through passaporte; service edges are SPIFFE-
    /// identified separately.
    #[serde(default)]
    pub saguao: bool,
}

impl MeshSpec {
    /// Run all type-level invariants.
    pub fn validate(&self) -> std::result::Result<(), Vec<ValidateError>> {
        validate::all(self)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DataPlaneSpec {
    /// Proxy implementation. Today: `aresta` only. M6 adds linkerd-
    /// proxy / envoy / cilium-envoy.
    #[serde(default)]
    pub proxy: ProxyKind,

    /// Sidecar mode — auto picks ebpf when supported, else sidecar.
    #[serde(default)]
    pub sidecar_mode: SidecarMode,

    /// L7 sub-component. Today: `hanabi` only.
    #[serde(default)]
    pub l7: L7Kind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ProxyKind {
    #[default]
    Aresta,
    LinkerdProxy,
    Envoy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum SidecarMode {
    #[default]
    Auto,
    Sidecar,
    Ebpf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum L7Kind {
    #[default]
    Hanabi,
    None,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyDefaults {
    #[serde(default = "policy::default_retries")]
    pub retries: u32,
    #[serde(default = "policy::default_retry_budget")]
    pub retry_budget: f32,
    #[serde(
        default = "policy::default_timeout",
        with = "policy::humantime_serde_compat"
    )]
    pub timeout: Duration,
    #[serde(
        default = "policy::default_slow_timeout",
        with = "policy::humantime_serde_compat"
    )]
    pub slow_timeout: Duration,
    #[serde(default)]
    pub circuit_breaker: CircuitBreakerSpec,
}

impl Default for PolicyDefaults {
    fn default() -> Self {
        Self {
            retries: policy::default_retries(),
            retry_budget: policy::default_retry_budget(),
            timeout: policy::default_timeout(),
            slow_timeout: policy::default_slow_timeout(),
            circuit_breaker: CircuitBreakerSpec::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum PolicySource {
    /// Inherit from Aplicacao `:contratos` block (the recommended
    /// default — keeps single-source-of-truth).
    Contratos,
    /// Explicit per-edge policy defined on the mesh spec itself.
    Explicit,
}

fn default_inherit_policy() -> PolicySource {
    PolicySource::Contratos
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum GatewaySource {
    /// Inherit from Aplicacao `:entrada` block.
    Entrada,
    /// No public gateway (mesh is internal-only).
    None,
}

fn default_inherit_gateway() -> GatewaySource {
    GatewaySource::Entrada
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaml_round_trip_minimal() {
        let yaml = r#"
name: openclaw-mesh
aplicacao: openclaw
identity:
  kind: spiffe
  trust_domain: pleme.io
  issuer: spire
data_plane:
  proxy: aresta
  sidecar_mode: auto
  l7: hanabi
"#;
        let spec: MeshSpec = serde_yaml_ng::from_str(yaml).unwrap();
        assert_eq!(spec.name, "openclaw-mesh");
        assert_eq!(spec.aplicacao, "openclaw");
        assert_eq!(spec.identity.trust_domain, "pleme.io");
        assert_eq!(spec.data_plane.proxy, ProxyKind::Aresta);
        assert_eq!(spec.data_plane.l7, L7Kind::Hanabi);
        assert_eq!(spec.policy, PolicySource::Contratos);
        assert_eq!(spec.gateway, GatewaySource::Entrada);
        assert!(!spec.saguao);
    }

    #[test]
    fn rejects_unknown_top_level_field() {
        let yaml = r#"
name: openclaw-mesh
aplicacao: openclaw
identity: { kind: spiffe, trust_domain: pleme.io, issuer: spire }
data_plane: {}
not_a_real_slot: 1
"#;
        let err = serde_yaml_ng::from_str::<MeshSpec>(yaml).unwrap_err();
        assert!(err.to_string().contains("unknown field"));
    }

    #[test]
    fn full_form_round_trips() {
        let yaml = include_str!("../examples/openclaw-mesh.yaml");
        let spec: MeshSpec = serde_yaml_ng::from_str(yaml).unwrap();
        spec.validate().unwrap();
        let reserialized = serde_yaml_ng::to_string(&spec).unwrap();
        let spec2: MeshSpec = serde_yaml_ng::from_str(&reserialized).unwrap();
        spec2.validate().unwrap();
    }
}
