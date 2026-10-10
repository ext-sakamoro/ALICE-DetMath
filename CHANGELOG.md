# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.0] - 2026-10-08

**Breaking:** `SEMANTICS_ID` changes, from
`d2209b30f6f1f45baa1b638bcdfee34ac64773b2e63b9c083b2e77afc691398e` to
`a6fe7dc833d2a35012fc11eaf8e253eca6bfdb76db2931f056132f44bf79d6af`. No
function returns different bits than in 0.4.0: the identifier moves because
the set of functions it covers grew by 11. A consumer that mixes it into its
own identifiers has to re-record them, and gains coverage of the `metric`
module in exchange.

### ライセンス

- `MIT OR Apache-2.0` から **`Apache-2.0` のみ**に変えた (0.5.0 から) `LICENSE-MIT` を削除し、`LICENSE-APACHE` に加えて `NOTICE` (著作権表示と Cephes / fdlibm / musl の出典) と `TRADEMARK_NOTICE` (ALICE の名称の扱い、ライセンスとは独立) を置いた 0.4.0 までの公開版は `MIT OR Apache-2.0` のまま 依存する側が `alice-det-math` を permissive として扱う点は変わらない (Apache-2.0 は MIT / Apache / AGPL の crate から引ける) `scripts/license_check.py` が `Cargo.toml` の宣言・同梱 file・`cargo package --list`・README の記載の一致を検査する (CI と preflight、検査 0 件で失敗)

### Added

- `PREVIOUS_SEMANTICS_IDS`, `SemanticsCheck` and `check_semantics`, so data
  stamped with 0.4.0's identifier (`d2209b30…`) stays readable under 0.5.0.
  The 0.4.0 identifier is accepted because its functions return the same bits
  here, which a test proves: it carries 0.4.0's pin table verbatim, checks
  that it folds to `d2209b30…`, and checks that all 40 of its pins equal the
  pins this release recomputes (one renamed: `metric_norm_mix` → `metric_norm`).
  An identifier is added only together with such a table, and the count of
  identifiers must equal the count of tables. The result is three-way —
  `Verified`, `VerifiedPrevious(id)`, `Mismatch` — so a reader never treats an
  earlier identifier as silently equal. Two predicates name the two uses:
  `is_reproducible()` (readers of stored residuals, law identifiers,
  snapshots) and `is_current()` (identifiers compared as keys, and writers,
  which always stamp `SEMANTICS_ID`). The module computes no floating-point
  value and is outside the pinned numeric surface; a test fails if a float
  appears in it. Five mutations are red: a changed hash in either table, an
  extra identifier without a table, a float in the module, and a dropped
  rename.

### Fixed

- `SEMANTICS_ID` now covers the whole public numeric surface. It covered 37
  functions out of 48: the check behind it read only the
  `pub use <module>::{ … }` lists in `src/lib.rs`, so it saw the 34 names
  re-exported at the crate root, while the `metric` module reaches callers as
  `pub mod metric` with no re-export. Eleven entry points —
  `metric::{norm_l1, norm_l2, norm_linf, lerp, clamp}` and
  `MetricWeights::{new, weights, minimum, axis_extent, euclidean_radius,
  normalised}` — therefore had no pin, and their output bits could change
  without the identifier moving. Each one now has a pin in `tests/golden.rs`,
  and the `metric_weights` pin folds in `MetricWeights::{L1, L2, LINF}` and
  `Default` as well, so the three named metrics are covered too.

### Changed

- The coverage check reads the `pub fn` / `pub const fn` definitions out of
  every source file the crate compiles, rather than the re-export lists, and
  asserts the surface size (48) and the pin count (51) exactly, so a parser
  that stopped seeing the surface fails instead of comparing nothing. A new
  test asserts that every module the crate declares is one the parser reads,
  which is what let the `metric` module stay outside until now. Measured
  against six mutations, all red: a reordered sum in `norm_l1`, a reciprocal
  multiply in `axis_extent`, the other spelling of `lerp`, an unpinned new
  `pub fn`, an unparsed new module, and a deleted SIMD parity test.
- The SIMD kernels are stated as covered by parity rather than by pins, and a
  test enforces it: all 21 `pub fn` in `src/simd.rs` are exercised by
  `tests/simd_parity.rs`. Folding them into `SEMANTICS_ID` would make the
  identifier depend on the feature set, which would defeat its purpose.
- The golden pin `metric_norm_mix` is renamed `metric_norm`, so that the pin
  for a method is `<module>_<method>` throughout. Its hash is unchanged.
- The documented contract is corrected: the identifier changes when any
  function's bits change *or* when the set of functions it covers grows. The
  old wording ("and only then") did not admit the second case, which this
  release is.

## [0.4.0] - 2026-10-07

