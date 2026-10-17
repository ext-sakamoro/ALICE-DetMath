# alice-det-math

[![CI](https://github.com/ext-sakamoro/ALICE-DetMath/actions/workflows/ci.yml/badge.svg)](https://github.com/ext-sakamoro/ALICE-DetMath/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/alice-det-math.svg)](https://crates.io/crates/alice-det-math)
[![docs.rs](https://docs.rs/alice-det-math/badge.svg)](https://docs.rs/alice-det-math)

[日本語](README_JP.md)

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

It is not a faster or more accurate `libm`: the functions are within 1–2 ulp
of correctly rounded, not correctly rounded, and they keep a fixed operation
order (no `mul_add`) rather than the fastest one for each target. Use it where two machines must produce the same bits; where
only the value matters, the platform `libm` is the better choice.

License: Apache-2.0

## Contents

- [Installation](#installation)
- [Example](#example)
- [Features](#features)
- [`SEMANTICS_ID`](#semantics_id)
- [Policy for consumers](#policy-for-consumers)
- [Accuracy and guarantee scope](#accuracy-and-guarantee-scope)
- [no_std](#no_std)
- [Minimum supported Rust version](#minimum-supported-rust-version)
- [Building and testing](#building-and-testing)
- [Related Projects](#related-projects)
- [License](#license)

## Installation

```sh
cargo add alice-det-math
cargo add alice-det-math --features simd            # f32x8 versions
cargo add alice-det-math --no-default-features      # no_std (software sqrt)
```

## Example

```rust
use alice_det_math::{atan2, exp, sin_cos, sin_cos64};

let (s, c) = sin_cos(1.0_f32);
assert_eq!(s.to_bits(), 0x3f57_6aa5); // the same bits on every platform
assert_eq!(c.to_bits(), 0x3f0a_5140);
assert_eq!(exp(1.0_f32).to_bits(), 0x402d_f854);
let a = atan2(0.0_f32, -1.0); // π, fdlibm special cases
assert_eq!(a, core::f32::consts::PI);

// f64 over the whole range: 1e22 needs the Payne–Hanek reduction
let (s, c) = sin_cos64(1.0e22);
assert_eq!(s.to_bits(), 0xbfeb_453a_b76b_f397); // -0.8522008497671888
assert_eq!(c.to_bits(), 0x3fe0_be2c_ef01_c8f4); // 0.523214785395139
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

## Features

- **Scalar** `f32` kernels (Cephes `sinf` / `cosf` / `expf`, musl `logf`) and
  `f64` kernels (fdlibm `atan` / `atan2` / `asin` / `acos` / `exp` / `log`),
  ≤ 1–2 ulp of correctly rounded over the documented domains
- **`sin64` / `cos64` / `sin_cos64`**: `f64` sine and cosine over every finite
  argument — musl `sin.c` / `cos.c` with the Payne–Hanek reduction for
  `|x| ≥ 2^20·π/2`, ≤ 1 ulp of correctly rounded (measured against 6234
  values computed with `mpmath` at 2400 bits, not against a libm);
  `sin_cos64` reduces once and is bit-identical to the two separate calls
- **`tan64`**: `f64` tangent over every finite argument — fdlibm `k_tan.c` on
<!-- claim-test: tan64_within_1_ulp_of_correctly_rounded_reference -->
  the same reduction, ≤ 1 ulp of correctly rounded (measured against 6310
  values from `mpmath` at 2400 bits). It is not compared with a platform libm
  at all: one is ~1.0e5 ulp off in the Payne–Hanek range while another is
  within 2 ulp, so a bound on that difference would depend on the machine
- **`log2` / `log10` / `log2_64` / `log10_64`**: base-2 and base-10
<!-- claim-test: log2_log10_within_1_ulp_of_correctly_rounded -->
  logarithms on the exponent split of fdlibm `e_log10.c`, which keeps the
  mantissa below 1 for a negative exponent so nothing cancels around `x = 1`.
  A power of two returns its exponent exactly, down to the last subnormal;
  `log2_64` is ≤ 2 ulp and `log10_64` ≤ 1 ulp, and the `f32` entry points are
  the `f64` kernels rounded once (measured max 0 ulp from correctly rounded)
- **SIMD** (`simd` feature): `wide::f32x8` versions of `sin` / `cos` /
  `sin_cos` / `exp` / `ln` / `round` / `sqrt`, lane-for-lane bit-identical to
  the scalar functions — same constants, same operation order, no `mul_add`
- **`metric`**: `lerp` / `clamp` / `smoothstep` with one canonical operation
<!-- claim-test: every_nan_these_kernels_create_is_the_canonical_one -->
  order, the three 3D norms, and `MetricWeights` — a metric built as a
  non-negative combination of `‖·‖₁` / `‖·‖₂` / `‖·‖∞`, carrying exact closed
  forms for its Lipschitz constant and for how far a ball of radius `r`
  reaches in Euclidean space
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
- **`SEMANTICS_ID`**: one 32-byte constant that identifies this crate's
<!-- claim-test: semantics_id_matches_the_recorded_constant -->
  numeric behaviour (see below)

## `SEMANTICS_ID`

`SEMANTICS_ID` is a 32-byte constant that identifies *how this crate evaluates
<!-- claim-test: semantics_id_covers_every_public_numeric_function -->
arithmetic*. It changes whenever any function here would return different bits
for the same input, and otherwise only when the set of functions it covers
grows: it is the SHA-256 of the per-function bit pins in `tests/golden.rs`,
folded in ascending order of function name, each as the name's length in four
big-endian bytes, then the name, then its 32-byte pin. No platform `libm`, no
timing and no environment enters it, so it is the same value on every target
the pins reproduce on.

It is for callers that build an identifier out of a formula and its
parameters: mixing this in makes the identifier cover the arithmetic as well,
so two runs that agree on formula, parameters and this value computed the same
bits — and if the value differs, they did not, however well everything else
matches.

Three tests keep it honest. One recomputes it from the pins and fails if the
constant has drifted. The second reads every `pub fn` the crate defines out of
its sources — not only the names re-exported at the crate root — and fails if
any of them has no pin, since a function outside the table could change
behaviour without moving the identifier; it also fails if a module file is
added that the parser does not read. The third covers the SIMD kernels, which
carry no pin of their own because they are feature-gated and an identifier
that changed with the feature set would identify nothing: each one must return
the bits its scalar counterpart returns, lane for lane, so the test fails if
any SIMD function has no parity test.

It moved in 0.4.0, when a corrected `atan64` coefficient changed that
function's output bits — which is the intended behaviour: a consumer that
mixes this value into its own identifiers sees the arithmetic change instead
of silently inheriting it. It moved again in 0.5.0, for both reasons at once:
the output bits of `sin` / `cos` / `sin_cos`, `round` / `round64` / `cbrt`, the
`metric` functions and `powf` changed, and the 11 `metric` entry points that
had been outside the table joined it. It moved once more in 0.5.1 for the
second reason only: `max` / `min` / `max64` / `min64` were added to the table,
and every function 0.5.0 covered still returns the same bits.

### Reading an identifier back

Data written by an earlier release carries that release's identifier.
`check_semantics(&id)` returns `Verified` for `SEMANTICS_ID`,
`VerifiedPrevious(id)` for an entry of `PREVIOUS_SEMANTICS_IDS`, and
`Mismatch` otherwise. An entry is listed only when this release returns that
release's bits on every function the release pinned — 0.5.0 is listed, since
0.5.1 changed no output, while 0.4.0 is not, since 0.5.0 did — and a test
proves it from that release's pin table.
<!-- claim-test: every_previous_identifier_is_backed_by_a_reproduced_pin_table -->
Readers of stored values use `is_reproducible()` (accepts both verified
cases); code that compares identifiers as keys uses `is_current()`. Writers
always stamp `SEMANTICS_ID`.

## Policy for consumers

Add the platform `libm` methods to `clippy.toml` `disallowed-methods` and run
clippy with `-D warnings`, so a stray `x.sin()` on a float is a red gate:

```toml
disallowed-methods = [
    { path = "f32::sin", reason = "platform libm; use alice_det_math::sin" },
    { path = "f32::cos", reason = "platform libm; use alice_det_math::cos" },
    { path = "f64::sin", reason = "platform libm; use alice_det_math::sin64" },
    { path = "f64::cos", reason = "platform libm; use alice_det_math::cos64" },
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

The reference values for every `f64` function whose bound is asserted are
committed under `tests/data/`, so no `f64` bound depends on the platform
`libm` (`atan64` is the one exception). They are regenerated with

```sh
uv run --with mpmath python3 scripts/gen_sin_cos64_reference.py
uv run --with mpmath python3 scripts/gen_tan64_reference.py
uv run --with mpmath python3 scripts/gen_exp64_reference.py
uv run --with mpmath python3 scripts/gen_log64_reference.py
uv run --with mpmath python3 scripts/gen_powf64_reference.py
uv run --with mpmath python3 scripts/gen_atan64_reference.py
```

## no_std

The crate is `#![no_std]` without the default `std` feature and needs no
`alloc`. `sqrt` is then a correctly rounded software square root that returns
the same bits as the hardware instruction; CI builds the library for
`thumbv7em-none-eabihf` (with and without `simd`) and runs the golden hashes
with the software path.

## Minimum supported Rust version

Rust 1.85 (`rust-version` in `Cargo.toml`), checked in CI for the library with
default features, with `std,simd`, and as a `no_std` rlib for
`thumbv7em-none-eabihf`. Development and CI use the toolchain pinned in
`rust-toolchain.toml`.

## Building and testing

```sh
cargo test                                          # unit, accuracy, golden, doc tests
cargo test --features std,simd                      # + SIMD parity
cargo test --no-default-features --features simd    # software sqrt path
cargo clippy --all-targets --features std,simd -- -D warnings
python3 scripts/docs_lint.py --check                # public documents and CHANGELOG
scripts/preflight.sh                                # every CI step with the same arguments
scripts/preflight.sh --quick                        # without the test suites
```

## Related Projects

Crates that evaluate their laws through this one, so two machines agree bit for
bit on the result. This crate is the joint of the ALICE core: a signed-distance
field ([ALICE-SDF](https://github.com/ext-sakamoro/ALICE-SDF)) and the bodies
moving through it ([ALICE-Physics](https://github.com/ext-sakamoro/ALICE-Physics))
only agree on a surface if they compute `sin` the same way, and a law verifier
([ALICE-LOL](https://github.com/ext-sakamoro/ALICE-LOL)) can only prove something
about a field that is reproducible in the first place.

> **Keep the version unified across the resolved graph.** Two versions of this
> crate in one dependency tree means two implementations of the same function,
> and the guarantee is gone. Check with `cargo tree -i alice-det-math`.

| Project | Why it needs bit-exact transcendentals | Links |
|---------|----------------------------------------|-------|
| **ALICE-SDF** | A signed-distance field evaluated on two machines must agree which side of a surface a point is on | [crates.io](https://crates.io/crates/alice-sdf) · [docs.rs](https://docs.rs/alice-sdf) · [GitHub](https://github.com/ext-sakamoro/ALICE-SDF) |
| **ALICE-Physics** | Lockstep / rollback simulation diverges across peers the moment one `sin` differs by an ulp | [crates.io](https://crates.io/crates/alice-physics) · [docs.rs](https://docs.rs/alice-physics) · [GitHub](https://github.com/ext-sakamoro/ALICE-Physics) |
| **ALICE-LOL** | Its SDF law checks run through ALICE-SDF, which evaluates through this crate; from its next release, the `research_law` module also evaluates multi-variable research laws (`exp`, `ln`, `sqrt`, powers, `sin`, `cos`) through the `f64` functions here, so a law re-computed on another machine gives the same bits | [crates.io](https://crates.io/crates/alice-lol) · [docs.rs](https://docs.rs/alice-lol) · [GitHub](https://github.com/ext-sakamoro/ALICE-LOL) |

[ALICE-Zip](https://github.com/ext-sakamoro/ALICE-Zip) does not depend on this
crate: its generators currently use `libm`.

<!-- law-integration:begin law=986f6fb1915b11777677b56397fb49e215f94518 -->
## ALICE-Law integration

Generated by ALICE-Law `scripts/gen-law-readme-block.py` from ALICE-Law `986f6fb1915b` and this repository's `alice-law-conformance.toml`; do not edit it by hand.

| | |
|---|---|
| Release train | alice-det-math 0.5.0 (published), law dependency: yes |
| Arithmetic | alice-det-math 0.5.0 in the train, arithmetic identifier `135c407d4249c1c200e4d1be79a12e848ce5c355fc3a918da965849e697ef000`; this crate is alice-det-math |
| End to end check | covered (`alice-det-math = "0.5"`), digest `966a78784bb4e3341ac6627d519db896f398176b71c2402b188294db6c8e3728`, the same on aarch64 and x86_64 |

### Conformance (G1-G14)

| gate | status | note |
|---|---|---|
| G1 no-libm-direct | pass | G1 bans the platform transcendentals outside alice-det-math; this is that crate, so the gate here is the inverse claim it names it to carry: its own … |
| G2 ieee-max-min | known unmet | the published crate's one Law-bearing f32/f64 max/min is src/simd.rs:270, kf.max(C_NEG126P0).min(C_127P0) -- a wide::f32x8 exponent clamp in the SIMD exp … |
| G3 no-outer-fneg-of-fma | pass | no fused multiply-add call in the library sources: the syntax-tree check reads 22 files and counts 0 fma-family calls against the declared 0, so the first new … |
| G4 debug-release-arch-goldens | known unmet | the bit-pattern goldens exist and are strong -- tests/golden.rs pins every function over large integer-built grids (with an emptiness guard: the fold rejects … |
| G5 release-profile-test-leg | known unmet | no job anywhere runs the Law-bearing test suite under --release, on aarch64 or any arch: the test matrix runs cargo test in debug on all five native runners … |
| G6 semantics-id | pass | alice-det-math defines its own identifier: SEMANTICS_ID (src/lib.rs:148) is the SHA-256 of the per-function bit pins in tests/golden.rs, folded in ascending … |
| G7 handshake-carries-id | n/a | G7 governs a Law-bearing value crossing a crate boundary carrying its producer's G6 identifier. alice-det-math is the root compute library: its public API is … |
| G8 persisted-semantics-id | n/a | G8 governs Law-bearing values written to a durable form carrying their own persisted identifier. alice-det-math persists nothing: it is a no_std, no_alloc … |
| G9 persisted-meaning-change-is-major | pass | definition: SEMANTICS_ID moving counts as a persisted-meaning change, recorded as the semver-major equivalent (a 0.x minor), unless both hold -- the old … |
| G10 no-run-varying-artifacts | pass | the published crate's library src/ reads no wall clock, no unseeded RNG and no UUID, and the absence is ratcheted by two layers with the clock-via-elapsed hole … |
| G11 gate-self-tests | known unmet | unmet: clause (c). scripts/clippy_canary.py, scripts/test_clippy_canary.py, scripts/gen_clippy_canary_calls.py and scripts/test_gen_clippy_canary_calls.py are … |
| G12 public-repo-text-rules | pass | the public-text.yml job runs scripts/public_text.sh, which fetches ALICE-Law's own scripts/public_text_check.py at the revision .github/public-text-pin names … |
| G13 no-unspecified-order | known unmet | the published crate's library src/ names HashMap / HashSet / BTreeMap nowhere (a full scan returned zero), and it is no_std without alloc, so the standard … |
| G14 float-environment-invariants | pass | the simd-paths job runs the tests under three codegen settings, one of them target-cpu=native, to show the results do not depend on the CPU; those are test … |

### Features and the guarantee

| feature | guarantee | note |
|---|---|---|
| `simd` | inside | simd enables the wide::f32x8 kernels. They are lane-for-lane bit-identical to the scalar f32 functions, proven by tests/simd_parity.rs over dense sweeps and … |
| `std` | inside | std enables the hardware sqrt instruction for ops::sqrt / sqrt64 (IEEE 754 requires sqrt correctly rounded, so the hardware result is bit-identical to the … |
<!-- law-integration:end -->

## License

Apache-2.0 (`LICENSE-APACHE`, with `NOTICE`). Releases up to and including
0.4.0 were published as `MIT OR Apache-2.0`. Coefficients and algorithms are
from Cephes (Stephen L. Moshier), fdlibm (Sun Microsystems) and musl; see the
source for attribution. A redistribution, in source or binary form, must carry
`LICENSE-APACHE` and `NOTICE`; they need not appear in a user interface. The
ALICE name is covered by `TRADEMARK_NOTICE`, not by the licence.
