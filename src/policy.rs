//! Per-edge policy + observability slots.

use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum MtlsPosture {
    /// Plaintext only.
    Off,
    /// Accept either; useful during migration.
    Permissive,
    /// All edges require mTLS via SPIFFE SVIDs.
    #[default]
    Strict,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CircuitBreakerSpec {
    #[serde(default = "default_cb_max_requests")]
    pub max_requests: u32,
    #[serde(default = "default_cb_max_failures")]
    pub max_failures: u32,
    #[serde(default = "default_cb_reset", with = "humantime_serde_compat")]
    pub reset: Duration,
}

impl Default for CircuitBreakerSpec {
    fn default() -> Self {
        Self {
            max_requests: default_cb_max_requests(),
            max_failures: default_cb_max_failures(),
            reset: default_cb_reset(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RetrySpec {
    #[serde(default = "default_retries")]
    pub retries: u32,
    #[serde(default = "default_retry_budget")]
    pub retry_budget: f32,
}

impl Default for RetrySpec {
    fn default() -> Self {
        Self {
            retries: default_retries(),
            retry_budget: default_retry_budget(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TimeoutSpec {
    #[serde(default = "default_timeout", with = "humantime_serde_compat")]
    pub timeout: Duration,
    #[serde(default = "default_slow_timeout", with = "humantime_serde_compat")]
    pub slow_timeout: Duration,
}

impl Default for TimeoutSpec {
    fn default() -> Self {
        Self {
            timeout: default_timeout(),
            slow_timeout: default_slow_timeout(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ObservabilitySpec {
    #[serde(default)]
    pub traces: TraceFormat,
    #[serde(default = "default_metrics_format")]
    pub metrics: String,
    #[serde(default = "default_logs_format")]
    pub logs: String,
}

impl Default for ObservabilitySpec {
    fn default() -> Self {
        Self {
            traces: TraceFormat::default(),
            metrics: default_metrics_format(),
            logs: default_logs_format(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum TraceFormat {
    /// W3C `traceparent` propagation, OTel sink.
    #[default]
    Otel,
    /// Zipkin-shaped headers, b3 propagation.
    Zipkin,
    /// No tracing.
    Off,
}

// ── default fns ──────────────────────────────────────────────────

#[must_use]
pub fn default_retries() -> u32 {
    3
}
#[must_use]
pub fn default_retry_budget() -> f32 {
    0.2
}
#[must_use]
pub fn default_timeout() -> Duration {
    Duration::from_secs(30)
}
#[must_use]
pub fn default_slow_timeout() -> Duration {
    Duration::from_secs(300)
}
#[must_use]
pub fn default_cb_max_requests() -> u32 {
    100
}
#[must_use]
pub fn default_cb_max_failures() -> u32 {
    10
}
#[must_use]
pub fn default_cb_reset() -> Duration {
    Duration::from_secs(30)
}
#[must_use]
pub fn default_metrics_format() -> String {
    "prometheus".into()
}
#[must_use]
pub fn default_logs_format() -> String {
    "json".into()
}

/// humantime-shaped duration serde adapter.
pub mod humantime_serde_compat {
    use std::time::Duration;

    use serde::{Deserialize, Deserializer, Serializer, de::Error};

    pub fn serialize<S: Serializer>(d: &Duration, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(&format_duration(*d))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Duration, D::Error> {
        let raw = String::deserialize(d)?;
        parse_duration(&raw).map_err(D::Error::custom)
    }

    fn format_duration(d: Duration) -> String {
        let secs = d.as_secs();
        if secs % 3600 == 0 && secs > 0 {
            format!("{}h", secs / 3600)
        } else if secs % 60 == 0 && secs > 0 {
            format!("{}m", secs / 60)
        } else {
            format!("{secs}s")
        }
    }

    fn parse_duration(s: &str) -> Result<Duration, String> {
        let s = s.trim();
        if let Some(rest) = s.strip_suffix("ms") {
            return rest
                .parse::<u64>()
                .map(Duration::from_millis)
                .map_err(|e| e.to_string());
        }
        let (num, unit) = s.split_at(s.len() - 1);
        let n: u64 = num
            .parse()
            .map_err(|e: std::num::ParseIntError| e.to_string())?;
        match unit {
            "s" => Ok(Duration::from_secs(n)),
            "m" => Ok(Duration::from_secs(n * 60)),
            "h" => Ok(Duration::from_secs(n * 3600)),
            other => Err(format!("unknown duration unit: {other}")),
        }
    }
}
