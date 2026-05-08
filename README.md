# tatara-mesh

Typed `(defmesh …)` primitive for the pleme-io service mesh
([`theory/MESH.md`](https://github.com/pleme-io/theory/blob/main/MESH.md)).

This crate ships:

- **`MeshSpec`** — top-level Rust struct mirroring the form in
  `MESH.md` §VI (identity, data plane, mTLS posture, retry/timeout/CB
  defaults, observability, policy/gateway sources, saguão
  integration).
- **Validation** — fail-closed type-level invariants (retry-budget
  ∈ [0,1], strict mTLS requires identity ≠ off, slow-timeout >
  timeout, CB max-failures ≤ max-requests, trust-domain shape).
- **YAML round-trip** — `serde_yaml_ng`. Lisp-side authoring via
  `#[derive(TataraDomain)]` lands when the proc-macro stabilizes
  fleet-wide (per `theory/MESH-EXECUTION-PLAN.md` open question 6).

Sprint **M4.1** of [`MESH-EXECUTION-PLAN.md`](https://github.com/pleme-io/theory/blob/main/MESH-EXECUTION-PLAN.md).

## Usage

```rust
use tatara_mesh::MeshSpec;

let yaml = std::fs::read_to_string("openclaw-mesh.yaml")?;
let spec: MeshSpec = serde_yaml_ng::from_str(&yaml)?;
spec.validate().map_err(|errs| {
    for e in errs { eprintln!("invalid: {e}"); }
    std::process::exit(1);
})?;
// arch-synthesizer dispatches `spec` to the chosen renderer backend.
```

## Worked example

See [`examples/openclaw-mesh.yaml`](examples/openclaw-mesh.yaml) — the
canonical openclaw mesh, the demo Aplicacao through M1–M4.

## Consumers

- **`arch-synthesizer`** — runs `validate()` then dispatches to per-
  backend renderers (k8s+sidecar, Linkerd, Istio, Cilium ServiceMesh,
  native-no-mesh).
- **`tatara-lisp`** — when TataraDomain stabilizes, parses
  `(defmesh …)` author surface → `MeshSpec`.

## License

Dual MIT OR Apache-2.0.
