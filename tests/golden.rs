//! Cross-platform bit pins: a SHA-256 over the output bits of every function
//! on a fixed input grid. The constants were recorded on aarch64-apple-darwin
//! and must reproduce on every target CI runs (macOS Intel, Linux x86 / ARM,
//! Windows MSVC, wasm32, and the SIMD fallback paths). A mismatch on one
//! target means an operation in that function is not IEEE-basic on that
//! target — it is a bug in this crate, never a reason to relax the pin.
//!
//! The input grids are built from integer arithmetic and bit patterns only —
//! a grid generated with the platform `powf` would itself differ per target.
//!
//! `DET_MATH_PRINT_GOLDEN=1 cargo test --test golden -- --nocapture` prints
//! the hashes for re-pinning after an intentional algorithm change. Update the
//! per-function hashes below *before* re-pinning
//! [`alice_det_math::SEMANTICS_ID`]: that value is folded from this table as
//! it stands in the source, so a re-pin run before the table is updated prints
//! the old value and looks as though the identifier missed the change.

#![allow(clippy::cast_precision_loss)]

use sha2::{Digest, Sha256};

fn grid32() -> Vec<f32> {
    let mut v = Vec::with_capacity(300_000);
    for i in 0..=100_000u32 {
        v.push(-200.0 + 400.0 * (i as f32 / 100_000.0));
    }
    // log-spaced without libm: a linear walk over the bit patterns from the
    // smallest subnormal to f32::MAX is geometric in value
    for i in 0..=50_000u32 {
        let bits = 1 + (0x7f7f_fffe / 50_000) * i;
        v.push(f32::from_bits(bits));
        v.push(f32::from_bits(bits | 0x8000_0000));
    }
    // xorshift32 over the whole bit space (NaN payloads, subnormals, huge)
    let mut s = 0x2545_f491u32;
    for _ in 0..100_000 {
        s ^= s << 13;
        s ^= s >> 17;
        s ^= s << 5;
        v.push(f32::from_bits(s));
    }
    v
}

fn grid64() -> Vec<f64> {
    let mut v = Vec::with_capacity(200_000);
    for i in 0..=100_000u32 {
        v.push(-800.0 + 1600.0 * (f64::from(i) / 100_000.0));
    }
    for i in 0..=50_000u64 {
        v.push(f64::from_bits(1 + (0x7fef_ffff_ffff_fffe / 50_000) * i));
    }
    let mut s = 0x9e37_79b9_7f4a_7c15u64;
    for _ in 0..50_000 {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        v.push(f64::from_bits(s));
    }
    v
}

fn hash32(xs: &[f32], f: impl Fn(f32) -> f32) -> String {
    let mut h = Sha256::new();
    for &x in xs {
        h.update(f(x).to_bits().to_le_bytes());
    }
    format!("{:x}", h.finalize())
}

fn hash32x2(xs: &[f32], f: impl Fn(f32, f32) -> f32) -> String {
    let mut h = Sha256::new();
    for (i, &x) in xs.iter().enumerate() {
        let y = xs[(i * 7 + 3) % xs.len()];
        h.update(f(x, y).to_bits().to_le_bytes());
    }
    format!("{:x}", h.finalize())
}

fn hash32x3(xs: &[f32], f: impl Fn(f32, f32, f32) -> f32) -> String {
    let mut h = Sha256::new();
    for (i, &x) in xs.iter().enumerate() {
        let y = xs[(i * 7 + 3) % xs.len()];
        let z = xs[(i * 13 + 11) % xs.len()];
        h.update(f(x, y, z).to_bits().to_le_bytes());
    }
    format!("{:x}", h.finalize())
}

fn hash64(xs: &[f64], f: impl Fn(f64) -> f64) -> String {
    let mut h = Sha256::new();
    for &x in xs {
        h.update(f(x).to_bits().to_le_bytes());
    }
    format!("{:x}", h.finalize())
}

/// A metric's three weights folded into one `f32`, so that one pin can fix
/// all three. The multipliers are powers of two and the sum runs left to
/// right, so every operation is IEEE-basic and the fold itself cannot differ
/// between targets.
fn fold_weights(w: alice_det_math::metric::MetricWeights) -> f32 {
    let (l1, l2, linf) = w.weights();
    l1 + 2.0 * l2 + 4.0 * linf
}

fn check(name: &str, got: &str, want: &str) {
    if std::env::var_os("DET_MATH_PRINT_GOLDEN").is_some() {
        // re-pin mode: print every hash instead of stopping at the first mismatch
        println!("    (\"{name}\", \"{got}\"),");
        return;
    }
    assert_eq!(got, want, "golden mismatch for {name}");
}

