# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-09-16

Extracted from `alice-physics` 1.2.0 `det_math` (bit-for-bit: the pins in
`tests/accuracy.rs::bit_exact_pins` are the 1.2.0 values).

### Added

- `f32`: `sin`, `cos`, `sin_cos`, `exp`, `ln`, `powf`, `powi`, `cbrt`, `hypot`,
  `atan`, `atan2`, `asin`, `acos`, `tan`, `tanh`
- `f64`: `atan64`, `atan2_64`, `asin64`, `acos64`, `exp64`, `ln64`, `powf64`
- `ops`: `round`, `round64` (musl formula, bit-identical to `f32::round`) and
  `sqrt`, `sqrt64` (hardware with `std`, correctly rounded software root
  without — same bits), so the crate is `no_std` on bare-metal targets
- `simd` feature: `wide::f32x8` `sin`, `cos`, `sin_cos`, `exp`, `ln`, `round`,
  `sqrt`, lane-for-lane bit-identical to the scalar functions, plus per-lane
  wrappers for the `f64`-kernel functions
- `tests/golden.rs`: SHA-256 pins of every function over a libm-free input
  grid, reproduced by CI on 5 native platforms, wasm32, the SSE2-only SIMD
  fallback, AVX2 + FMA, and `no_std`
- `tests/simd_parity.rs`, `tests/accuracy.rs` (≤ 1–2 ulp oracle against the
  correctly rounded reference, NaN canonicalisation), 3 fuzz targets

### Changed (vs `alice-physics` 1.2.0 `det_math`)

- `asin64` / `acos64` / `powi`: a NaN input returns the canonical NaN instead of
  propagating through arithmetic (whose payload / sign rules differ between
  aarch64, x86 and wasm — found by the wasm32 golden lane). Non-NaN inputs are
  unchanged.
- `sin` / `cos` / `sin_cos` / `tan`: for `|x| > 2^24` (beyond the reduction)
  a NaN produced by the arithmetic is replaced by the canonical NaN — the
  platform default NaN's sign differs between x86 (negative) and AArch64
  (positive). Finite results are unchanged.
- `cbrt`: inputs above 2^96 are scaled down by 2^24 before the Newton steps
  (`y³` overflowed near `f32::MAX` and produced `inf / inf`); results that
  were finite before are bit-identical, `cbrt(f32::MAX)` is now correct.
- `asin64` / `acos64` are public (they were private kernels).

[Unreleased]: https://github.com/ext-sakamoro/ALICE-DetMath/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/ext-sakamoro/ALICE-DetMath/releases/tag/v0.1.0