**Breaking:** `atan64` and `atan2_64` return different bits than 0.3.x for
some inputs, because a mistranscribed coefficient was corrected (below).
Anything that pins their output — a golden hash, a recorded simulation, a
stored signed-distance field — has to be re-recorded against this version.
`SEMANTICS_ID` changes with them, which is what it is for: a consumer mixing
it into an identifier sees the arithmetic change rather than silently
inheriting it. No API changed, and no other function's bits changed (the
`f32` `atan` has its own coefficients and is unaffected).

### Changed
- `cargo semver-checks` runs twice in CI and in `scripts/preflight.sh`, and
  the gate is the number of comparisons rather than the exit code. The first
  run selects its lints by the bump already declared, so with Cargo.toml a
  major (or 0.x minor) ahead of crates.io it decides every lint is unnecessary
  and compares nothing — measured on this release: `0 checks: 0 pass,
  254 skip`, printed as `no semver update required` with exit 0. The second
  run asks for a patch release, which exercises the major and minor lints
  (223 of them here), and both files fail if the count is absent or zero.
  Exit 100 stays an acceptable outcome: it is how a genuinely breaking release
  reports its list.
- `SEMANTICS_ID` and `tests/golden.rs` document the order a re-pin has to
  follow: the per-function hashes first, the identifier second. The fold reads
  the table as written in the source, so recomputing the identifier before the
  table is updated reproduces the old value and reads as though it had failed
  to notice the change.

### Fixed
- `atan64`: `AT[2]`, the third coefficient of the fdlibm `s_atan.c`
  polynomial, was mistranscribed as `1.42857142759371231480e-01` where the
  source has `1.42857142725034663711e-01` — a relative error of 2.4e-10 in a
  term of the polynomial. Just below the `0.4375` branch threshold this left
  the result **1898 ulp** from correctly rounded, against a documented bound
  of 1 ulp: at `x = 0.4374999999999999` it returned `4.12410441597281852e-01`
  where the correctly rounded value is `4.12410441597387212e-01`. The error
  grows as the fourth power of the argument inside that branch, so it was
  invisible at small `|x|` and absent from the other four branches, where the
  coefficient's contribution is negligible. The other ten `AT` coefficients
  and all eight `ATANHI` / `ATANLO` entries match the source exactly, checked
  value by value. With the correction every branch is within 1 ulp.
- The `atan64` accuracy test could not have caught it. It swept
  `-1e6 + 2e6·(i/200000)`, a step of 10, so all 200001 points had
  `|x| >= 2.4375`: one of the five branches of the reduction was measured and
  four were not, while the assertion read `<= 1 ulp` and passed. It now runs
  over the committed reference and **counts how many inputs reach each
  branch, failing if any branch is unmeasured**, so narrowing the inputs
  cannot quietly recreate the blind spot.

### Added
- `log2` / `log10` (`f32`) and `log2_64` / `log10_64` (`f64`): deterministic
  base-2 and base-10 logarithms. The exponent split of fdlibm `e_log10.c`
  gives `x = m·2^y` with `m` kept below 1 when `y` is negative, so there is no
  cancellation around `x = 1`; the fraction comes from `ln64` and is converted
  with `log10(2)` split high / low (base 10) or `1/ln 2` split into a 24-bit
  head and a tail (base 2). A power of two returns its exponent exactly, down
  to the last subnormal. The `f32` entry points are the `f64` kernels rounded
  once, as `asin` / `acos` / `powf` already are. Measured against the platform
  libm: `log2_64` ≤ 2 ulp, `log10_64` ≤ 1 ulp; `log2` / `log10` are within
  0 ulp of the correctly rounded `f32` over a 200 000-point log-spaced sweep.
  Domain edges follow `ln`: `-inf` at either zero, the canonical NaN below it
  and for a NaN input, `inf` at `inf`.
- `tan64`: `f64` tangent over every finite argument, a port of musl `tan.c`
  with the fdlibm `k_tan.c` kernel, reusing the Cody–Waite and Payne–Hanek
  reduction already used by `sin64` / `cos64`. Only IEEE 754 basic operations
  in the source's order, no `mul_add`. `tan(±0) = ±0`, subnormals return `x`,
  `±inf` and NaN return the canonical NaN. Within 1 ulp of correctly rounded
  at 76 reference points (independent 2400-bit evaluation, 21 of them above
  `1e9` so the Payne–Hanek path is covered); the platform libm is itself up to
  3 ulp off at several of those points.
- `tests/accuracy.rs`: the logarithms against the correctly rounded reference
  and against every power of two, both logarithms' domain edges and the
  tangent's behaviour at the nine `f64` nearest to the first poles (large and
  finite, sign flipping across the pole), `tan64` against the 76-point
  reference table and against `sin64 / cos64`, and the new functions in the
  canonical-NaN check.