const GOLDEN: &[(&str, &str)] = &[
    (
        "smoothstep",
        "d43e5a4972e13773caf1da15db30501850e505f361fd7526c908c35ff8e7cc7b",
    ),
    (
        "metric_norm",
        "00f35aac72ff55e1069259196427d703926b47f7b4349fb4cc7f3f7b8effdc60",
    ),
    (
        "metric_lipschitz",
        "566d51df6ba7ba7358f51c25b8c19b8ad85879caca497bd6f6f22ad8d332076e",
    ),
    (
        "norm_l1",
        "d42b5308210f6ccd35b2b30c7d147f4e500e702babfc88f9940911687c652cc3",
    ),
    (
        "norm_l2",
        "69e09bc27dee17bbdc9eeeb4ab0f984f6539e4c487463c725b34b9f48426dbf9",
    ),
    (
        "norm_linf",
        "803bba978f40181193b704539f30a458d85e487116ccab92ae2a1083de7934f9",
    ),
    (
        "lerp",
        "fe010775a3074825cbf8e3b28cf598d7e7d3a598e7c3da185f7f9525c7b6e6a6",
    ),
    (
        "clamp",
        "64f40a7cb1c1b64a8b1c9d8c662d7a2a58c8f8d8afda24e9808d5520c2bca267",
    ),
    (
        "metric_new",
        "604816265b1dd7bae1a41e479345ee0a4280b59eb46d229c21393a7121aa1849",
    ),
    (
        "metric_weights",
        "087ef0b899081bebeff373bffaee6df5c675b7851b2c78da12c3a8192fd7a927",
    ),
    (
        "metric_minimum",
        "7c8f001eef1728c8b70b22396b3c7f7c4deeeb8bce252226ea0fc7d0ecf16927",
    ),
    (
        "metric_axis_extent",
        "71be828d7ad082fc09e0a4d50c0f1111fab3af528cfa31d93b0af81f759e5632",
    ),
    (
        "metric_euclidean_radius",
        "ab01bbc9917308157470c5cf5d8fdc7f1025cbc339d619682a17ed5646b1341c",
    ),
    (
        "metric_normalised",
        "75ac63304282eaf0596bc81a30d77eb0324e412332b13b578bc04c5a7f28556e",
    ),
    (
        "sin",
        "5e51709ffaa05b16afe14f2268c97633a0058eee46ff2f563e24fb8426967ef1",
    ),
    (
        "cos",
        "d80b955d724c77e2d64c58dab6e5dc32cbca3a8274db4b9065f1d8528011f896",
    ),
    (
        "sin_cos",
        "e8fbc874df490f32d4bbd895cfa30b0d42c3624a58f042863e08ec6becd145fb",
    ),
    (
        "exp",
        "fa74cbe1433a80c6a2c1f02332275bdd624da09f773c69877696d989948010c8",
    ),
    (
        "ln",
        "f740b2d9021d091ae3f5a18ecf346fbe5205b2d3b0209e96dca70b710da2e924",
    ),
    (
        "log2",
        "31911ba674cb7bce895b09de45c8d9ea1a84a5c26ba5936ef5a01faa05ac551e",
    ),
    (
        "log10",
        "2acdd36905c9d2ae3bb1d97db141d37f6c4823f630703fd8efede750a5c1caa1",
    ),
    (
        "powf",
        "aeeb77e2053b833490655db02c832d6305502dfb873b2bbb03b2557f64460e0e",
    ),
    (
        "powi",
        "769743b0123bb1c3926c8ec7445002b3de11fb6476f438dc67139313c2064117",
    ),
    (
        "cbrt",
        "b363cc6aa6be74c982b4351f0db1700a31d65ef68a00ab60503f495a9278ea65",
    ),
    (
        "hypot",
        "0cae2245259eaa5ec7267c3abe0db438088ebf5f53d33e28319bfe2579a3c4cd",
    ),
    (
        "atan",
        "3c5b4e38c7b09d816a5815073c8128d698b92173c2c443eef9a12fb06043dc78",
    ),
    (
        "atan2",
        "6fcbd5192a639e6e9b6b431485a4c3421b009b28f5d667b8a263124bf3ea28a4",
    ),
    (
        "asin",
        "ca20e5817f24d9970dca42171443c33af93ed8123c2eb9bb3441cd5427fb2fb7",
    ),
    (
        "acos",
        "b2e19e15112788c5523f4d7e53fcc143b58a1f24be543ffc76219c430fe56a56",
    ),
    (
        "tan",
        "ba94160fda3f50340c31f1abe034701d4bda9993cf0608635e802db72a9164fb",
    ),
    (
        "tanh",
        "8ae2d42020033aebfa823eb7ff3996d03ce8c0b4f5fd1f38c58d6197af1a3ff4",
    ),
    (
        "round",
        "7e92a4c7958c2972273ae47988f4818dfe01d16a30803e9b7cd3f06f14261359",
    ),
    (
        "sqrt",
        "64877c5a8a670097edf21dd0aeca4faf56ad9925952e7c48cb460ea5b7fb829d",
    ),
    (
        "atan64",
        "110c479ea68f1a9acd63fef9328f388d5ddf39ac1082274e9a35855dd60784e1",
    ),
    (
        "atan2_64",
        "f2263508c440333a745930d61fb5b65ab7efa441f7846f7048ca701af46caa26",
    ),
    (
        "asin64",
        "3f1953ff567f5d6c4e69b61a0b0b31100528d0f837788693a5617bf042dcc32d",
    ),
    (
        "acos64",
        "fe196c16623f5b61946c47cb78b286ca6ee1ad04233a5e438dd90b7c4e9fa43b",
    ),
    (
        "exp64",
        "a3782891336fb2cf0385790ea3589fd05fce413c4bbf5252048a6f8b72d54a39",
    ),
    (
        "ln64",
        "053aec9251fe146bb064b9a66a5f662ca59eb31b8507b9a50f27135f39dfc8c9",
    ),
    (
        "log2_64",
        "c3586ee8c0d3d001f36b1d57846cc1172d0b42b7629588054c050fc74126fd07",
    ),
    (
        "log10_64",
        "b3e8502088c819da30e0f12076d08b0fc52ef6f3496970b4600d1ff2f1c97934",
    ),
    (
        "powf64",
        "6bd59fa5f716c2adc9d5d9a4bdecf4f600889587ba0b5a927d4826fa1b593969",
    ),
    (
        "round64",
        "5e10f4fa0a2251cca43b0caf760c57906703bb9de8008efc6652195a10fddef2",
    ),
    (
        "sqrt64",
        "45b138399157594f7ce1e36a01940a185ec7f04de57fe041e47ecf99aa40bf00",
    ),
    (
        "sin64",
        "f67cfac4494c1bc738191d44db12d9a02df667dc86c5bf9471dded124824b367",
    ),
    (
        "cos64",
        "c8b013bf72744ff237e34a15b5cb05c3b33391970917db0074446fd70b349377",
    ),
    (
        "tan64",
        "bb8669e863c925b799c8d73eaf50965b2e7c60eab00a9bacbc24644025d7d3d6",
    ),
    (
        "sin64_dense",
        "ce7f4729ce2f51e9c0fccd9657ffee833d04cfdb4497f72b4ab70da89e0dd525",
    ),
    (
        "cos64_dense",
        "4de5c741eb8cb0fb9cfea9aeda36ffeebbd0de8b4f1ad3b697f0ed9eb6e60918",
    ),
    (
        "tan64_dense",
        "cf5945d72c4e963e438588ff84821a51bc2b2d909db13239dea400667f6ea80e",
    ),
    (
        "sin_cos64",
        "37e1a316a90d2f09b318b52950c4ddaa2103b6410a075b1b43ac70101debc1a8",
    ),
];

fn want(name: &str) -> &'static str {
    GOLDEN
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, h)| *h)
        .expect("golden entry")
}

