//! Deterministic `f32` / `f64` transcendentals (cross-platform bit-exact).
//!
//! IEEE 754 guarantees that `+`, `-`, `*`, `/` and `sqrt` produce the same
//! bits on every platform Rust targets (SSE2+, aarch64, wasm32). It does
//! **not** cover `sin`, `exp`, `ln`, `powf`, `cbrt`, `hypot`, … — those come
//! from the platform `libm` and differ in the last ulp between macOS, glibc,
//! MSVC and wasm. A lockstep simulation that calls them diverges across
//! peers; a signed-distance field evaluated on two machines disagrees about
//! which side of a surface a point is on.
//!
//! Every function here is built from the guaranteed operations only —
//! integer range reduction (`trunc(t + (t < 0 ? -0.5 : 0.5))`, three
//! operations), fixed-degree polynomials evaluated in a fixed order, and
//! bit-level exponent construction — so the result is a pure function of
//! the input bits on every target. `mul_add` is not used
//! anywhere: it fuses into one rounding on FMA hardware and stays two
//! roundings elsewhere, which would break the guarantee.
//!
//! The crate is `no_std` (no `alloc` either). `round` and `sqrt`, which
//! `core` does not provide, are in [`ops`]; with the `std` feature `sqrt` is
//! the hardware instruction, without it a correctly rounded software one —
//! both return the same bits.
//!
//! With the `simd` feature, the `simd` module provides `wide::f32x8` versions
//! that are lane-for-lane bit-identical to the scalar functions.
//!
//! # Accuracy
//!
//! Measured over dense sweeps (`tests/accuracy.rs`). `f32` functions are
//! compared with a correctly rounded reference (`f64` libm rounded once to
//! `f32`), the `f64` functions with the platform `f64` libm — which itself
//! varies by ≤ 1 ulp between macOS / glibc / MSVC, so those bounds carry that
//! slack:
//!
//! | function | domain measured | max error |
//! |----------|-----------------|-----------|
//! | [`sin`] / [`cos`] / [`sin_cos`] | `|x| ≤ 100` | ≤ 2 ulp (Cephes single-precision coefficients) |
//! | [`exp`] | `[-87, 88]` | ≤ 2 ulp |
//! | [`ln`] | `[1e-30, 1e30]` | ≤ 1 ulp (musl `logf` algorithm) |
//! | [`cbrt`] | `[1e-30, 1e30]` | ≤ 1 ulp (bit-hack seed + 3 Newton steps) |
//! | [`hypot`] | finite | ≤ 1 ulp of `sqrt(x² + y²)` |
//! | [`powf`] | `x ∈ [1e-3, 1e3]`, `|y| ≤ 8` | ≤ 1 ulp (evaluated in `f64`, rounded once) |
//! | [`atan`] / [`atan2`] | `|x| ≤ 1e4`, all quadrants | ≤ 1 ulp (fdlibm `s_atanf` / `e_atan2f`, single precision) |
//! | [`asin`] / [`acos`] | `[-1, 1]` | ≤ 1 ulp (fdlibm in `f64`, rounded once) |
//! | [`tan`] | `|x| ≤ 100` | ≤ 1 ulp (fdlibm `k_sin`/`k_cos` in `f64`) |
//! | [`tanh`] | finite | ≤ 1 ulp (`exp64` based, `x` below 2^-14) |
//! | [`exp64`] / [`ln64`] | as above | ≤ 1 ulp on macOS libm, bound 2 across platform libms (fdlibm algorithms) |
//! | [`powf64`] | as above | ≤ 16 ulp measured 13 (`exp64(y·ln64 x)` with double-double argument) |
//! | [`round`] / [`round64`] / [`sqrt`] / [`sqrt64`] | all | exact (bit-identical to `f32::round` / `f32::sqrt`) |
//!
//! Large arguments to [`sin`] / [`cos`] (`|x| > 2¹³`) are still deterministic
//! but the single-precision Cody–Waite reduction loses accuracy, exactly as
//! `libm`'s own `sinf` does without Payne–Hanek.
//!
//! # Guarantee scope
//!
//! Bit-exactness relies on the target honouring IEEE 754 for the basic
//! operations: `x86_64` (SSE2), `aarch64`, `wasm32` and every other Rust tier-1/2
//! target qualify. 32-bit x86 built for x87 (`i586`) without SSE2, or any
//! build with fast-math style flags, is outside the guarantee.
//! `tests/golden.rs` pins a SHA-256 of every function's output over a fixed
//! input grid; CI runs it on macOS (ARM / Intel), Linux (x86 / ARM), Windows
//! and wasm32, and with the SIMD fallback paths forced.
//!
//! # Policy for consumers
//!
//! List the `f32` / `f64` `libm` methods under `clippy.toml`
//! `disallowed-methods` and run clippy with `-D warnings`, so a stray
//! `x.sin()` on a float is a red gate; call these functions instead.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod double;
pub mod ops;
#[cfg(feature = "simd")]
#[cfg_attr(docsrs, doc(cfg(feature = "simd")))]
pub mod simd;
pub mod single;

/// The `f32` kernel constants (Cephes / musl), for code generators that must
/// emit the *same* law elsewhere (a JIT, a shader) and be checked against
/// these functions bit for bit.
pub mod consts {
    pub use crate::single::{
        COS_P, EXP_HI, EXP_LO, EXP_P, FRAC_2_PI, LG1, LG2, LG3, LG4, LN2_HI, LN2_LO, LOG2E, PIO2_1,
        PIO2_2, PIO2_3, SIN_P, SQRT2_BITS,
    };
}

pub use double::{acos64, asin64, atan2_64, atan64, exp64, ln64, powf64};
pub use ops::{round, round64, sqrt, sqrt64};
pub use single::{
    acos, asin, atan, atan2, cbrt, cos, exp, hypot, ln, powf, powi, sin, sin_cos, tan, tanh,
};
