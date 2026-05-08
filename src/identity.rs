//! Identity backend — what issues + manages the workload SVIDs that
//! the data-plane proxies use for mTLS.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IdentitySpec {
    #[serde(default)]
    pub kind: IdentityKind,

    /// Trust-domain name. SPIFFE-IDs in this mesh take the form
    /// `spiffe://<trust_domain>/ns/<ns>/sa/<sa>`.
    pub trust_domain: String,

    /// Issuer driver — concrete identity backend behind `kind:
    /// spiffe`.
    #[serde(default)]
    pub issuer: IssuerKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum IdentityKind {
    #[default]
    Spiffe,
    AuthentikMtls,
    PkiOnly,
    Off,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum IssuerKind {
    /// Vendor upstream SPIRE (the M1 default — see
    /// `theory/MESH-EXECUTION-PLAN.md`).
    #[default]
    Spire,
    /// Future Rust-from-scratch SPIRE-equivalent (M5+).
    Shikiriya,
    /// Use an external pre-existing trust anchor. Renderer skips
    /// SPIRE chart emission.
    External,
}
