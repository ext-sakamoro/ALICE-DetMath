# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- README / lib.rs の全称 claim を実態に限定し、各 claim 行に `<!-- claim-test: fn -->` で検証 test を紐付け (strict-eval 検査 1、2026-09-17)

## [0.2.0] - 2026-09-17

Results change in the last ulp for some inputs (the law changed, see below);
every consumer re-pins its goldens: alice-physics 1.4.0, alice-sdf 3.1.0.

### Changed

- Range reduction (`sin`, `cos`, `sin_cos`, `exp`): the nearest integer is
  `trunc(t + (t < 0 ? -0.5 : 0.5))` (three basic operations) instead of the
  exact `round` (musl `roundf`, ~20 operations). It differs from `round` only
  within an ulp of an exact tie, where either neighbour is a valid reduction;
  accuracy bounds are unchanged (`tests/accuracy.rs`).
- `atan` / `atan2`: fdlibm's single-precision `s_atanf.c` / `e_atan2f.c`
  (≤ 1 ulp) instead of the double-precision kernel rounded once — ~3× faster
  per lane; the `f64` kernels `atan64` / `atan2_64` are unchanged.
- `simd`: the kernels are written with float arithmetic, compares + blends
  and lane shifts only, and every constant is a `const` vector. With wide 0.7
  on aarch64, `f32x8::splat` compiles to a `memset_pattern16` *call* per
  constant and the integer / bitwise lane ops have no NEON path; the previous
  version paid 27 calls per `sin_cos` (~90 ns / `f32x8`, now ~25 ns — on par
  with `wide`'s own `sin_cos`). Bit-identical to the scalar functions as
  before (`tests/simd_parity.rs`).
- `#[inline]` on every public function (cross-crate inlining without LTO).

## [0.1.1] - 2026-09-16

### Added

- `consts` module: the `f32` kernel constants (`SIN_P`, `COS_P`, `PIO2_*`,
  `EXP_P`, `LG*`, …) for code generators that emit the same law elsewhere
  (alice-sdf's Cranelift JIT) and are checked against these functions bit
  for bit.

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

[Unreleased]: https://github.com/ext-sakamoro/ALICE-DetMath/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/ext-sakamoro/ALICE-DetMath/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/ext-sakamoro/ALICE-DetMath/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/ext-sakamoro/ALICE-DetMath/releases/tag/v0.1.0
