//! Deterministic blending and 3D norms — the kernels a distance field needs
//! when the *metric itself* becomes a value.
//!
//! A signed distance field is written against a norm. Change the norm and the
//! same expression describes a different shape: `‖p‖₂ − r` is a sphere,
//! `‖p‖∞ − r` is a cube, `‖p‖₁ − r` an octahedron. Blending between those
//! three, and blending between two *regions* that use different norms, is the
//! whole of that operation — and every port of it (a SIMD kernel, a JIT, a
//! shader) has to agree bit for bit or the field disagrees with itself about
//! which side of a surface a point is on.
//!
//! This module pins that law once:
//!
//! * [`lerp`], [`clamp`], [`smoothstep`] — the blend weights. They use only
//!   IEEE basic operations, so they were already deterministic; what was
//!   missing is a *single canonical operation order* that every port copies.
//!   `smoothstep` in particular is spelled differently by GLSL, HLSL and WGSL
//!   implementations, and a shader compiled with fast-math will not reproduce
//!   any of them.
//! * [`norm_l1`], [`norm_l2`], [`norm_linf`] and [`MetricWeights`] — the norm
//!   and the convex combination of the three bases, with the two closed forms
//!   a caller needs to stay safe: how fast the field can change
//!   ([`MetricWeights::lipschitz`]) and how far a ball of radius `r` actually
//!   reaches ([`MetricWeights::euclidean_radius`]).
//!
//! # Why a convex combination and not a free exponent
//!
//! The obvious parameterisation is the Minkowski `p`-norm
//! `(|x|ᵖ + |y|ᵖ + |z|ᵖ)^(1/p)`, but it calls [`powf`](crate::powf) three
//! times plus a root *per sample*, and `p < 1` silently stops being a norm.
//! A weighted sum `w₁‖·‖₁ + w₂‖·‖₂ + w∞‖·‖∞` covers the same visual range
//! (octahedron ↔ sphere ↔ cube) with three cheap evaluations, is a norm for
//! exactly the non-negative weights — a non-negative combination of norms
//! satisfies the triangle inequality term by term — and the failure case is
//! then a *type* error rather than a field that tears at run time.
//! [`MetricWeights::new`] rejects it.
//!
//! # Closed forms (3D)
//!
//! With `g(h) = w₁‖h‖₁ + w₂‖h‖₂ + w∞‖h‖∞`, positively homogeneous of degree
//! one, the extremes over the Euclidean unit sphere are exact, not estimates.
//! Sign and permutation symmetry let one take `h` descending and
//! non-negative, and then `g − w₂` is the linear functional `⟨u, h⟩` with
//! `u = (w₁+w∞, w₁, w₁)`:
//!
//! ```text
//! max g = ‖u‖₂ + w₂ = √((w₁ + w∞)² + 2w₁²) + w₂        (at u/‖u‖₂)
//! min g = min_{k∈{1,2,3}} (k·w₁ + w∞)/√k + w₂          (on an extreme ray)
//! ```
//!
//! The maximiser `u/‖u‖₂` is itself descending and non-negative, so it lies in
//! the cone and the maximum is attained; the minimiser `−u/‖u‖₂` does not, so
//! the minimum sits on one of the cone's three extreme rays `(1,0,0)`,
//! `(1,1,0)/√2`, `(1,1,1)/√3`. For the pure bases this gives `√3` and `1`
//! for `‖·‖₁`, `1` and `1` for `‖·‖₂`, `1` and `1/√3` for `‖·‖∞` — the last
//! being why a cube-metric ball of radius `r` reaches `√3·r` in Euclidean
//! space, and why an axis-aligned bound computed the Euclidean way misses it.
//!
//! `tests/analytic_metric.rs` checks both closed forms against a brute-force
//! sweep that shares no code with this module.

use crate::ops::sqrt;

/// Replaces a NaN produced by arithmetic with the canonical positive quiet
/// NaN.
///
/// `inf/inf` and `0·inf` raise the *default* NaN, whose sign bit is target
/// dependent: `+NaN` (`0x7fc0_0000`) on `aarch64` and `wasm32`, `−NaN`
/// (`0xffc0_0000`) on `x86_64`. Measured, not assumed — a sweep of the
/// special values over [`smoothstep`] differs in 293 of 8000 triples between
/// the two, all of them generated NaNs. Propagating an *input* NaN keeps its
/// payload identically on every target, so the pass-through arms
/// ([`clamp`], and the NaN arm of [`smoothstep`]) need no canonicalisation;
/// only results that arithmetic could have created do. Same policy as
/// [`sqrt`].
#[inline]
fn canonical(v: f32) -> f32 {
    if v.is_nan() {
        f32::NAN
    } else {
        v
    }
}