#[test]
fn scalar_outputs_match_recorded_hashes() {
    use alice_det_math::*;
    let g = grid32();
    check("sin", &hash32(&g, sin), want("sin"));
    check("cos", &hash32(&g, cos), want("cos"));
    check(
        "sin_cos",
        &hash32(&g, |x| {
            let (s, c) = sin_cos(x);
            f32::from_bits(s.to_bits() ^ c.to_bits().rotate_left(16))
        }),
        want("sin_cos"),
    );
    check("exp", &hash32(&g, exp), want("exp"));
    check("ln", &hash32(&g, ln), want("ln"));
    check("log2", &hash32(&g, log2), want("log2"));
    check("log10", &hash32(&g, log10), want("log10"));
    check("powf", &hash32x2(&g, powf), want("powf"));
    check(
        "powi",
        &hash32(&g, |x| powi(x, (x.to_bits() % 31) as i32 - 15)),
        want("powi"),
    );
    check("cbrt", &hash32(&g, cbrt), want("cbrt"));
    check("hypot", &hash32x2(&g, hypot), want("hypot"));
    check("atan", &hash32(&g, atan), want("atan"));
    check("atan2", &hash32x2(&g, atan2), want("atan2"));
    check("asin", &hash32(&g, asin), want("asin"));
    check("acos", &hash32(&g, acos), want("acos"));
    check("tan", &hash32(&g, tan), want("tan"));
    check("tanh", &hash32(&g, tanh), want("tanh"));
    check("round", &hash32(&g, round), want("round"));
    check("sqrt", &hash32(&g, sqrt), want("sqrt"));
    check(
        "smoothstep",
        &hash32x3(&g, metric::smoothstep),
        want("smoothstep"),
    );
    // a mixed metric exercises all three bases and the left-to-right sum
    let mixed = metric::MetricWeights::new(0.3, 0.5, 0.2).unwrap();
    check(
        "metric_norm",
        &hash32x3(&g, |x, y, z| mixed.norm([x, y, z])),
        want("metric_norm"),
    );
    check(
        "metric_lipschitz",
        &hash32x3(&g, |a, b, c| {
            metric::MetricWeights::new(a.abs(), b.abs(), c.abs())
                .map_or(f32::NAN, metric::MetricWeights::lipschitz)
        }),
        want("metric_lipschitz"),
    );
    check(
        "norm_l1",
        &hash32x3(&g, |x, y, z| metric::norm_l1([x, y, z])),
        want("norm_l1"),
    );
    check(
        "norm_l2",
        &hash32x3(&g, |x, y, z| metric::norm_l2([x, y, z])),
        want("norm_l2"),
    );
    check(
        "norm_linf",
        &hash32x3(&g, |x, y, z| metric::norm_linf([x, y, z])),
        want("norm_linf"),
    );
    check("lerp", &hash32x3(&g, metric::lerp), want("lerp"));
    check("clamp", &hash32x3(&g, metric::clamp), want("clamp"));
    // the acceptance rule itself is part of the semantics: which triples are
    // a metric decides which fields exist, so each rejection is folded in
    // under its own value rather than collapsing to one "rejected"
    check(
        "metric_new",
        &hash32x3(&g, |a, b, c| match metric::MetricWeights::new(a, b, c) {
            Ok(w) => fold_weights(w),
            Err(metric::MetricError::NotFinite) => -1.0,
            Err(metric::MetricError::NotConvex) => -2.0,
            Err(metric::MetricError::Degenerate) => -3.0,
            Err(_) => -4.0,
        }),
        want("metric_new"),
    );
    // `weights` returns what the named metrics and `Default` store, so this
    // pin is also what makes `L1` / `L2` / `LINF` / `default()` — public
    // values this parser does not see, being consts and a trait impl — part
    // of the identifier
    check(
        "metric_weights",
        &hash32x3(&g, |a, b, c| {
            let named = fold_weights(metric::MetricWeights::L1)
                + 8.0 * fold_weights(metric::MetricWeights::L2)
                + 64.0 * fold_weights(metric::MetricWeights::LINF)
                + 512.0 * fold_weights(metric::MetricWeights::default());
            let built = metric::MetricWeights::new(a.abs(), b.abs(), c.abs()).unwrap_or_default();
            named + 4096.0 * fold_weights(built)
        }),
        want("metric_weights"),
    );
    check(
        "metric_minimum",
        &hash32x3(&g, |a, b, c| {
            metric::MetricWeights::new(a.abs(), b.abs(), c.abs())
                .map_or(f32::NAN, metric::MetricWeights::minimum)
        }),
        want("metric_minimum"),
    );
    check(
        "metric_axis_extent",
        &hash32x3(&g, |a, b, c| {
            metric::MetricWeights::new(a.abs(), b.abs(), c.abs())
                .map_or(f32::NAN, |w| w.axis_extent(a))
        }),
        want("metric_axis_extent"),
    );
    check(
        "metric_euclidean_radius",
        &hash32x3(&g, |a, b, c| {
            metric::MetricWeights::new(a.abs(), b.abs(), c.abs())
                .map_or(f32::NAN, |w| w.euclidean_radius(b))
        }),
        want("metric_euclidean_radius"),
    );
    check(
        "metric_normalised",
        &hash32x3(&g, |a, b, c| {
            metric::MetricWeights::new(a.abs(), b.abs(), c.abs())
                .map_or(f32::NAN, |w| fold_weights(w.normalised()))
        }),
        want("metric_normalised"),
    );
    let g = grid64();
    check("atan64", &hash64(&g, atan64), want("atan64"));
    check(
        "atan2_64",
        &hash64(&g, |x| atan2_64(x, x * 0.37 - 1.0)),
        want("atan2_64"),
    );
    check("asin64", &hash64(&g, asin64), want("asin64"));
    check("acos64", &hash64(&g, acos64), want("acos64"));
    check("exp64", &hash64(&g, exp64), want("exp64"));
    check("ln64", &hash64(&g, ln64), want("ln64"));
    check("log2_64", &hash64(&g, log2_64), want("log2_64"));
    check("log10_64", &hash64(&g, log10_64), want("log10_64"));
    check("tan64", &hash64(&g, tan64), want("tan64"));
    check(
        "powf64",
        &hash64(&g, |x| powf64(x.abs(), x * 1.0e-2)),
        want("powf64"),
    );
    check("round64", &hash64(&g, round64), want("round64"));
    check("sqrt64", &hash64(&g, sqrt64), want("sqrt64"));
    check("sin64", &hash64(&g, sin64), want("sin64"));
    check("cos64", &hash64(&g, cos64), want("cos64"));
    check(
        "sin_cos64",
        &hash64(&g, |x| {
            let (s, c) = sin_cos64(x);
            f64::from_bits(s.to_bits() ^ c.to_bits().rotate_left(32))
        }),
        want("sin_cos64"),
    );
    // dense over the first reduction cases (|x| ≤ 9π/4 and a little beyond),
    // where the coarse grid has only a few hundred points
    let dense: Vec<f64> = (0..=400_000u32)
        .map(|i| -10.0 + 20.0 * (f64::from(i) / 400_000.0))
        .collect();
    check("sin64_dense", &hash64(&dense, sin64), want("sin64_dense"));
    check("cos64_dense", &hash64(&dense, cos64), want("cos64_dense"));
    // the tangent's kernel branches on |reduced| ≥ 0.6744 and on the parity
    // of the quadrant, so the dense grid covers all four combinations
    check("tan64_dense", &hash64(&dense, tan64), want("tan64_dense"));
}

// ---------------------------------------------------------------------------
// SEMANTICS_ID: the table above, folded into one identifier
// ---------------------------------------------------------------------------

