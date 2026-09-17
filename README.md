# alice-det-math

[![CI](https://github.com/ext-sakamoro/ALICE-DetMath/actions/workflows/ci.yml/badge.svg)](https://github.com/ext-sakamoro/ALICE-DetMath/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/alice-det-math.svg)](https://crates.io/crates/alice-det-math)
[![docs.rs](https://docs.rs/alice-det-math/badge.svg)](https://docs.rs/alice-det-math)

Cross-platform **bit-exact** `f32` / `f64` transcendentals — `sin`, `cos`, `exp`,
<!-- claim-test: scalar_outputs_match_recorded_hashes -->
`ln`, `atan2`, `asin`, `acos`, `tan`, `tanh`, `powf`, `cbrt`, `hypot`, … — built
from IEEE 754 basic operations only, so every function is a pure function of
its input bits on x86_64, aarch64, wasm32 and every other IEEE target.
`no_std`, no `alloc`, no `unsafe`.

The platform `libm` is not: `sin(x)` differs in the last ulp between macOS,
glibc, MSVC and wasm. A lockstep simulation that calls it diverges across
peers; a signed-distance field evaluated on two machines disagrees about which
side of a surface a point is on. This crate is the transcendental layer of
[alice-physics](https://crates.io/crates/alice-physics) (128-bit deterministic
physics) and [alice-sdf](https://crates.io/crates/alice-sdf) (SDF geometry),
extracted so both — and anything else — evaluate the same law.

## Features

- **Scalar** `f32` kernels (Cephes `sinf` / `cosf` / `expf`, musl `logf`) and
  `f64` kernels (fdlibm `atan` / `atan2` / `asin` / `acos` / `exp` / `log`),
  ≤ 1–2 ulp of correctly rounded over the documented domains
- **SIMD** (`simd` feature): `wide::f32x8` versions of `sin` / `cos` /
  `sin_cos` / `exp` / `ln` / `round` / `sqrt`, lane-for-lane bit-identical to
  the scalar functions — same constants, same operation order, no `mul_add`
- **`round` / `sqrt` without libm**: musl's `x + 2^23 − 2^23` rounding and a
  correctly rounded software square root, so the crate works on bare-metal
  targets and returns the same bits there
- **Canonical NaN**: a NaN input never propagates through arithmetic (whose
  payload / sign rules are platform-specific); every function returns the
  canonical NaN or passes the input bits through unchanged
- **Pinned**: `tests/golden.rs` holds a SHA-256 of every function over a fixed
  input grid; CI reproduces it on macOS ARM / Intel, Linux x86 / ARM, Windows,
  wasm32 (wasmtime), with the SSE2-only SIMD fallback, with AVX2 + FMA forced
  on, and with `no_std` (software `sqrt`)

## Quick start

```toml
[dependencies]
alice-det-math = "0.1"
# alice-det-math = { version = "0.1", features = ["simd"] }
# alice-det-math = { version = "0.1", default-features = false }   # no_std
```

```rust
use alice_det_math::{atan2, exp, sin_cos};

let (s, c) = sin_cos(1.0_f32);
assert_eq!(s.to_bits(), 0x3f57_6aa5); // the same bits on every platform
assert_eq!(c.to_bits(), 0x3f0a_5140);
assert_eq!(exp(1.0_f32).to_bits(), 0x402d_f854);
let a = atan2(0.0_f32, -1.0); // π, fdlibm special cases
assert_eq!(a, core::f32::consts::PI);
```

With `simd`:

```rust
use alice_det_math::simd;
use wide::f32x8;

let x = f32x8::new([0.0, 0.5, 1.0, 2.0, 3.0, -1.0, 100.0, 1.0e-3]);
let (s, c) = simd::sin_cos(x);
for (i, xi) in x.to_array().into_iter().enumerate() {
    let (ss, sc) = alice_det_math::sin_cos(xi);
    assert_eq!(s.to_array()[i].to_bits(), ss.to_bits());
    assert_eq!(c.to_array()[i].to_bits(), sc.to_bits());
}
```

## Policy for consumers

Add the platform `libm` methods to `clippy.toml` `disallowed-methods` and run
clippy with `-D warnings`, so a stray `x.sin()` on a float is a red gate:

```toml
disallowed-methods = [
    { path = "f32::sin", reason = "platform libm; use alice_det_math::sin" },
    { path = "f32::cos", reason = "platform libm; use alice_det_math::cos" },
    { path = "f32::exp", reason = "platform libm; use alice_det_math::exp" },
    { path = "f32::ln", reason = "platform libm; use alice_det_math::ln" },
    { path = "f32::atan2", reason = "platform libm; use alice_det_math::atan2" },
    { path = "f32::mul_add", reason = "fused on FMA targets only; write a * b + c" },
]
```

## Accuracy and guarantee scope

See the crate documentation (`cargo doc --open`) for the per-function error
table and the exact scope of the bit-exactness guarantee (IEEE 754 basic
operations on the target; x87 without SSE2 and fast-math builds are outside it).

## License

MIT OR Apache-2.0. Coefficients and algorithms are from Cephes (Stephen L.
Moshier), fdlibm (Sun Microsystems) and musl; see the source for attribution.