/// √2 as `f32`, exact literal (never computed at run time).
const SQRT_2: f32 = 1.414_213_5;
/// √3 as `f32`, exact literal (never computed at run time).
const SQRT_3: f32 = 1.732_050_8;

/// Why a weight triple is not a metric.
///
/// Values are produced by [`MetricWeights::new`] only; downstream code
/// matching on it needs a wildcard arm, as the list can grow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MetricError {
    /// A weight was NaN or infinite.
    NotFinite,
    /// A weight was negative: the unit ball stops being convex, the triangle
    /// inequality fails, and the field is no longer a distance in any metric.
    NotConvex,
    /// Every weight was zero: there is no norm to speak of.
    Degenerate,
}

impl core::fmt::Display for MetricError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let s = match self {
            Self::NotFinite => "metric weight is not finite",
            Self::NotConvex => "metric weight is negative (unit ball not convex)",
            Self::Degenerate => "all metric weights are zero",
        };
        f.write_str(s)
    }
}

impl core::error::Error for MetricError {}

/// `‖v‖₁` — the octahedral norm.
#[inline]
#[must_use]
pub fn norm_l1(v: [f32; 3]) -> f32 {
    v[0].abs() + v[1].abs() + v[2].abs()
}

/// `‖v‖₂` — the Euclidean norm, via the deterministic [`sqrt`].
#[inline]
#[must_use]
pub fn norm_l2(v: [f32; 3]) -> f32 {
    sqrt(v[0] * v[0] + v[1] * v[1] + v[2] * v[2])
}

/// `‖v‖∞` — the cubic (Chebyshev) norm.
#[inline]
#[must_use]
pub fn norm_linf(v: [f32; 3]) -> f32 {
    let a = v[0].abs();
    let b = v[1].abs();
    let c = v[2].abs();
    let m = if a > b { a } else { b };
    if m > c {
        m
    } else {
        c
    }
}

/// A norm built as a non-negative combination of `‖·‖₁`, `‖·‖₂` and `‖·‖∞`.
///
/// The weights are private: a triple that is not a metric must not exist, so
/// the only way in is [`MetricWeights::new`], which validates. The three
/// bases are available as [`L1`](Self::L1), [`L2`](Self::L2) and
/// [`LINF`](Self::LINF), and [`L2`](Self::L2) reproduces the Euclidean field
/// bit for bit (weights `0, 1, 0` contribute `0·n` terms, which are exact).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MetricWeights {
    l1: f32,
    l2: f32,
    linf: f32,
}

impl Default for MetricWeights {
    /// The Euclidean metric — the only default that leaves a field unchanged.
    fn default() -> Self {
        Self::L2
    }
}

impl MetricWeights {
    /// Pure `‖·‖₁`: the unit ball is an octahedron.
    pub const L1: Self = Self {
        l1: 1.0,
        l2: 0.0,
        linf: 0.0,
    };
    /// Pure `‖·‖₂`: the unit ball is a sphere, the field is unchanged.
    pub const L2: Self = Self {
        l1: 0.0,
        l2: 1.0,
        linf: 0.0,
    };
    /// Pure `‖·‖∞`: the unit ball is a cube.
    pub const LINF: Self = Self {
        l1: 0.0,
        l2: 0.0,
        linf: 1.0,
    };

    /// Builds a metric from three weights.
    ///
    /// The weights are kept as given (not rescaled): the caller decides
    /// whether to [`normalised`](Self::normalised) them.
    ///
    /// # Errors
    ///
    /// [`MetricError::NotFinite`] if any weight is NaN or infinite,
    /// [`MetricError::NotConvex`] if any weight is negative — a negative
    /// weight dents the unit ball inwards and the triangle inequality stops
    /// holding — and [`MetricError::Degenerate`] if all three are zero.
    #[inline]
    pub fn new(l1: f32, l2: f32, linf: f32) -> Result<Self, MetricError> {
        if !l1.is_finite() || !l2.is_finite() || !linf.is_finite() {
            return Err(MetricError::NotFinite);
        }
        if l1 < 0.0 || l2 < 0.0 || linf < 0.0 {
            return Err(MetricError::NotConvex);
        }
        if l1 + l2 + linf <= 0.0 {
            return Err(MetricError::Degenerate);
        }
        Ok(Self { l1, l2, linf })
    }

    /// The three weights, in the order `(l1, l2, linf)`.
    #[inline]
    #[must_use]
    pub const fn weights(self) -> (f32, f32, f32) {
        (self.l1, self.l2, self.linf)
    }

    /// `g(v) = w₁‖v‖₁ + w₂‖v‖₂ + w∞‖v‖∞`, summed left to right.
    ///
    /// The order is part of the law: a port that sums right to left is a
    /// different function in the last ulp. A non-finite `v` gives a
    /// non-finite result; where that is a NaN it is the canonical one (a
    /// zero weight times an infinite norm raises the default NaN, whose sign
    /// differs between targets).
    #[inline]
    #[must_use]
    pub fn norm(self, v: [f32; 3]) -> f32 {
        canonical(self.l1 * norm_l1(v) + self.l2 * norm_l2(v) + self.linf * norm_linf(v))
    }