/// The 32 bytes a 64-character hex string stands for; `Err` if the string is
/// not exactly 64 hex digits (a malformed entry must not fold silently).
fn hex32(s: &str) -> Result<[u8; 32], String> {
    let b = s.as_bytes();
    if b.len() != 64 {
        return Err(format!("hash {s:?} is {} chars, not 64", b.len()));
    }
    let mut out = [0u8; 32];
    for (i, byte) in out.iter_mut().enumerate() {
        let hi = (b[2 * i] as char).to_digit(16).ok_or("not a hex digit")?;
        let lo = (b[2 * i + 1] as char)
            .to_digit(16)
            .ok_or("not a hex digit")?;
        *byte = (hi * 16 + lo) as u8;
    }
    Ok(out)
}

/// Fold a table of `(name, hash)` pins into one identifier:
/// `SHA256` over, for each entry in ascending byte order of the name, the
/// name's length as four big-endian bytes, then the name, then the 32 bytes
/// of its hash.
///
/// The length prefix is what makes the concatenation unambiguous: without it
/// two different tables can produce the same byte stream (pinned by
/// [`the_length_prefix_separates_tables_that_would_otherwise_collide`]).
/// Sorting by name means the result does not depend on the order the entries
/// happen to be written in, and a repeated name is rejected rather than
/// ordered arbitrarily.
fn fold_semantics_id(entries: &[(&str, &str)]) -> Result<[u8; 32], String> {
    if entries.is_empty() {
        return Err("the table is empty: there would be nothing to identify".into());
    }
    let mut sorted = entries.to_vec();
    sorted.sort_unstable_by_key(|(name, _)| *name);
    for pair in sorted.windows(2) {
        if pair[0].0 == pair[1].0 {
            return Err(format!("`{}` appears twice: no unique order", pair[0].0));
        }
    }
    let mut h = Sha256::new();
    for (name, hash) in sorted {
        if name.is_empty() || !name.is_ascii() {
            return Err(format!("name {name:?} is empty or not ASCII"));
        }
        let len = u32::try_from(name.len()).map_err(|e| e.to_string())?;
        h.update(len.to_be_bytes());
        h.update(name.as_bytes());
        h.update(hex32(hash)?);
    }
    Ok(h.finalize().into())
}

// ---------------------------------------------------------------------------
// The public numeric surface, read out of the sources
// ---------------------------------------------------------------------------

/// Every source file the crate compiles, with the module path it provides
/// (`""` for the crate root).
///
/// Reading the *definitions* is what keeps the coverage check honest. An
/// earlier version read only the `pub use <module>::{ … }` lists in
/// `src/lib.rs`, so it saw the 34 re-exported transcendentals and nothing
/// else: the whole `metric` module reaches callers as `pub mod metric` with
/// no re-export, and 11 of its entry points had no pin at all — their bits
/// could change without moving [`alice_det_math::SEMANTICS_ID`].
/// [`every_module_declaration_is_parsed`] keeps this table complete, so a new
/// module file cannot repeat that.
const SOURCES: &[(&str, &str)] = &[
    ("", include_str!("../src/lib.rs")),
    ("double", include_str!("../src/double.rs")),
    ("double::trig", include_str!("../src/double/trig.rs")),
    ("metric", include_str!("../src/metric.rs")),
    ("ops", include_str!("../src/ops.rs")),
    ("simd", include_str!("../src/simd.rs")),
    ("semantics", include_str!("../src/semantics.rs")),
    ("single", include_str!("../src/single.rs")),
];

/// One public function of the crate.
struct Surface {
    /// The module path it is defined in, as spelled in [`SOURCES`].
    module: &'static str,
    /// The type it is a method of, or `None` for a free function.
    ty: Option<String>,
    /// The function's own name.
    name: String,
}

impl Surface {
    /// How this function is written in Rust, for failure messages.
    fn id(&self) -> String {
        match (&self.ty, self.module) {
            (Some(t), "") => format!("{t}::{}", self.name),
            (Some(t), m) => format!("{m}::{t}::{}", self.name),
            (None, "") => self.name.clone(),
            (None, m) => format!("{m}::{}", self.name),
        }
    }

    /// The pin in [`GOLDEN`] that fixes its output bits: a free function is
    /// pinned under its own name (`sin`, `lerp`), a method under
    /// `<module>_<method>` (`metric_norm`), since method names alone
    /// (`new`, `norm`) would not say what they belong to.
    fn pin(&self) -> String {
        if self.ty.is_none() {
            self.name.clone()
        } else {
            let last = self.module.rsplit("::").next().unwrap_or(self.module);
            format!("{last}_{}", self.name)
        }
    }
}

/// Whether `hay` names `needle` as a whole path: the character after the
/// match must not continue the identifier, so looking for `simd::sin` is not
/// satisfied by `simd::sin_cos`.
fn mentions(hay: &str, needle: &str) -> bool {
    hay.match_indices(needle).any(|(i, _)| {
        hay[i + needle.len()..]
            .chars()
            .next()
            .is_none_or(|c| !c.is_ascii_alphanumeric() && c != '_')
    })
}

/// The type an indented `pub fn` belongs to: the subject of the last `impl …`
/// line at column zero before it. `impl Trait for Type` names `Type`.
/// The parser must read a source the same way whichever line ending it has.
///
/// Windows checkouts hold `\r\n`, so a parser that walks byte offsets while
/// taking its lines from `str::lines` (which drops the `\r`) falls one byte
/// behind per line. The shape that breaks is a file whose only `impl` sits far
/// down with its method just below it: by then the accumulated shortfall is
/// larger than the distance from the `impl` to the method, the window handed to
/// [`enclosing_type`] ends above the `impl`, and the method looks like a free
/// function defined at an impossible indent — a panic on Windows alone, with
/// every other target green. `src/simd.rs` has exactly that shape.
///
/// The fixture reproduces it without depending on the real sources staying that
/// way, and the assertion is equality between the two line endings rather than
/// a fixed expected list, so it keeps its teeth as the parser grows.
#[test]
fn the_parser_does_not_depend_on_the_line_ending() {
    use std::fmt::Write as _;

    let mut src = String::new();
    for i in 0..400 {
        writeln!(src, "// filler {i}").expect("writing to a String cannot fail");
    }
    src.push_str("impl Widget {\n");
    src.push_str("    /// doc\n");
    src.push_str("    #[inline]\n");
    src.push_str("    pub fn method(x: f64) -> f64 {\n        x\n    }\n");
    src.push_str("}\n");
    src.push_str("pub fn free(x: f64) -> f64 {\n    x\n}\n");

    let crlf = src.replace('\n', "\r\n");
    assert!(crlf.len() > src.len(), "the CRLF fixture must differ");

    let lf_out = parsed_fns("fixture", &src);
    let crlf_out = parsed_fns("fixture", &crlf);

    // the fixture has to actually exercise both arms, or the test proves nothing
    assert_eq!(
        lf_out,
        vec![
            (Some("Widget".to_owned()), "method".to_owned()),
            (None, "free".to_owned()),
        ],
        "the LF fixture no longer parses as one method and one free function"
    );
    assert_eq!(
        crlf_out, lf_out,
        "the parser reads CRLF input differently from LF input"
    );
}