- `tests/golden.rs`: `log2`, `log10`, `log2_64`, `log10_64`, `tan64`, and a
  dense grid over `[−10, 10]` for the tangent, which covers both sides of the
  kernel's `0.6744` branch in both quadrant parities.
- `tests/data/tan64_reference.txt`: correctly rounded `tan` at the 6234 inputs
  of the sine / cosine reference (1964 of them in the Payne–Hanek range),
  from the same independent 2400-bit evaluation, with
  `scripts/gen_tan64_reference.py` to regenerate it. `tan64` is within 1 ulp
  of it, measured max 1.
- `SEMANTICS_ID`: a `[u8; 32]` that identifies this crate's numeric behaviour.
  It is the SHA-256 of the per-function bit pins in `tests/golden.rs`, folded
  in ascending order of function name, each as the name's length in four
  big-endian bytes, then the name, then its 32-byte pin — so it changes
  exactly when some function would return different bits for the same input,
  and nothing outside this crate (no platform `libm`, no environment) enters
  it. A caller deriving an identifier from a formula and its parameters can
  mix it in so the identifier also covers how the arithmetic was evaluated.
  Available in `no_std`. `tests/golden.rs` recomputes it from the pins, reads
  the public numeric functions out of `src/lib.rs` and fails if any of them
  has no pin, checks that perturbing any single pin moves the value, that the
  fold does not depend on the written order, that the name length is part of
  the input (with the pair of tables that would otherwise collide), and that
  an empty, duplicated, non-ASCII or malformed table is rejected rather than
  folded.

### Changed
- `powf64` documents that outside its domain (`x ∈ [1e-3, 1e3]`, `|y| ≤ 8`)
  the result can be wrong rather than merely less accurate, with the mechanism
  and a measured case: the residual step `l_lo = x·exp64(−l_hi) − 1` collapses
  to `x − 1` when `x` is within one ulp of 1, because `exp64(−l_hi)` rounds to
  exactly 1, so `y·l_lo` adds a second copy of `y·ln x` and the exponent is
  doubled. At `x = 1 − 1 ulp`, `y = 1e15` it returns `7.9556e-1` against a
  correctly rounded `8.9492e-1`. Inside the domain the duplicated term is
  below half an ulp and the bound holds, so this is a documentation change:
  no behaviour changed.
