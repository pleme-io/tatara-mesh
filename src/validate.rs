//! Type-level invariants — fail-closed gate before any renderer
//! consumes a `MeshSpec`. Each rule is tested independently.

use crate::{IdentityKind, MeshSpec, MtlsPosture};

#[derive(Debug, thiserror::Error, Clone, PartialEq)]
pub enum ValidateError {
    #[error("name must be non-empty")]
    EmptyName,
    #[error("aplicacao must be non-empty")]
    EmptyAplicacao,
    #[error("identity.trust_domain must be non-empty")]
    EmptyTrustDomain,
    #[error("identity.trust_domain '{0}' is not a valid SPIFFE trust-domain (no slashes, no scheme)")]
    BadTrustDomain(String),
    #[error("retry_budget must be in [0.0, 1.0], got {0}")]
    RetryBudgetOutOfRange(f32),
    #[error(
        "mtls=strict requires identity.kind != off (got identity.kind={0:?}); \
         can't enforce mTLS without a workload identity backend"
    )]
    StrictMtlsWithoutIdentity(IdentityKind),
    #[error(
        "circuit_breaker.max_failures must be <= max_requests (got max_failures={0}, \
         max_requests={1})"
    )]
    CbFailuresExceedRequests(u32, u32),
    #[error("timeout must be < slow_timeout (got timeout={0}s, slow_timeout={1}s)")]
    SlowTimeoutInverted(u64, u64),
}

/// Run every invariant, accumulating all violations. Returns `Ok(())`
/// only when the spec is fully valid.
pub fn all(spec: &MeshSpec) -> std::result::Result<(), Vec<ValidateError>> {
    let mut errors = Vec::new();

    if spec.name.trim().is_empty() {
        errors.push(ValidateError::EmptyName);
    }
    if spec.aplicacao.trim().is_empty() {
        errors.push(ValidateError::EmptyAplicacao);
    }

    let td = &spec.identity.trust_domain;
    if td.trim().is_empty() {
        errors.push(ValidateError::EmptyTrustDomain);
    } else if td.contains('/') || td.contains("://") {
        errors.push(ValidateError::BadTrustDomain(td.clone()));
    }

    let rb = spec.defaults.retry_budget;
    if !(0.0..=1.0).contains(&rb) || rb.is_nan() {
        errors.push(ValidateError::RetryBudgetOutOfRange(rb));
    }

    if spec.mtls == MtlsPosture::Strict && spec.identity.kind == IdentityKind::Off {
        errors.push(ValidateError::StrictMtlsWithoutIdentity(spec.identity.kind));
    }

    let cb = &spec.defaults.circuit_breaker;
    if cb.max_failures > cb.max_requests {
        errors.push(ValidateError::CbFailuresExceedRequests(
            cb.max_failures,
            cb.max_requests,
        ));
    }

    let t = spec.defaults.timeout;
    let st = spec.defaults.slow_timeout;
    if t >= st {
        errors.push(ValidateError::SlowTimeoutInverted(t.as_secs(), st.as_secs()));
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CircuitBreakerSpec, DataPlaneSpec, GatewaySource, IdentitySpec, IssuerKind, MeshSpec,
        ObservabilitySpec, PolicyDefaults, PolicySource,
    };
    use std::time::Duration;

    fn happy() -> MeshSpec {
        MeshSpec {
            name: "openclaw-mesh".into(),
            aplicacao: "openclaw".into(),
            identity: IdentitySpec {
                kind: IdentityKind::Spiffe,
                trust_domain: "pleme.io".into(),
                issuer: IssuerKind::Spire,
            },
            data_plane: DataPlaneSpec::default(),
            mtls: MtlsPosture::Strict,
            defaults: PolicyDefaults::default(),
            observability: ObservabilitySpec::default(),
            policy: PolicySource::Contratos,
            gateway: GatewaySource::Entrada,
            saguao: false,
        }
    }

    impl Default for DataPlaneSpec {
        fn default() -> Self {
            Self {
                proxy: crate::ProxyKind::Aresta,
                sidecar_mode: crate::SidecarMode::Auto,
                l7: crate::L7Kind::Hanabi,
            }
        }
    }

    #[test]
    fn happy_passes() {
        happy().validate().unwrap();
    }

    #[test]
    fn empty_name_fails() {
        let mut s = happy();
        s.name = "".into();
        let errs = s.validate().unwrap_err();
        assert!(errs.contains(&ValidateError::EmptyName));
    }

    #[test]
    fn slash_in_trust_domain_fails() {
        let mut s = happy();
        s.identity.trust_domain = "spiffe://pleme.io".into();
        let errs = s.validate().unwrap_err();
        assert!(errs.iter().any(|e| matches!(e, ValidateError::BadTrustDomain(_))));
    }

    #[test]
    fn retry_budget_above_one_fails() {
        let mut s = happy();
        s.defaults.retry_budget = 1.5;
        let errs = s.validate().unwrap_err();
        assert!(
            errs.iter()
                .any(|e| matches!(e, ValidateError::RetryBudgetOutOfRange(_)))
        );
    }

    #[test]
    fn strict_mtls_without_identity_fails() {
        let mut s = happy();
        s.identity.kind = IdentityKind::Off;
        s.mtls = MtlsPosture::Strict;
        let errs = s.validate().unwrap_err();
        assert!(
            errs.iter()
                .any(|e| matches!(e, ValidateError::StrictMtlsWithoutIdentity(_)))
        );
    }

    #[test]
    fn circuit_breaker_failures_exceeds_requests_fails() {
        let mut s = happy();
        s.defaults.circuit_breaker = CircuitBreakerSpec {
            max_requests: 5,
            max_failures: 50,
            reset: Duration::from_secs(30),
        };
        let errs = s.validate().unwrap_err();
        assert!(
            errs.iter()
                .any(|e| matches!(e, ValidateError::CbFailuresExceedRequests(50, 5)))
        );
    }

    #[test]
    fn timeout_must_be_less_than_slow_timeout() {
        let mut s = happy();
        s.defaults.timeout = Duration::from_secs(600);
        s.defaults.slow_timeout = Duration::from_secs(300);
        let errs = s.validate().unwrap_err();
        assert!(
            errs.iter()
                .any(|e| matches!(e, ValidateError::SlowTimeoutInverted(_, _)))
        );
    }
}