fn enclosing_type(src: &str, upto: usize) -> Option<&str> {
    src[..upto]
        .lines()
        .rfind(|l| l.starts_with("impl "))
        .map(|l| {
            let head = l.trim_end().trim_end_matches('{').trim_end();
            let subject = head.rsplit(" for ").next().unwrap_or(head);
            let subject = subject.rsplit(' ').next().unwrap_or(subject);
            subject.split('<').next().unwrap_or(subject)
        })
}

/// Every `pub fn` / `pub const fn` the crate defines, parsed out of
/// [`SOURCES`] and sorted.
///
/// Free functions sit at column zero; a method is indented inside an `impl`
/// block. An indented `pub fn` with no `impl` above it is something this
/// parser does not understand, so it panics instead of dropping the function
/// silently — a dropped function is a function with no pin.
fn public_numeric_surface() -> Vec<Surface> {
    let mut out = Vec::new();
    for (module, src) in SOURCES {
        if IDENTIFIER_MODULES.contains(module) {
            continue;
        }
        for (ty, name) in parsed_fns(module, src) {
            out.push(Surface { module, ty, name });
        }
    }
    out.sort_unstable_by_key(Surface::id);
    out
}

/// One source's `(enclosing type, function name)` pairs, in source order.
///
/// Split out of [`public_numeric_surface`] so that the parser can be fed text
/// directly: [`the_parser_does_not_depend_on_the_line_ending`] runs the same
/// source through it with both line endings and requires the same answer.
///
/// The byte offsets this walks are the reason that test exists. `str::lines`
/// drops a trailing `\r`, so advancing by `line.len()` plus one byte is short
/// by one byte per line on CRLF input, and the window handed to
/// [`enclosing_type`] slides away from the `impl` it is supposed to find.
/// Taking the terminator with the line keeps the arithmetic true for both.
fn parsed_fns(module: &str, src: &str) -> Vec<(Option<String>, String)> {
    let mut out = Vec::new();
    {
        let mut at = 0usize;
        for raw in src.split_inclusive('\n') {
            let start = at;
            at += raw.len();
            let line = raw.trim_end_matches(['\n', '\r']);
            let body = line.trim_start();
            let Some(rest) = body
                .strip_prefix("pub fn ")
                .or_else(|| body.strip_prefix("pub const fn "))
            else {
                continue;
            };
            let name = rest
                .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                .next()
                .unwrap_or_default();
            assert!(!name.is_empty(), "`pub fn` with no name in `{module}`");
            let ty = if line.len() == body.len() {
                None
            } else {
                Some(
                    enclosing_type(src, start)
                        .unwrap_or_else(|| {
                            panic!("indented `pub fn {name}` in `{module}` has no `impl` above it")
                        })
                        .to_owned(),
                )
            };
            assert!(
                !module.is_empty(),
                "the crate root defines `{name}`; extend this parser to see it"
            );
            out.push((ty, name.to_owned()));
        }
    }
    out
}

/// Modules whose functions compare arithmetic identifiers and compute no
/// floating-point value, so they have no bits to pin and are not part of the
/// numeric surface. [`identifier_modules_compute_no_float`] keeps that true.
const IDENTIFIER_MODULES: &[&str] = &["semantics"];

/// The excluded modules define exactly the functions expected, and none of
/// them returns or takes a float: a numeric function added there would
/// otherwise escape the pins.
#[test]
fn identifier_modules_compute_no_float() {
    let mut seen = Vec::new();
    for (module, src) in SOURCES {
        if !IDENTIFIER_MODULES.contains(module) {
            continue;
        }
        for (ty, name) in parsed_fns(module, src) {
            seen.push(format!(
                "{module}::{}{name}",
                ty.map(|t| t + "::").unwrap_or_default()
            ));
        }
        for word in ["f32", "f64"] {
            assert!(
                !src.contains(word),
                "`{module}` mentions `{word}`: a float in an identifier module escapes the pins"
            );
        }
    }
    seen.sort();
    assert_eq!(
        seen,
        [
            "semantics::SemanticsCheck::is_current",
            "semantics::SemanticsCheck::is_reproducible",
            "semantics::check_semantics",
        ],
        "identifier module surface changed"
    );
}

/// Pins in [`GOLDEN`] that do not name a public function: the second, denser
/// input grid used for the functions whose first reduction cases the coarse
/// grid barely samples.
const EXTRA_PINS: &[&str] = &["sin64_dense", "cos64_dense", "tan64_dense"];

/// Every module the crate declares is in [`SOURCES`], and every entry in
/// [`SOURCES`] is declared somewhere.
///
/// Without this, a new module file would carry functions
/// [`public_numeric_surface`] never reads, and the coverage check below would
/// pass while saying nothing about them.
#[test]
fn every_module_declaration_is_parsed() {
    let known: Vec<&str> = SOURCES.iter().map(|(m, _)| *m).collect();
    let mut declared: Vec<String> = Vec::new();
    for (module, src) in SOURCES {
        for line in src.lines() {
            let body = line.trim_start();
            let Some(rest) = body
                .strip_prefix("mod ")
                .or_else(|| body.strip_prefix("pub mod "))
            else {
                continue;
            };
            // `mod name { … }` is written in place, so it is already parsed
            let Some(name) = rest.trim_end().strip_suffix(';') else {
                continue;
            };
            if name == "tests" {
                continue;
            }
            declared.push(if module.is_empty() {
                name.to_owned()
            } else {
                format!("{module}::{name}")
            });
        }
    }
    // comparing nothing would pass: there are at least the five modules
    // `lib.rs` declares plus `double::trig`
    assert!(
        declared.len() >= 6,
        "only {} module declarations parsed: {declared:?}",
        declared.len()
    );
    for d in &declared {
        assert!(
            known.contains(&d.as_str()),
            "module `{d}` is declared but missing from SOURCES, so its \
             functions are never read and could have no pin"
        );
    }
    for m in known.iter().filter(|m| !m.is_empty()) {
        assert!(
            declared.iter().any(|d| d == m),
            "SOURCES lists `{m}` but no `mod` declaration names it"
        );
    }
}