    /// `max_{‖h‖₂=1} g(h)` — how fast the field can change per unit of
    /// Euclidean distance.
    ///
    /// Above `1` the field over-reports distance: a ray marcher that steps by
    /// the reported value steps past the surface, and thin geometry is
    /// pierced. [`normalised`](Self::normalised) divides that away.
    #[inline]
    #[must_use]
    pub fn lipschitz(self) -> f32 {
        let s = self.l1 + self.linf;
        sqrt(s * s + 2.0 * self.l1 * self.l1) + self.l2
    }

    /// `min_{‖h‖₂=1} g(h)` — the smallest the field can report for a unit
    /// Euclidean step.
    ///
    /// Below `1` the field under-reports: nothing is pierced, but a ray
    /// marcher takes proportionally more steps to cover the same ground.
    /// Always strictly positive for a value built by [`new`](Self::new).
    #[inline]
    #[must_use]
    pub fn minimum(self) -> f32 {
        let k1 = self.l1 + self.linf;
        let k2 = (2.0 * self.l1 + self.linf) / SQRT_2;
        let k3 = (3.0 * self.l1 + self.linf) / SQRT_3;
        let m = if k1 < k2 { k1 } else { k2 };
        let m = if m < k3 { m } else { k3 };
        m + self.l2
    }

    /// The Euclidean radius reached by a ball of radius `r` in this metric.
    ///
    /// `r / minimum()`. This is the factor an axis-aligned bound has to grow
    /// by before it can be trusted under this metric — `√3` for
    /// [`LINF`](Self::LINF), `1` for [`L1`](Self::L1) and [`L2`](Self::L2).
    #[inline]
    #[must_use]
    pub fn euclidean_radius(self, r: f32) -> f32 {
        r / self.minimum()
    }

    /// The same metric scaled to be 1-Lipschitz.
    ///
    /// Dividing by [`lipschitz`](Self::lipschitz) — which is homogeneous of
    /// degree one in the weights — is the correction that stops a field from
    /// over-reporting distance. The shape of the unit ball is unchanged, only
    /// its size.
    #[inline]
    #[must_use]
    pub fn normalised(self) -> Self {
        let k = self.lipschitz();
        Self {
            l1: self.l1 / k,
            l2: self.l2 / k,
            linf: self.linf / k,
        }
    }
}

/// `(1 − t)·a + t·b`, exact at both endpoints.
///
/// The other common spelling, `a + t·(b − a)`, is monotone in `t` but returns
/// `a + (b − a)` at `t = 1`, which is not `b` in general. For a blended
/// region that matters more than monotonicity: outside the blend the weight is
/// exactly `1` or exactly `0`, and the field there has to be the untouched
/// original, not the original plus a rounding error.
///
/// `t` outside `[0, 1]` extrapolates. An infinite `a` or `b` gives the
/// canonical NaN at the opposite endpoint (`0 · ∞`).
#[inline]
#[must_use]
pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
    canonical((1.0 - t) * a + t * b)
}

/// `x` restricted to `[low, high]`.
///
/// NaN passes through unchanged (`f32::clamp` would too, but it also panics on
/// inverted bounds). With `low > high` the low bound wins, which keeps the
/// function total.
#[inline]
#[must_use]
pub fn clamp(x: f32, low: f32, high: f32) -> f32 {
    if x.is_nan() {
        return x;
    }
    let y = if x > high { high } else { x };
    if y < low {
        low
    } else {
        y
    }
}

/// The cubic Hermite blend `3t² − 2t³` with `t = clamp((x − e0)/(e1 − e0), 0, 1)`.
///
/// Flat at both edges (`s'(e0) = s'(e1) = 0`) and symmetric about the
/// midpoint. Its steepest slope is `1.5/(e1 − e0)` at the midpoint, so a
/// transition of width `W` contributes at most `1.5/W` to the gradient of
/// whatever it blends — the number that decides whether a blended field stays
/// within its Lipschitz budget.
///
/// A degenerate or inverted interval (`e1 ≤ e0`) is treated as a step at
/// `e0`: below it `0`, at or above it `1`, with no division. NaN in `x`
/// propagates with its payload; a NaN the division created is canonicalised.
#[inline]
#[must_use]
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    if x.is_nan() {
        return x;
    }
    if edge1 <= edge0 || edge1.is_nan() || edge0.is_nan() {
        return if x < edge0 { 0.0 } else { 1.0 };
    }
    let t = clamp((x - edge0) / (edge1 - edge0), 0.0, 1.0);
    canonical(t * t * (3.0 - 2.0 * t))
}