- No `f64` accuracy bound is asserted against a platform libm any more.
  `exp64`, `ln64`, `log2_64`, `log10_64` and `powf64` join `tan64` in being
  measured against committed tables of correctly rounded values
  (`tests/data/exp64_reference.txt`, `log64_reference.txt`,
  `powf64_reference.txt`, from the new `scripts/gen_exp64_reference.py`,
  `gen_log64_reference.py` and `gen_powf64_reference.py`). Measured against
  the true value the errors are smaller than the libm comparison suggested:
  `exp64` 1 ulp, `ln64` 1, `log2_64` 1 (the libm comparison said 2, one of
  which was the libm's own), `log10_64` 1, `powf64` 13. The bounds are
  unchanged; the distance from the platform libm is printed, not asserted.
  `atan64` is the one `f64` function still measured against the libm.
- The `log2`/`log10` identity check takes its inputs from the committed table
  instead of a `powf`-generated sweep, so the points it measures at no longer
  depend on the platform either.
- `tan64`'s accuracy test no longer asserts against the platform libm. The
  libm disagrees with the correctly rounded value by ~1.0e5 ulp at
  `x = 0x6404c96c11134d36` on one platform and by 2 ulp on another, so a
  bound on that difference passed or failed according to which machine ran
  it. The assertions are now against the committed reference above; the
  difference from the platform libm is printed, not asserted.

## [0.3.2] - 2026-10-07

Additive: no existing function changes its bits (every previously recorded
golden hash is unchanged).

### Added
- `sin64` / `cos64` / `sin_cos64`: `f64` sine and cosine over every finite
  argument, a line-by-line port of musl `sin.c` / `cos.c` with the fdlibm
  kernels `__sin` / `__cos`, the Cody–Waite reduction of `__rem_pio2` up to
  `2^20·π/2` and the Payne–Hanek reduction of `__rem_pio2_large` (2/π to
  1584 bits) above it. Only IEEE 754 basic operations in the source's order,
  no `mul_add`; `±inf` and NaN return the canonical NaN, `±0` and subnormals
  return `x` (sine) and `1` (cosine). `sin_cos64` reduces once and is
  bit-identical to the two separate calls.
- `tests/data/sin_cos64_reference.txt`: correctly rounded `sin` / `cos` at
  6234 inputs (small, medium, every reduction-branch threshold, near multiples
  of π/2, the input closest to a multiple of π/2 in every third binade, and
  log-uniform values up to `f64::MAX`), generated by
  `scripts/gen_sin_cos64_reference.py` with `mpmath` at 2400 bits — not by a
  libm. `sin64` / `cos64` are within 1 ulp of it (measured max 1 ulp).
  `sin64(−x) = −sin64(x)` and `cos64(−x) = cos64(x)` are checked bitwise.
- `tests/golden.rs`: `sin64`, `cos64`, `sin_cos64`, and a dense grid over
  `[−10, 10]` for the first reduction cases.
- `README_JP.md`: the Japanese README, with the same sections and identical
  code blocks.

### Changed
- README: installation with `cargo add` (no version pinned in the text), the
  first example is the crate doctest, `sin64` / `cos64` / `sin_cos64` in the
  feature list, a note on what the crate is not for, and sections for
  `no_std`, MSRV and building. Related projects: ALICE-LOL added; ALICE-Zip
  removed from the list of users, since it does not depend on this crate.
- CI: `scripts/docs_lint.py` (with its tests in `scripts/test_docs_lint.py`)
  runs on Linux, macOS and Windows and in `scripts/preflight.sh`. It checks
  the public documents and every tracked file for development-process
  vocabulary and private names (hashed, not listed), the CHANGELOG heading
  structure, and that the README's first example equals the crate doctest;
  a check that compares nothing fails. Comments that it flagged were reworded.

## [0.3.1] - 2026-09-27

### Added
- `MetricWeights::axis_extent` — the half-width along each axis of the ball
  `{x : g(x) ≤ r}`, exactly `r / (w₁ + w₂ + w∞)`. It is deliberately *not*
  `euclidean_radius`: a cube-metric ball of radius `r` is the cube
  `[−r, r]³` (half-width `r`) even though it reaches `√3·r` along its
  diagonal. The first is the tight box of a ball, the second is what a
  *clearance* measured in the metric guarantees in Euclidean space, and
  using one where the other belongs is either a missed pair (too small) or a
  box three times too big. Checked against a brute-force sweep.

## [0.3.0] - 2026-09-27

Additive: no existing function changes its bits (the goldens for every
previously recorded function are unchanged).

### Added
- `metric` module: the kernels a distance field needs when the metric itself
  becomes a value — `lerp` / `clamp` / `smoothstep` (one canonical operation
  order for every port to copy), `norm_l1` / `norm_l2` / `norm_linf`, and
  `MetricWeights`, a norm built as a non-negative combination of the three
  bases. `MetricWeights::new` rejects a negative weight (the unit ball stops
  being convex, so it is not a metric) and the two extremes over the
  Euclidean unit sphere are exact closed forms, not estimates:
  `lipschitz() = √((w₁+w∞)² + 2w₁²) + w₂` and
  `minimum() = min_k (k·w₁ + w∞)/√k + w₂` — the second gives
  `euclidean_radius()`, the factor an axis-aligned bound must grow by
  (`√3` for the cube metric). Checked against a brute-force sweep in
  `tests/analytic_metric.rs` (17 tests).
- `simd`: `lerp`, `clamp`, `smoothstep`, `metric_norm` and `MetricWeightsX8`,
  lane-for-lane bit-identical to the scalar versions over the special values
  and sprays (`tests/simd_parity.rs`). `‖·‖∞` is a mask + blend rather than
  `f32x8::max`, whose NaN behaviour differs between the SSE and NEON backends.
- `tests/golden.rs`: `smoothstep`, `metric_norm_mix` and `metric_lipschitz`
  hashes, so the new kernels are pinned across every CI target too.

### Fixed
- `metric`: a NaN raised by arithmetic (`inf/inf` in `smoothstep`, `0·inf` in
  `lerp` and `MetricWeights::norm`) kept the hardware's default sign — `+NaN`
  on aarch64 / wasm32, `−NaN` on x86_64 — which splits the golden hash
  between targets. Caught by the x86_64 leg of `scripts/preflight.sh` before
  the first commit: 293 of 8000 special-value triples differed. Generated
  NaNs are now canonicalised, as `sqrt` already did; an *input* NaN still
  passes its payload through, which every target does identically.

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

[Unreleased]: https://github.com/ext-sakamoro/ALICE-DetMath/compare/v0.5.0...HEAD
[0.5.0]: https://github.com/ext-sakamoro/ALICE-DetMath/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/ext-sakamoro/ALICE-DetMath/compare/v0.3.2...v0.4.0
[0.3.2]: https://github.com/ext-sakamoro/ALICE-DetMath/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/ext-sakamoro/ALICE-DetMath/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/ext-sakamoro/ALICE-DetMath/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/ext-sakamoro/ALICE-DetMath/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/ext-sakamoro/ALICE-DetMath/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/ext-sakamoro/ALICE-DetMath/releases/tag/v0.1.0