/// Every public numeric function has a pin in [`GOLDEN`], and every pin in
/// [`GOLDEN`] corresponds to something.
///
/// Without this, a function added to the crate without a golden entry would
/// leave [`alice_det_math::SEMANTICS_ID`] unchanged while the crate's
/// behaviour grew — the identifier would then not identify the behaviour.
/// Neither direction may compare nothing: the counts are asserted exactly, so
/// a parser that stopped seeing the public surface fails here.
#[test]
fn semantics_id_covers_every_public_numeric_function() {
    let (simd, scalar): (Vec<Surface>, Vec<Surface>) = public_numeric_surface()
        .into_iter()
        .partition(|s| s.module.starts_with("simd"));
    // 48 = 13 `double` (9 of its own + 4 from its private `trig`)
    //    + 4 `ops` + 17 `single`
    //    + 14 `metric` (6 free functions + 8 methods on `MetricWeights`).
    // Adding one means adding its golden pin and raising this number in the
    // same commit.
    assert_eq!(
        scalar.len(),
        48,
        "public numeric surface: {:?}",
        ids(&scalar)
    );
    // `simd` is deliberately not folded into SEMANTICS_ID: it is
    // feature-gated, and an identifier that changed with the feature set
    // would not identify the crate's arithmetic. Its functions add no
    // semantics of their own — each must return the bits its scalar
    // counterpart returns, which `every_simd_function_has_a_parity_test`
    // keeps true.
    assert_eq!(simd.len(), 21, "SIMD surface: {:?}", ids(&simd));
    for sentinel in [
        "single::sin",
        "double::ln64",
        "ops::sqrt",
        "double::trig::tan64",
        "metric::norm_l1",
        "metric::lerp",
        "metric::MetricWeights::axis_extent",
    ] {
        assert!(
            scalar.iter().any(|s| s.id() == sentinel),
            "the parse lost `{sentinel}`: {:?}",
            ids(&scalar)
        );
    }
    let pinned: Vec<&str> = GOLDEN.iter().map(|(n, _)| *n).collect();
    assert_eq!(pinned.len(), 51, "golden pins: {pinned:?}");
    let wanted: Vec<String> = scalar.iter().map(Surface::pin).collect();
    for s in &scalar {
        assert!(
            pinned.contains(&s.pin().as_str()),
            "`{}` is public but has no entry in GOLDEN under `{}`, so a \
             change to it would not move SEMANTICS_ID",
            s.id(),
            s.pin()
        );
    }
    for p in &pinned {
        assert!(
            wanted.iter().any(|w| w == p) || EXTRA_PINS.contains(p),
            "golden pin `{p}` names neither a public function nor a known \
             extra pin (a typo here silently pins nothing)"
        );
    }
}

/// The ids of a surface, for failure messages.
fn ids(surface: &[Surface]) -> Vec<String> {
    surface.iter().map(Surface::id).collect()
}

/// Every `pub fn` in the SIMD module is exercised by `simd_parity.rs`.
///
/// The SIMD paths carry no pin of their own (see
/// [`semantics_id_covers_every_public_numeric_function`]); what covers them
/// is parity with the scalar functions, lane for lane. That argument holds
/// only while every SIMD function actually has a parity test, so a new kernel
/// added without one fails here rather than shipping with nothing pinning its
/// bits.
#[test]
fn every_simd_function_has_a_parity_test() {
    const PARITY: &str = include_str!("simd_parity.rs");
    let simd: Vec<Surface> = public_numeric_surface()
        .into_iter()
        .filter(|s| s.module.starts_with("simd"))
        .collect();
    assert_eq!(simd.len(), 21, "SIMD surface: {:?}", ids(&simd));
    for s in &simd {
        let needle = s.ty.as_ref().map_or_else(
            || format!("simd::{}", s.name),
            |t| format!("{t}::{}", s.name),
        );
        assert!(
            mentions(PARITY, &needle),
            "`{}` has no parity test in simd_parity.rs (looked for `{needle}`), \
             so nothing pins its bits",
            s.id()
        );
    }
}

/// The recorded [`alice_det_math::SEMANTICS_ID`] is what the golden table
/// folds to, so the constant cannot drift from the crate's measured
/// behaviour: any change to any pinned function changes its hash, which
/// changes the fold, which fails here until the constant is updated.
#[test]
fn semantics_id_matches_the_recorded_constant() {
    let got = fold_semantics_id(GOLDEN).expect("the golden table folds");
    let hex = got.iter().fold(String::new(), |mut s, b| {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
        s
    });
    if std::env::var_os("DET_MATH_PRINT_GOLDEN").is_some() {
        // re-pin mode, as for the hashes above
        println!("SEMANTICS_ID = {hex}");
        let bytes = got
            .iter()
            .map(|b| format!("0x{b:02x},"))
            .collect::<Vec<_>>()
            .join(" ");
        println!("    {bytes}");
        return;
    }
    assert_eq!(
        got,
        alice_det_math::SEMANTICS_ID,
        "SEMANTICS_ID is {hex} now; re-pin with DET_MATH_PRINT_GOLDEN=1"
    );
}

/// Folding the same table twice gives the same bytes, and the result does not
/// depend on the order the entries are written in.
#[test]
fn semantics_id_is_stable_and_order_independent() {
    let once = fold_semantics_id(GOLDEN).expect("folds");
    let twice = fold_semantics_id(GOLDEN).expect("folds");
    assert_eq!(once, twice);
    let mut shuffled = GOLDEN.to_vec();
    shuffled.reverse();
    assert_eq!(
        fold_semantics_id(&shuffled).expect("folds"),
        once,
        "the fold depends on the written order"
    );
    // and it is not a constant: a different table gives a different answer
    let shorter = &GOLDEN[1..];
    assert_ne!(
        fold_semantics_id(shorter).expect("folds"),
        once,
        "dropping a pin did not change the fold"
    );
}

/// Every pin in [`GOLDEN`] is load-bearing: perturbing any one of its hashes
/// by a single bit changes the fold. A pin that did not move the identifier
/// would be a function whose behaviour could change silently.
///
/// The five functions added alongside this identifier are named explicitly as
/// well, so that a table which lost one of them fails here rather than
/// quietly folding 35 pins instead of 40.
#[test]
fn every_pin_moves_the_identifier_including_the_newest_functions() {
    let base = fold_semantics_id(GOLDEN).expect("folds");
    for i in 0..GOLDEN.len() {
        let mut table = GOLDEN.to_vec();
        // flip the low bit of the last hex digit of entry `i`
        let (name, hash) = table[i];
        let last = hash.chars().next_back().expect("non-empty hash");
        let flipped = format!(
            "{}{:x}",
            &hash[..hash.len() - 1],
            last.to_digit(16).expect("hex digit") ^ 1
        );
        table[i] = (name, flipped.as_str());
        assert_ne!(
            fold_semantics_id(&table).expect("folds"),
            base,
            "changing `{name}` does not change SEMANTICS_ID"
        );
    }
    for name in ["log2", "log10", "log2_64", "log10_64", "tan64"] {
        assert!(
            GOLDEN.iter().any(|(n, _)| *n == name),
            "`{name}` has no pin, so its behaviour is outside SEMANTICS_ID"
        );
    }
}

