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
//! the hashes for re-pinning after an intentional algorithm change.

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
        "metric_norm_mix",
        &hash32x3(&g, |x, y, z| mixed.norm([x, y, z])),
        want("metric_norm_mix"),
    );
    check(
        "metric_lipschitz",
        &hash32x3(&g, |a, b, c| {
            metric::MetricWeights::new(a.abs(), b.abs(), c.abs())
                .map_or(f32::NAN, metric::MetricWeights::lipschitz)
        }),
        want("metric_lipschitz"),
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

/// The crate's public numeric functions, read out of `src/lib.rs`: the names
/// it re-exports at the crate root (`pub use <module>::{ … }`), keeping the
/// `snake_case` entries (functions) and dropping the `SCREAMING_SNAKE_CASE`
/// ones (the kernel constants the `consts` module re-exports). That list *is* the
/// crate's public numeric surface — every transcendental has its entry point
/// there — so counting it is how this test knows whether the golden table
/// has fallen behind the code.
fn public_numeric_functions() -> Vec<String> {
    const LIB: &str = include_str!("../src/lib.rs");
    // a `pub fn` written directly in lib.rs would not be in a `pub use` list
    assert!(
        !LIB.contains("pub fn "),
        "lib.rs declares a function directly; extend this parser to see it"
    );
    let mut out = Vec::new();
    let mut blocks = 0usize;
    let mut rest = LIB;
    while let Some(i) = rest.find("pub use ") {
        rest = &rest[i + "pub use ".len()..];
        let open = rest.find('{').expect("`pub use` with no brace list");
        assert!(
            !rest[..open].contains(';'),
            "`pub use` without a brace list: {:?}",
            &rest[..open]
        );
        let close = rest.find("};").expect("unterminated `pub use` list");
        blocks += 1;
        for raw in rest[open + 1..close].split(',') {
            let name = raw.trim();
            if !name.is_empty() && !name.chars().any(|c| c.is_ascii_uppercase()) {
                out.push(name.to_owned());
            }
        }
        rest = &rest[close..];
    }
    assert!(
        blocks >= 3,
        "only {blocks} `pub use` lists parsed in lib.rs"
    );
    out
}

/// Pins in [`GOLDEN`] that are not themselves a re-exported function: the
/// `metric` module's entry points, and the second, denser input grid used for
/// the functions whose first reduction cases the coarse grid barely samples.
const EXTRA_PINS: &[&str] = &[
    "smoothstep",
    "metric_norm_mix",
    "metric_lipschitz",
    "sin64_dense",
    "cos64_dense",
    "tan64_dense",
];

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
    let functions = public_numeric_functions();
    // 34 = 13 from `double` + 4 from `ops` + 17 from `single`; adding one
    // means adding its golden pin and raising this number in the same commit
    assert_eq!(
        functions.len(),
        34,
        "public numeric functions: {functions:?}"
    );
    for sentinel in ["sin", "ln64", "sqrt", "tan64", "log2", "log10_64"] {
        assert!(
            functions.iter().any(|f| f == sentinel),
            "the parse lost `{sentinel}`: {functions:?}"
        );
    }
    let pinned: Vec<&str> = GOLDEN.iter().map(|(n, _)| *n).collect();
    assert_eq!(pinned.len(), 40, "golden pins: {pinned:?}");
    for f in &functions {
        assert!(
            pinned.contains(&f.as_str()),
            "`{f}` is public but has no entry in GOLDEN, so a change to it \
             would not move SEMANTICS_ID"
        );
    }
    for p in &pinned {
        assert!(
            functions.iter().any(|f| f == p) || EXTRA_PINS.contains(p),
            "golden pin `{p}` names neither a public function nor a known \
             extra pin (a typo here silently pins nothing)"
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