/// Two tables that differ only in where a name ends produce the same byte
/// stream if the name is not length-prefixed. These two are such a pair: with
/// `("ax", 11…78)` and `("b", 22…)` against `("a", 78 11…)` and `("xb", 22…)`
/// the unprefixed concatenations are both
/// `61 78 11×31 78 62 22×32`, so the fold would collide.
#[test]
fn the_length_prefix_separates_tables_that_would_otherwise_collide() {
    let h1 = "11".repeat(31) + "78";
    let h2 = "22".repeat(32);
    let k1 = "78".to_owned() + &"11".repeat(31);
    let a = [("ax", h1.as_str()), ("b", h2.as_str())];
    let b = [("a", k1.as_str()), ("xb", h2.as_str())];
    assert_ne!(
        fold_semantics_id(&a).expect("folds"),
        fold_semantics_id(&b).expect("folds"),
        "the fold does not separate these two tables, so the name length is \
         not part of the input"
    );
}

/// A table the fold cannot order or parse is rejected instead of folded: an
/// empty table identifies nothing, a repeated name has no unique order, and a
/// name or hash that is not what it claims to be would make the identifier
/// depend on how the malformed text happened to be handled.
#[test]
fn a_table_that_cannot_be_ordered_or_parsed_is_rejected() {
    let h = "ab".repeat(32);
    let h = h.as_str();
    assert!(fold_semantics_id(&[]).is_err(), "empty table");
    assert!(
        fold_semantics_id(&[("sin", h), ("sin", h)]).is_err(),
        "duplicate name"
    );
    assert!(fold_semantics_id(&[("", h)]).is_err(), "empty name");
    assert!(fold_semantics_id(&[("sín", h)]).is_err(), "non-ASCII name");
    assert!(fold_semantics_id(&[("sin", "ab")]).is_err(), "short hash");
    assert!(
        fold_semantics_id(&[("sin", &"zz".repeat(32))]).is_err(),
        "non-hex hash"
    );
    // a minimal well-formed table does fold, so the checks above are not
    // rejecting everything
    assert!(fold_semantics_id(&[("sin", h)]).is_ok());
}

/// The SIMD path hashes to the same bytes as the scalar one (parity is
/// checked lane by lane in `simd_parity.rs`; this pins it against the same
/// recorded constants so a platform-dependent SIMD op shows up here too).
#[cfg(feature = "simd")]
#[test]
fn simd_outputs_match_recorded_hashes() {
    use alice_det_math::simd;
    use wide::f32x8;
    fn via(f: fn(f32x8) -> f32x8) -> impl Fn(f32) -> f32 {
        move |x: f32| f(f32x8::splat(x)).to_array()[0]
    }
    let g = grid32();
    check("sin", &hash32(&g, via(simd::sin)), want("sin"));
    check("cos", &hash32(&g, via(simd::cos)), want("cos"));
    check("exp", &hash32(&g, via(simd::exp)), want("exp"));
    check("ln", &hash32(&g, via(simd::ln)), want("ln"));
    check("round", &hash32(&g, via(simd::round)), want("round"));
    check("sqrt", &hash32(&g, via(simd::sqrt)), want("sqrt"));
}

// ---------------------------------------------------------------------------
// PREVIOUS_SEMANTICS_IDS: earlier releases with the same bits
// ---------------------------------------------------------------------------

/// The pin table of release 0.4.0, copied verbatim from `tests/golden.rs` at
/// tag `v0.4.0` (`git show v0.4.0:tests/golden.rs`). Not edited: the proof
/// below depends on it being what that release pinned.
const GOLDEN_V0_4_0: &[(&str, &str)] = &[
    (
        "smoothstep",
        "d43e5a4972e13773caf1da15db30501850e505f361fd7526c908c35ff8e7cc7b",
    ),
    (
        "metric_norm_mix",
        "00f35aac72ff55e1069259196427d703926b47f7b4349fb4cc7f3f7b8effdc60",
    ),
    (
        "metric_lipschitz",
        "566d51df6ba7ba7358f51c25b8c19b8ad85879caca497bd6f6f22ad8d332076e",
    ),
    (
        "sin",
        "5e51709ffaa05b16afe14f2268c97633a0058eee46ff2f563e24fb8426967ef1",
    ),
    (
        "cos",
        "d80b955d724c77e2d64c58dab6e5dc32cbca3a8274db4b9065f1d8528011f896",
    ),
    (
        "sin_cos",
        "e8fbc874df490f32d4bbd895cfa30b0d42c3624a58f042863e08ec6becd145fb",
    ),
    (
        "exp",
        "fa74cbe1433a80c6a2c1f02332275bdd624da09f773c69877696d989948010c8",
    ),
    (
        "ln",
        "f740b2d9021d091ae3f5a18ecf346fbe5205b2d3b0209e96dca70b710da2e924",
    ),
    (
        "log2",
        "31911ba674cb7bce895b09de45c8d9ea1a84a5c26ba5936ef5a01faa05ac551e",
    ),
    (
        "log10",
        "2acdd36905c9d2ae3bb1d97db141d37f6c4823f630703fd8efede750a5c1caa1",
    ),
    (
        "powf",
        "aeeb77e2053b833490655db02c832d6305502dfb873b2bbb03b2557f64460e0e",
    ),
    (
        "powi",
        "769743b0123bb1c3926c8ec7445002b3de11fb6476f438dc67139313c2064117",
    ),
    (
        "cbrt",
        "b363cc6aa6be74c982b4351f0db1700a31d65ef68a00ab60503f495a9278ea65",
    ),
    (
        "hypot",
        "0cae2245259eaa5ec7267c3abe0db438088ebf5f53d33e28319bfe2579a3c4cd",
    ),
    (
        "atan",
        "3c5b4e38c7b09d816a5815073c8128d698b92173c2c443eef9a12fb06043dc78",
    ),
    (
        "atan2",
        "6fcbd5192a639e6e9b6b431485a4c3421b009b28f5d667b8a263124bf3ea28a4",
    ),
    (
        "asin",
        "ca20e5817f24d9970dca42171443c33af93ed8123c2eb9bb3441cd5427fb2fb7",
    ),
    (
        "acos",
        "b2e19e15112788c5523f4d7e53fcc143b58a1f24be543ffc76219c430fe56a56",
    ),
    (
        "tan",
        "ba94160fda3f50340c31f1abe034701d4bda9993cf0608635e802db72a9164fb",
    ),
    (
        "tanh",
        "8ae2d42020033aebfa823eb7ff3996d03ce8c0b4f5fd1f38c58d6197af1a3ff4",
    ),
    (
        "round",
        "7e92a4c7958c2972273ae47988f4818dfe01d16a30803e9b7cd3f06f14261359",
    ),
    (
        "sqrt",
        "64877c5a8a670097edf21dd0aeca4faf56ad9925952e7c48cb460ea5b7fb829d",
    ),
    (
        "atan64",
        "110c479ea68f1a9acd63fef9328f388d5ddf39ac1082274e9a35855dd60784e1",
    ),
    (
        "atan2_64",
        "f2263508c440333a745930d61fb5b65ab7efa441f7846f7048ca701af46caa26",
    ),
    (
        "asin64",
        "3f1953ff567f5d6c4e69b61a0b0b31100528d0f837788693a5617bf042dcc32d",
    ),
    (
        "acos64",
        "fe196c16623f5b61946c47cb78b286ca6ee1ad04233a5e438dd90b7c4e9fa43b",
    ),
    (
        "exp64",
        "a3782891336fb2cf0385790ea3589fd05fce413c4bbf5252048a6f8b72d54a39",
    ),
    (
        "ln64",
        "053aec9251fe146bb064b9a66a5f662ca59eb31b8507b9a50f27135f39dfc8c9",
    ),
    (
        "log2_64",
        "c3586ee8c0d3d001f36b1d57846cc1172d0b42b7629588054c050fc74126fd07",
    ),
    (
        "log10_64",
        "b3e8502088c819da30e0f12076d08b0fc52ef6f3496970b4600d1ff2f1c97934",
    ),
    (
        "powf64",
        "6bd59fa5f716c2adc9d5d9a4bdecf4f600889587ba0b5a927d4826fa1b593969",
    ),
    (
        "round64",
        "5e10f4fa0a2251cca43b0caf760c57906703bb9de8008efc6652195a10fddef2",
    ),
    (
        "sqrt64",
        "45b138399157594f7ce1e36a01940a185ec7f04de57fe041e47ecf99aa40bf00",
    ),
    (
        "sin64",
        "f67cfac4494c1bc738191d44db12d9a02df667dc86c5bf9471dded124824b367",
    ),
    (
        "cos64",
        "c8b013bf72744ff237e34a15b5cb05c3b33391970917db0074446fd70b349377",
    ),
    (
        "tan64",
        "bb8669e863c925b799c8d73eaf50965b2e7c60eab00a9bacbc24644025d7d3d6",
    ),
    (
        "sin64_dense",
        "ce7f4729ce2f51e9c0fccd9657ffee833d04cfdb4497f72b4ab70da89e0dd525",
    ),
    (
        "cos64_dense",
        "4de5c741eb8cb0fb9cfea9aeda36ffeebbd0de8b4f1ad3b697f0ed9eb6e60918",
    ),
    (
        "tan64_dense",
        "cf5945d72c4e963e438588ff84821a51bc2b2d909db13239dea400667f6ea80e",
    ),
    (
        "sin_cos64",
        "37e1a316a90d2f09b318b52950c4ddaa2103b6410a075b1b43ac70101debc1a8",
    ),
];

/// Pins renamed since 0.4.0 without changing what they hash: `(old, new)`.
const RENAMED_SINCE_V0_4_0: &[(&str, &str)] = &[("metric_norm_mix", "metric_norm")];

/// One pin table per entry of [`alice_det_math::PREVIOUS_SEMANTICS_IDS`], in
/// the same order, with the renames that map its names onto [`GOLDEN`].
/// A pin table: `(name, 64 hex digits)` per function.
type PinTable = &'static [(&'static str, &'static str)];

const PREVIOUS_TABLES: &[(PinTable, PinTable)] = &[(GOLDEN_V0_4_0, RENAMED_SINCE_V0_4_0)];

/// Every previous identifier is backed by its release's pin table, and this
/// release reproduces every pin in it.
///
/// The tests above recompute every hash in [`GOLDEN`] from this release's
/// code. So if each pin of a previous table is equal to the [`GOLDEN`] entry
/// for the same function, this release returns that release's bits on every
/// function the release covered; and if the table folds to the identifier,
/// the identifier is that release's. Both together are what lets a reader
/// accept the identifier as [`SemanticsCheck::VerifiedPrevious`].
#[test]
fn every_previous_identifier_is_backed_by_a_reproduced_pin_table() {
    use alice_det_math::{check_semantics, SemanticsCheck, PREVIOUS_SEMANTICS_IDS, SEMANTICS_ID};
    assert!(
        !PREVIOUS_TABLES.is_empty(),
        "no previous table: nothing was proved"
    );
    assert_eq!(
        PREVIOUS_TABLES.len(),
        PREVIOUS_SEMANTICS_IDS.len(),
        "each previous identifier needs exactly one pin table"
    );
    let mut compared = 0usize;
    for ((table, renames), id) in PREVIOUS_TABLES.iter().zip(PREVIOUS_SEMANTICS_IDS) {
        assert_eq!(
            fold_semantics_id(table).expect("the previous table folds"),
            *id,
            "the previous table does not fold to its identifier"
        );
        assert_ne!(
            *id, SEMANTICS_ID,
            "a previous identifier equals the current one"
        );
        for (name, hash) in *table {
            let current = renames
                .iter()
                .find(|(old, _)| old == name)
                .map_or(*name, |(_, new)| *new);
            let now = GOLDEN
                .iter()
                .find(|(n, _)| *n == current)
                .unwrap_or_else(|| panic!("`{name}` (now `{current}`) is no longer pinned"));
            assert_eq!(
                now.1, *hash,
                "`{name}` returns different bits than the release that stamped this identifier"
            );
            compared += 1;
        }
        assert_eq!(check_semantics(id), SemanticsCheck::VerifiedPrevious(*id));
    }
    assert_eq!(compared, 40, "pins compared");
}

/// The three-way result and the two predicates, including an identifier one
/// bit away from each known one.
#[test]
fn check_semantics_classifies_current_previous_and_unknown() {
    use alice_det_math::{check_semantics, SemanticsCheck, PREVIOUS_SEMANTICS_IDS, SEMANTICS_ID};
    let cur = check_semantics(&SEMANTICS_ID);
    assert_eq!(cur, SemanticsCheck::Verified);
    assert!(cur.is_current() && cur.is_reproducible());
    for prev in PREVIOUS_SEMANTICS_IDS {
        let c = check_semantics(prev);
        assert_eq!(c, SemanticsCheck::VerifiedPrevious(*prev));
        assert!(!c.is_current() && c.is_reproducible());
    }
    for known in std::iter::once(&SEMANTICS_ID).chain(PREVIOUS_SEMANTICS_IDS) {
        for byte in [0usize, 31] {
            let mut near = *known;
            near[byte] ^= 1;
            let c = check_semantics(&near);
            assert_eq!(c, SemanticsCheck::Mismatch, "one bit off is accepted");
            assert!(!c.is_current() && !c.is_reproducible());
        }
    }
}
