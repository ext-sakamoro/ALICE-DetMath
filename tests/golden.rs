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

mod common;
#[path = "support/digest_bits.rs"]
mod digest_bits;

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

/// Dense over the first reduction cases of `sin64` / `cos64` / `tan64` (|x| ≤ 9π/4 and a little beyond), where
/// [`grid64`] has only a few hundred points.
fn dense64() -> Vec<f64> {
    (0..=400_000u32)
        .map(|i| -10.0 + 20.0 * (f64::from(i) / 400_000.0))
        .collect()
}

/// The second operand of a two-argument pin: index `i` paired with this one.
fn pair_index(i: usize, n: usize) -> usize {
    (i * 7 + 3) % n
}

/// The third operand of a three-argument pin.
fn triple_index(i: usize, n: usize) -> usize {
    (i * 13 + 11) % n
}

fn hash32(xs: &[f32], f: impl Fn(f32) -> f32) -> String {
    hash32_bits(xs, |x| digest_bits::bits32(f(x)))
}

/// [`hash32`] over bits the caller folded from outputs it read with [`digest_bits::bits32`] (a fold
/// of two canonical NaNs is not a NaN the fold returned, so it is hashed as bits).
fn hash32_bits(xs: &[f32], f: impl Fn(f32) -> u32) -> String {
    let mut h = Sha256::new();
    for &x in xs {
        h.update(f(x).to_le_bytes());
    }
    format!("{:x}", h.finalize())
}

fn hash32x2(xs: &[f32], f: impl Fn(f32, f32) -> f32) -> String {
    let mut h = Sha256::new();
    for (i, &x) in xs.iter().enumerate() {
        let y = xs[pair_index(i, xs.len())];
        h.update(digest_bits::out32(f(x, y)));
    }
    format!("{:x}", h.finalize())
}

fn hash32x3(xs: &[f32], f: impl Fn(f32, f32, f32) -> f32) -> String {
    let mut h = Sha256::new();
    for (i, &x) in xs.iter().enumerate() {
        let y = xs[pair_index(i, xs.len())];
        let z = xs[triple_index(i, xs.len())];
        h.update(digest_bits::out32(f(x, y, z)));
    }
    format!("{:x}", h.finalize())
}

/// The `sin_cos` pin's value for one input: both outputs read with [`digest_bits::bits32`]
/// (each refused on its own if it is a non-canonical NaN), then folded into one word.
fn sin_cos_fold32(s: f32, c: f32) -> u32 {
    digest_bits::bits32(s) ^ digest_bits::bits32(c).rotate_left(16)
}

/// [`sin_cos_fold32`] for `sin_cos64`.
fn sin_cos_fold64(s: f64, c: f64) -> u64 {
    digest_bits::bits64(s) ^ digest_bits::bits64(c).rotate_left(32)
}

fn hash64(xs: &[f64], f: impl Fn(f64) -> f64) -> String {
    hash64_bits(xs, |x| digest_bits::bits64(f(x)))
}

/// [`hash64`] over bits folded from outputs read with [`digest_bits::bits64`].
fn hash64_bits(xs: &[f64], f: impl Fn(f64) -> u64) -> String {
    let mut h = Sha256::new();
    for &x in xs {
        h.update(f(x).to_le_bytes());
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

/// The inputs every pin of [`GOLDEN`] is computed over: (name, SHA-256 of the
/// inputs' bits, little-endian, in order, count). The pins fold into
/// [`alice_det_math::SEMANTICS_ID`], so an edit of a grid would move pins and
/// the identifier exactly like a change of the arithmetic; frozen here, such
/// an edit is a red of its own.
///
/// oracle: the digests are computed by an independent rebuild of each input
/// from its definition (Python: the integer and bit-pattern parts as written,
/// each `f32` operation computed in `f64` and rounded to `f32`, exact since
/// 53 ≥ 2·24 + 2), not by this code.
const REFERENCE_INPUTS: &[(&str, &str, usize)] = &[
    (
        "grid32",
        "e39deb3483a86dca12c757e7c70d67768269706b06c0fcf43fb4aab94b7c3868",
        300_003,
    ),
    (
        "grid64",
        "fc31296ddedf1ff54f8526c23ccbd2fc9f9fdfd49e3fcb6f61733e9bbac2fa9d",
        200_002,
    ),
    (
        "dense64",
        "d396908abad59d17d75964d76cd556945b9c252ab08a2cf5fad260ea10224e3e",
        400_001,
    ),
    (
        "grid32_pair_index",
        "0abaae78d73021190767ec04385ccbe75a9cccce96fc38d9ffbba994fb0182ea",
        300_003,
    ),
    (
        "grid32_triple_index",
        "de76c270f03bb43887140ab2e702937a5b464f5c3ed10c4ea818c86e45f4d387",
        300_003,
    ),
    (
        "SPECIALS32",
        "8d65dae0bc6024fd6297bc3667afe091f44c693ebdeea3ce2773a8a6a8dbf499",
        13,
    ),
    (
        "SPECIALS64",
        "facc5b4be7425dbc82f2b841a6a06f79bcfc0541a1acda34883e0e5a8407da8b",
        13,
    ),
];

fn input_digest(chunks: impl Iterator<Item = Vec<u8>>) -> (String, usize) {
    let mut h = Sha256::new();
    let mut n = 0;
    for c in chunks {
        h.update(&c);
        n += 1;
    }
    (format!("{:x}", h.finalize()), n)
}

#[test]
fn reference_inputs_are_frozen() {
    let g32 = grid32();
    let n32 = g32.len();
    let got = [
        (
            "grid32",
            input_digest(
                g32.iter()
                    .map(|&x| digest_bits::pattern32(x).to_le_bytes().to_vec()),
            ),
        ),
        (
            "grid64",
            input_digest(
                grid64()
                    .iter()
                    .map(|&x| digest_bits::pattern64(x).to_le_bytes().to_vec()),
            ),
        ),
        (
            "dense64",
            input_digest(
                dense64()
                    .iter()
                    .map(|&x| digest_bits::pattern64(x).to_le_bytes().to_vec()),
            ),
        ),
        (
            "grid32_pair_index",
            input_digest((0..n32).map(|i| {
                u32::try_from(pair_index(i, n32))
                    .unwrap()
                    .to_le_bytes()
                    .to_vec()
            })),
        ),
        (
            "grid32_triple_index",
            input_digest((0..n32).map(|i| {
                u32::try_from(triple_index(i, n32))
                    .unwrap()
                    .to_le_bytes()
                    .to_vec()
            })),
        ),
        (
            "SPECIALS32",
            input_digest(
                common::specials::SPECIALS32
                    .iter()
                    .map(|&x| digest_bits::pattern32(x).to_le_bytes().to_vec()),
            ),
        ),
        (
            "SPECIALS64",
            input_digest(
                common::specials::SPECIALS64
                    .iter()
                    .map(|&x| digest_bits::pattern64(x).to_le_bytes().to_vec()),
            ),
        ),
    ];
    assert_eq!(
        got.len(),
        REFERENCE_INPUTS.len(),
        "every input of the pins is frozen"
    );
    let mut changed = Vec::new();
    for ((name, (h, n)), (want_name, want_h, want_n)) in got.iter().zip(REFERENCE_INPUTS) {
        assert_eq!(name, want_name, "REFERENCE_INPUTS order");
        assert!(*n > 0, "{name}: empty");
        if (h.as_str(), *n) != (*want_h, *want_n) {
            changed.push(format!("    (\"{name}\", \"{h}\", {n}),"));
        }
    }
    assert!(
        changed.is_empty(),
        "an input of the pins changed (it moves the pins and SEMANTICS_ID): if intended, re-pin the table and the \
         identifier and record the new input here:\n{}",
        changed.join("\n")
    );
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
        "184e9e4776478a4608497033c7e9f39e14b6b629b691a9bb9ac2a4313f2de33e",
    ),
    (
        "metric_norm",
        "00f35aac72ff55e1069259196427d703926b47f7b4349fb4cc7f3f7b8effdc60",
    ),
    (
        "metric_lipschitz",
        "ff8c5743709b8a02beb42eb169064e2bd7f290096c9c99ac40551583be3b1f58",
    ),
    (
        "norm_l1",
        "522716e421c074fd03f831a1fb5b51280da91c573c90530b02c522982c677839",
    ),
    (
        "norm_l2",
        "69e09bc27dee17bbdc9eeeb4ab0f984f6539e4c487463c725b34b9f48426dbf9",
    ),
    (
        "norm_linf",
        "d4446c6e32de4948c74bbb5b76daa0e1de4daebe00e44503e720f149b981da18",
    ),
    (
        "lerp",
        "fe010775a3074825cbf8e3b28cf598d7e7d3a598e7c3da185f7f9525c7b6e6a6",
    ),
    (
        "clamp",
        "0839f2e8df960550266d37fc3280ce60ba686bd5f97673bc8a44663ad7a96243",
    ),
    (
        "metric_new",
        "b472f053e4fe43fedcff6e06189ce4ca6cff559cafe0326cdccf449f6596bff5",
    ),
    (
        "metric_weights",
        "f73d181f731d1d95280a5855ffcc3bd7413ccc974395d6be5add6be82e89337a",
    ),
    (
        "metric_minimum",
        "92f1a9f945518c7aeeb964edfd07e8920ca3f45a4f2158c6a6fa07a57e6ecf08",
    ),
    (
        "metric_axis_extent",
        "631977eb6838a15b591abe0c3734da5dc1526735934c3994aeebf6c9b540338a",
    ),
    (
        "metric_euclidean_radius",
        "99caadab73f41d5a4419a900e5995dfeb5a14d657ac2b747f9aab1b7257b8a41",
    ),
    (
        "metric_normalised",
        "0584c61ae6d933378863f0b612e59bbbb36db0c0d7fc958a316568f93bc2cb56",
    ),
    (
        "sin",
        "a9feff05303cfc4107ac3c855048efaa636615e23a477f6ea04075ff684f2517",
    ),
    (
        "cos",
        "376f24f9f5d9ee37dfa51e464bb6ae341cb69978b1a0768bba95f5b16ec96f22",
    ),
    (
        "sin_cos",
        "d0bf4c21d5f13283893fb954b45c2f9127d33d7862e844d7a050b03bfecd778c",
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
        "09275f109b8a4c4c7198e834fdd6f0f2083ab6db85909da5e847e993b5db1bfa",
    ),
    (
        "powi",
        "769743b0123bb1c3926c8ec7445002b3de11fb6476f438dc67139313c2064117",
    ),
    (
        "cbrt",
        "bbdce4d0d01cac1ca2389e7bfdee76fdba318edf3801cdc2a70888611105f3f3",
    ),
    (
        "hypot",
        "ec4c7823be970a04dd167bea5e3ee390f95fbb87fa223889c5519ea8241acdec",
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
        "erf",
        "d7b2c87da116e262612481accc1083b043b398a580aea23bfd020c8b3273d824",
    ),
    (
        "round",
        "3f4ae38799a1c0e4e19ed084c94b07dd2f65c052c7b4a3572a818ed3656aaf41",
    ),
    (
        "sqrt",
        "64877c5a8a670097edf21dd0aeca4faf56ad9925952e7c48cb460ea5b7fb829d",
    ),
    (
        "max",
        "93fd4a229d231ca16606ded184e71469c8dbffb54e62232d6086edf858240c78",
    ),
    (
        "min",
        "49f0b49b870d11dc1be9d2ffbf1f1f887400e93217943daaa885e145d6c9c571",
    ),
    (
        "max64",
        "6ae804e54c77d6cc2d690465511daa4bfa73f3345751c521bba499d49925d89c",
    ),
    (
        "min64",
        "24b06cd31dc3324a15bad0de2605ba8177ef471e30aae60c95104e0a9f009aa4",
    ),
    (
        "clamp64",
        "2f47d28209a3ba74192164d26004239137b9dde7ba13388f5f67d05950e5af65",
    ),
    (
        "argmax",
        "3d72c9a3a4e83ab2f894e9383b249befe3ea359e8b79b63d71bbcc4488380c08",
    ),
    (
        "argmin",
        "b4cd46d934eab486027ba62654ef06b5015344070a2b241c5062d3705ce23edf",
    ),
    (
        "ops_greatest",
        "195a5bffc7b88b2c1afbdfd618569fe47dc6764a39736a130fba34984fb06585",
    ),
    (
        "ops_least",
        "fe579644fa6ccdc2bfdc938509075b9ae3b7d5753be3553239fc71f56852d064",
    ),
    (
        "ops_offer",
        "5fb561d162a8ae64ae64c9dcda9b5e6a6ea6084a5908d15fa2322914d74c6d3d",
    ),
    (
        "ops_into_inner",
        "cf35da849d1984d2d8beaa944a8ddc21e8c25ea0114bed054e8ad180a2ef7df0",
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
        "tanh64",
        "cb5888ca0e34859ec7e05219632408fa4d8a81a65b9b48031c9f34b735295b09",
    ),
    (
        "hypot64",
        "c4eeb44d9e444a5c798a8e7b17d492d09c26b1df812ebaf511c0294a574c3b85",
    ),
    (
        "powi64",
        "63af9f58e31ac098d1183a1266dc550b2b0ef084ac0b7df9cf0ad814b8297961",
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
        "49fec7cfa239dadfe4fcd1dcb49fc3f09ca5eaa25c5497b254e17e01feaeea5a",
    ),
    (
        "round64",
        "b0cfa27c69e463997beae85c03c962fa14995026c465dba50005833862049284",
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
        "erf64",
        "557e03d68112056ca4226847a5542141e35949ef2079282883b80b2697997146",
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
    (
        "specials",
        "5647435cb9d32c85567bb6484c8fc9e6a76a0718ab7d74cbac031db72faf898f",
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
        &hash32_bits(&g, |x| {
            let (s, c) = sin_cos(x);
            sin_cos_fold32(s, c)
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
        &hash32(&g, |x| {
            powi(x, (digest_bits::pattern32(x) % 31) as i32 - 15)
        }),
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
    check("erf", &hash32(&g, erf), want("erf"));
    check("round", &hash32(&g, round), want("round"));
    check("sqrt", &hash32(&g, sqrt), want("sqrt"));
    check(
        "max",
        &hash32x2(&g, |x, y| max(x, y * 0.37 - 1.0)),
        want("max"),
    );
    check(
        "min",
        &hash32x2(&g, |x, y| min(x, y * 0.37 - 1.0)),
        want("min"),
    );
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
    // the selections: the index chosen among three operands (-1 for none), and the value and
    // index the streaming form keeps in each direction
    let index = |i: Option<usize>| i.map_or(-1.0, |i| i as f32);
    check(
        "argmax",
        &hash32x3(&g, |a, b, c| index(argmax(&[a, b, c]))),
        want("argmax"),
    );
    check(
        "argmin",
        &hash32x3(&g, |a, b, c| index(argmin(&[a, b, c]))),
        want("argmin"),
    );
    let select = |mut s: Select<u8>, a: f32, b: f32, c: f32| {
        s.offer(a, 0);
        s.offer(b, 1);
        s.offer(c, 2);
        s.into_inner()
    };
    check(
        "ops_greatest",
        &hash32x3(&g, |a, b, c| {
            select(Select::greatest(), a, b, c).map_or(-1.0, |(v, _)| v)
        }),
        want("ops_greatest"),
    );
    check(
        "ops_least",
        &hash32x3(&g, |a, b, c| {
            select(Select::least(), a, b, c).map_or(-1.0, |(v, _)| v)
        }),
        want("ops_least"),
    );
    check(
        "ops_offer",
        &hash32x3(&g, |a, b, c| {
            select(Select::greatest(), c, b, a).map_or(-1.0, |(_, i)| f32::from(i))
        }),
        want("ops_offer"),
    );
    check(
        "ops_into_inner",
        &hash32x3(&g, |a, b, c| {
            select(Select::least(), c, b, a).map_or(-1.0, |(_, i)| f32::from(i))
        }),
        want("ops_into_inner"),
    );
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
    check("tanh64", &hash64(&g, tanh64), want("tanh64"));
    check(
        "hypot64",
        &hash64(&g, |x| hypot64(x, x * 0.37 - 1.0)),
        want("hypot64"),
    );
    check(
        "powi64",
        &hash64(&g, |x| powi64(x, (x * 13.0) as i32 % 64 - 32)),
        want("powi64"),
    );
    check("ln64", &hash64(&g, ln64), want("ln64"));
    check("log2_64", &hash64(&g, log2_64), want("log2_64"));
    check("log10_64", &hash64(&g, log10_64), want("log10_64"));
    check("tan64", &hash64(&g, tan64), want("tan64"));
    check("erf64", &hash64(&g, erf64), want("erf64"));
    check(
        "powf64",
        &hash64(&g, |x| powf64(x.abs(), x * 1.0e-2)),
        want("powf64"),
    );
    check("round64", &hash64(&g, round64), want("round64"));
    check("sqrt64", &hash64(&g, sqrt64), want("sqrt64"));
    check(
        "max64",
        &hash64(&g, |x| max64(x, x * 0.37 - 1.0)),
        want("max64"),
    );
    check(
        "min64",
        &hash64(&g, |x| min64(x, x * 0.37 - 1.0)),
        want("min64"),
    );
    check(
        "clamp64",
        &hash64(&g, |x| clamp64(x, x * 0.37 - 1.0, x * 0.61 + 2.0)),
        want("clamp64"),
    );
    check("sin64", &hash64(&g, sin64), want("sin64"));
    check("cos64", &hash64(&g, cos64), want("cos64"));
    check(
        "sin_cos64",
        &hash64_bits(&g, |x| {
            let (s, c) = sin_cos64(x);
            sin_cos_fold64(s, c)
        }),
        want("sin_cos64"),
    );
    // dense over the first reduction cases (|x| ≤ 9π/4 and a little beyond),
    // where the coarse grid has only a few hundred points
    let dense = dense64();
    check("sin64_dense", &hash64(&dense, sin64), want("sin64_dense"));
    check("cos64_dense", &hash64(&dense, cos64), want("cos64_dense"));
    // the tangent's kernel branches on |reduced| ≥ 0.6744 and on the parity
    // of the quadrant, so the dense grid covers all four combinations
    check("tan64_dense", &hash64(&dense, tan64), want("tan64_dense"));
}

/// Hashes what [`common::specials::visit`] reports: each pin name
/// (length-prefixed) followed by its outputs' bits.
struct HashSink {
    h: Sha256,
    names: Vec<&'static str>,
}

impl common::specials::Sink for HashSink {
    fn name(&mut self, name: &'static str) {
        self.names.push(name);
        self.h
            .update(u32::try_from(name.len()).unwrap().to_le_bytes());
        self.h.update(name.as_bytes());
    }
    fn v32(&mut self, v: f32) {
        self.h.update(digest_bits::out32(v));
    }
    fn v64(&mut self, v: f64) {
        self.h.update(digest_bits::out64(v));
    }
}

/// The `specials` pin: every public scalar function at the IEEE special values
/// (±0, ±smallest subnormal, ±smallest normal, ±1, ±largest finite, ±inf,
/// NaN; all pairs and triples for two- and three-argument functions). The
/// grids above walk through `+0` but never `−0` and reach the other specials
/// only by chance, so the sign of zero, subnormal and non-finite behaviour
/// — and the NaN bits, which differ between targets when a function forgets
/// to canonicalise — are in the identifier only through this pin.
fn specials_hash(sin: fn(f32) -> f32) -> (String, Vec<&'static str>) {
    let mut s = HashSink {
        h: Sha256::new(),
        names: Vec::new(),
    };
    common::specials::visit(&mut s, sin);
    (format!("{:x}", s.h.finalize()), s.names)
}

/// The specials pin is a pin like the others. It is new in this release: no
/// earlier pin table has it, so it moves no old pin.
#[test]
fn every_function_at_the_ieee_specials_matches_the_recorded_hash() {
    let (hash, names) = specials_hash(alice_det_math::sin);
    check("specials", &hash, want("specials"));
    assert_eq!(names.len(), 63, "functions hashed at the specials");
}

/// The specials cover exactly the functions [`GOLDEN`] pins by name, so a
/// function added to the crate (which must get a pin) must be added there too.
#[test]
fn the_specials_cover_every_public_numeric_function() {
    let (_, names) = specials_hash(alice_det_math::sin);
    let mut covered: Vec<&str> = names.clone();
    covered.sort_unstable();
    covered.dedup();
    assert_eq!(
        covered.len(),
        names.len(),
        "a function is visited twice: {names:?}"
    );
    let mut wanted: Vec<String> = public_numeric_surface()
        .into_iter()
        .filter(|s| !s.module.starts_with("simd"))
        .map(|s| s.pin())
        .collect();
    wanted.sort_unstable();
    assert_eq!(wanted.len(), 63);
    assert_eq!(covered, wanted, "the specials miss or add a function");
}

/// A `sin` that loses the sign of zero (and only that) moves the pin.
#[test]
fn a_lost_sign_of_zero_moves_the_specials_pin() {
    fn sin_without_signed_zero(x: f32) -> f32 {
        let y = alice_det_math::sin(x);
        if y == 0.0 {
            0.0
        } else {
            y
        }
    }
    let (good, _) = specials_hash(alice_det_math::sin);
    let (bad, _) = specials_hash(sin_without_signed_zero);
    assert_ne!(good, bad);
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
    ("double::pow", include_str!("../src/double/pow.rs")),
    ("double::trig", include_str!("../src/double/trig.rs")),
    ("erf", include_str!("../src/erf.rs")),
    ("law_bits", include_str!("../src/law_bits.rs")),
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
        .rfind(|l| l.starts_with("impl ") || l.starts_with("impl<"))
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
        if IDENTIFIER_MODULES.contains(module) || BIT_MODULES.contains(module) {
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

/// Modules that read a float's bits and compute nothing from them: `law_bits` returns the
/// bits as they are or refuses a NaN, so it adds no arithmetic for [`SEMANTICS_ID`] to
/// identify (its behaviour is pinned by `tests/law_bits.rs`, every `f32` NaN pattern).
///
/// [`SEMANTICS_ID`]: alice_det_math::SEMANTICS_ID
const BIT_MODULES: &[&str] = &["law_bits"];

/// A bit module defines exactly the functions expected, and does no float arithmetic: its
/// only float operation is `to_bits` (a numeric function added there would escape the pins).
#[test]
fn bit_modules_compute_nothing_from_a_float() {
    let mut seen = Vec::new();
    for (module, src) in SOURCES {
        if !BIT_MODULES.contains(module) {
            continue;
        }
        for (ty, name) in parsed_fns(module, src) {
            seen.push(format!(
                "{module}::{}{name}",
                ty.map(|t| t + "::").unwrap_or_default()
            ));
        }
        let code: String = src
            .lines()
            .map(|l| l.split("//").next().unwrap_or_default())
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(
            code.matches(concat!(".to_", "bits()")).count(),
            2,
            "`{module}` reads bits twice"
        );
        for op in ["from_bits", "is_nan", " as f", ".abs(", "f32::", "f64::"] {
            assert!(
                !code.contains(op),
                "`{module}` uses `{op}`: a float operation escapes the pins"
            );
        }
    }
    seen.sort();
    assert_eq!(
        seen,
        ["law_bits::f32_bits", "law_bits::f64_bits"],
        "bit module surface changed"
    );
}

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
            "semantics::check_semantics_against",
            "semantics::verify_semantics_against",
        ],
        "identifier module surface changed"
    );
}

/// Pins in [`GOLDEN`] that do not name a public function: the second, denser
/// input grid used for the functions whose first reduction cases the coarse
/// grid barely samples, and `specials` (every function at the IEEE special
/// values, [`specials_hash`]).
const EXTRA_PINS: &[&str] = &["sin64_dense", "cos64_dense", "tan64_dense", "specials"];

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
    // 63 = 15 `double` (11 of its own + 4 from its private `trig`)
    //    + 2 `erf` (erf / erf64)
    //    + 15 `ops` (round / round64 / sqrt / sqrt64 / max / min / max64 / min64 / clamp64 /
    //      argmax / argmin, and `Select`'s greatest / least / offer / into_inner)
    //    + 17 `single`
    //    + 14 `metric` (6 free functions + 8 methods on `MetricWeights`).
    // Adding one means adding its golden pin and raising this number in the
    // same commit.
    assert_eq!(
        scalar.len(),
        63,
        "public numeric surface: {:?}",
        ids(&scalar)
    );
    // `simd` is deliberately not folded into SEMANTICS_ID: it is
    // feature-gated, and an identifier that changed with the feature set
    // would not identify the crate's arithmetic. Its functions add no
    // semantics of their own — each must return the bits its scalar
    // counterpart returns, which `every_simd_function_has_a_parity_test`
    // keeps true.
    assert_eq!(simd.len(), 23, "SIMD surface: {:?}", ids(&simd));
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
    assert_eq!(pinned.len(), 67, "golden pins: {pinned:?}");
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
    assert_eq!(simd.len(), 23, "SIMD surface: {:?}", ids(&simd));
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
/// tag `v0.4.0` (`git show v0.4.0:tests/golden.rs`). Not edited: the proofs
/// below depend on it being what that release pinned. It is no longer an
/// entry of [`alice_det_math::PREVIOUS_SEMANTICS_IDS`] (`sin` / `cos` /
/// `sin_cos` changed); it stays to prove exactly that.
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

/// The pin table of release 0.5.0, copied verbatim from `tests/golden.rs` at
/// tag `v0.5.0` (`git show v0.5.0:tests/golden.rs`). Not edited: the proofs
/// below depend on it being what that release pinned. Every function in it
/// still returns these exact bits (this release only adds `max` / `min` /
/// `max64` / `min64`, which 0.5.0 did not have), so its identifier is a
/// [`alice_det_math::PREVIOUS_SEMANTICS_IDS`] entry rather than rejected.
const GOLDEN_V0_5_0: &[(&str, &str)] = &[
    (
        "smoothstep",
        "184e9e4776478a4608497033c7e9f39e14b6b629b691a9bb9ac2a4313f2de33e",
    ),
    (
        "metric_norm",
        "00f35aac72ff55e1069259196427d703926b47f7b4349fb4cc7f3f7b8effdc60",
    ),
    (
        "metric_lipschitz",
        "ff8c5743709b8a02beb42eb169064e2bd7f290096c9c99ac40551583be3b1f58",
    ),
    (
        "norm_l1",
        "522716e421c074fd03f831a1fb5b51280da91c573c90530b02c522982c677839",
    ),
    (
        "norm_l2",
        "69e09bc27dee17bbdc9eeeb4ab0f984f6539e4c487463c725b34b9f48426dbf9",
    ),
    (
        "norm_linf",
        "d4446c6e32de4948c74bbb5b76daa0e1de4daebe00e44503e720f149b981da18",
    ),
    (
        "lerp",
        "fe010775a3074825cbf8e3b28cf598d7e7d3a598e7c3da185f7f9525c7b6e6a6",
    ),
    (
        "clamp",
        "626399c500e7ae66f5afed4ac55321304b7c488dd12889bb149188a604e65942",
    ),
    (
        "metric_new",
        "b472f053e4fe43fedcff6e06189ce4ca6cff559cafe0326cdccf449f6596bff5",
    ),
    (
        "metric_weights",
        "f73d181f731d1d95280a5855ffcc3bd7413ccc974395d6be5add6be82e89337a",
    ),
    (
        "metric_minimum",
        "92f1a9f945518c7aeeb964edfd07e8920ca3f45a4f2158c6a6fa07a57e6ecf08",
    ),
    (
        "metric_axis_extent",
        "631977eb6838a15b591abe0c3734da5dc1526735934c3994aeebf6c9b540338a",
    ),
    (
        "metric_euclidean_radius",
        "99caadab73f41d5a4419a900e5995dfeb5a14d657ac2b747f9aab1b7257b8a41",
    ),
    (
        "metric_normalised",
        "0584c61ae6d933378863f0b612e59bbbb36db0c0d7fc958a316568f93bc2cb56",
    ),
    (
        "sin",
        "84cf4edf30a5b6f644c1dc74dde2640e4467770ed5c35a296353c9e5b1174bf7",
    ),
    (
        "cos",
        "8310296979062107b644a1d340ca181facf1be5ea5122af76a74ec26eb33b38c",
    ),
    (
        "sin_cos",
        "19566da559090072ddbde9238a32500d4f254162991648082813f0fe504f739f",
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
        "bbdce4d0d01cac1ca2389e7bfdee76fdba318edf3801cdc2a70888611105f3f3",
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
        "3f4ae38799a1c0e4e19ed084c94b07dd2f65c052c7b4a3572a818ed3656aaf41",
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
        "b0cfa27c69e463997beae85c03c962fa14995026c465dba50005833862049284",
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
    (
        "specials",
        "2a9c0afc67d6cf6fef6d8a5ff3159abebe96dc48b8b7b5d9ddf19c4ca5653bec",
    ),
];

/// No pin was renamed between 0.5.0 and this release.
const RENAMED_SINCE_V0_5_0: &[(&str, &str)] = &[];

/// The pin table of 0.5.1 (also 0.5.2, which moved no pin): 0.5.0's functions plus `max` /
/// `min` / `max64` / `min64`, folded to `85e08846…b315`. This release changes the bits of
/// `clamp`, so that identifier is not reproduced and is not listed as previous;
/// [`the_0_5_identifiers_are_rejected_because_exactly_clamp_changed`] proves which pins moved.
const GOLDEN_V0_5_1: &[(&str, &str)] = &[
    (
        "smoothstep",
        "184e9e4776478a4608497033c7e9f39e14b6b629b691a9bb9ac2a4313f2de33e",
    ),
    (
        "metric_norm",
        "00f35aac72ff55e1069259196427d703926b47f7b4349fb4cc7f3f7b8effdc60",
    ),
    (
        "metric_lipschitz",
        "ff8c5743709b8a02beb42eb169064e2bd7f290096c9c99ac40551583be3b1f58",
    ),
    (
        "norm_l1",
        "522716e421c074fd03f831a1fb5b51280da91c573c90530b02c522982c677839",
    ),
    (
        "norm_l2",
        "69e09bc27dee17bbdc9eeeb4ab0f984f6539e4c487463c725b34b9f48426dbf9",
    ),
    (
        "norm_linf",
        "d4446c6e32de4948c74bbb5b76daa0e1de4daebe00e44503e720f149b981da18",
    ),
    (
        "lerp",
        "fe010775a3074825cbf8e3b28cf598d7e7d3a598e7c3da185f7f9525c7b6e6a6",
    ),
    (
        "clamp",
        "626399c500e7ae66f5afed4ac55321304b7c488dd12889bb149188a604e65942",
    ),
    (
        "metric_new",
        "b472f053e4fe43fedcff6e06189ce4ca6cff559cafe0326cdccf449f6596bff5",
    ),
    (
        "metric_weights",
        "f73d181f731d1d95280a5855ffcc3bd7413ccc974395d6be5add6be82e89337a",
    ),
    (
        "metric_minimum",
        "92f1a9f945518c7aeeb964edfd07e8920ca3f45a4f2158c6a6fa07a57e6ecf08",
    ),
    (
        "metric_axis_extent",
        "631977eb6838a15b591abe0c3734da5dc1526735934c3994aeebf6c9b540338a",
    ),
    (
        "metric_euclidean_radius",
        "99caadab73f41d5a4419a900e5995dfeb5a14d657ac2b747f9aab1b7257b8a41",
    ),
    (
        "metric_normalised",
        "0584c61ae6d933378863f0b612e59bbbb36db0c0d7fc958a316568f93bc2cb56",
    ),
    (
        "sin",
        "84cf4edf30a5b6f644c1dc74dde2640e4467770ed5c35a296353c9e5b1174bf7",
    ),
    (
        "cos",
        "8310296979062107b644a1d340ca181facf1be5ea5122af76a74ec26eb33b38c",
    ),
    (
        "sin_cos",
        "19566da559090072ddbde9238a32500d4f254162991648082813f0fe504f739f",
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
        "bbdce4d0d01cac1ca2389e7bfdee76fdba318edf3801cdc2a70888611105f3f3",
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
        "3f4ae38799a1c0e4e19ed084c94b07dd2f65c052c7b4a3572a818ed3656aaf41",
    ),
    (
        "sqrt",
        "64877c5a8a670097edf21dd0aeca4faf56ad9925952e7c48cb460ea5b7fb829d",
    ),
    (
        "max",
        "93fd4a229d231ca16606ded184e71469c8dbffb54e62232d6086edf858240c78",
    ),
    (
        "min",
        "49f0b49b870d11dc1be9d2ffbf1f1f887400e93217943daaa885e145d6c9c571",
    ),
    (
        "max64",
        "6ae804e54c77d6cc2d690465511daa4bfa73f3345751c521bba499d49925d89c",
    ),
    (
        "min64",
        "24b06cd31dc3324a15bad0de2605ba8177ef471e30aae60c95104e0a9f009aa4",
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
        "b0cfa27c69e463997beae85c03c962fa14995026c465dba50005833862049284",
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
    (
        "specials",
        "697892405de97e9649f9a3455d3a02aa20b2e4f68404f5319b821ce3c8fd1e83",
    ),
];

/// Pins renamed between 0.5.1 and this release: none.
const RENAMED_SINCE_V0_5_1: &[(&str, &str)] = &[];

/// The pin table of the unpublished identifier `a2e46794…e3cf` (main up to 39507d1, the
/// version still 0.5.2 while publication is frozen; consumers took it by git rev). This
/// release changes the bits of `sin` / `cos` / `sin_cos` (`|x| ≥ 2^13`), `powf` (negative
/// bases), `powf64` (fdlibm `e_pow.c`) and `hypot` (two subnormals), so that identifier is
/// not reproduced and is not listed as previous;
/// [`the_a2e46794_identifier_is_rejected_because_exactly_these_functions_changed`] proves
/// which pins moved.
const GOLDEN_A2E46794: &[(&str, &str)] = &[
    (
        "smoothstep",
        "184e9e4776478a4608497033c7e9f39e14b6b629b691a9bb9ac2a4313f2de33e",
    ),
    (
        "metric_norm",
        "00f35aac72ff55e1069259196427d703926b47f7b4349fb4cc7f3f7b8effdc60",
    ),
    (
        "metric_lipschitz",
        "ff8c5743709b8a02beb42eb169064e2bd7f290096c9c99ac40551583be3b1f58",
    ),
    (
        "norm_l1",
        "522716e421c074fd03f831a1fb5b51280da91c573c90530b02c522982c677839",
    ),
    (
        "norm_l2",
        "69e09bc27dee17bbdc9eeeb4ab0f984f6539e4c487463c725b34b9f48426dbf9",
    ),
    (
        "norm_linf",
        "d4446c6e32de4948c74bbb5b76daa0e1de4daebe00e44503e720f149b981da18",
    ),
    (
        "lerp",
        "fe010775a3074825cbf8e3b28cf598d7e7d3a598e7c3da185f7f9525c7b6e6a6",
    ),
    (
        "clamp",
        "0839f2e8df960550266d37fc3280ce60ba686bd5f97673bc8a44663ad7a96243",
    ),
    (
        "metric_new",
        "b472f053e4fe43fedcff6e06189ce4ca6cff559cafe0326cdccf449f6596bff5",
    ),
    (
        "metric_weights",
        "f73d181f731d1d95280a5855ffcc3bd7413ccc974395d6be5add6be82e89337a",
    ),
    (
        "metric_minimum",
        "92f1a9f945518c7aeeb964edfd07e8920ca3f45a4f2158c6a6fa07a57e6ecf08",
    ),
    (
        "metric_axis_extent",
        "631977eb6838a15b591abe0c3734da5dc1526735934c3994aeebf6c9b540338a",
    ),
    (
        "metric_euclidean_radius",
        "99caadab73f41d5a4419a900e5995dfeb5a14d657ac2b747f9aab1b7257b8a41",
    ),
    (
        "metric_normalised",
        "0584c61ae6d933378863f0b612e59bbbb36db0c0d7fc958a316568f93bc2cb56",
    ),
    (
        "sin",
        "84cf4edf30a5b6f644c1dc74dde2640e4467770ed5c35a296353c9e5b1174bf7",
    ),
    (
        "cos",
        "8310296979062107b644a1d340ca181facf1be5ea5122af76a74ec26eb33b38c",
    ),
    (
        "sin_cos",
        "19566da559090072ddbde9238a32500d4f254162991648082813f0fe504f739f",
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
        "bbdce4d0d01cac1ca2389e7bfdee76fdba318edf3801cdc2a70888611105f3f3",
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
        "3f4ae38799a1c0e4e19ed084c94b07dd2f65c052c7b4a3572a818ed3656aaf41",
    ),
    (
        "sqrt",
        "64877c5a8a670097edf21dd0aeca4faf56ad9925952e7c48cb460ea5b7fb829d",
    ),
    (
        "max",
        "93fd4a229d231ca16606ded184e71469c8dbffb54e62232d6086edf858240c78",
    ),
    (
        "min",
        "49f0b49b870d11dc1be9d2ffbf1f1f887400e93217943daaa885e145d6c9c571",
    ),
    (
        "max64",
        "6ae804e54c77d6cc2d690465511daa4bfa73f3345751c521bba499d49925d89c",
    ),
    (
        "min64",
        "24b06cd31dc3324a15bad0de2605ba8177ef471e30aae60c95104e0a9f009aa4",
    ),
    (
        "clamp64",
        "2f47d28209a3ba74192164d26004239137b9dde7ba13388f5f67d05950e5af65",
    ),
    (
        "argmax",
        "3d72c9a3a4e83ab2f894e9383b249befe3ea359e8b79b63d71bbcc4488380c08",
    ),
    (
        "argmin",
        "b4cd46d934eab486027ba62654ef06b5015344070a2b241c5062d3705ce23edf",
    ),
    (
        "ops_greatest",
        "195a5bffc7b88b2c1afbdfd618569fe47dc6764a39736a130fba34984fb06585",
    ),
    (
        "ops_least",
        "fe579644fa6ccdc2bfdc938509075b9ae3b7d5753be3553239fc71f56852d064",
    ),
    (
        "ops_offer",
        "5fb561d162a8ae64ae64c9dcda9b5e6a6ea6084a5908d15fa2322914d74c6d3d",
    ),
    (
        "ops_into_inner",
        "cf35da849d1984d2d8beaa944a8ddc21e8c25ea0114bed054e8ad180a2ef7df0",
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
        "b0cfa27c69e463997beae85c03c962fa14995026c465dba50005833862049284",
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
    (
        "specials",
        "ea1b9b7f266cd4c3c72a92ef6f1b0c9282479091aa100c43d077b1aabbd314aa",
    ),
];

/// Pins renamed since `a2e46794…`: none.
const RENAMED_SINCE_A2E46794: &[(&str, &str)] = &[];

/// One pin table per entry of [`alice_det_math::PREVIOUS_SEMANTICS_IDS`], in
/// the same order, with the renames that map its names onto [`GOLDEN`].
/// A pin table: `(name, 64 hex digits)` per function.
type PinTable = &'static [(&'static str, &'static str)];

/// Recomputes a past release's `specials` aggregate pin, restricted to the
/// function list that release's own specials sweep covered.
///
/// `specials` is not a per-function pin: it is one hash folded over every
/// function `common::specials::visit` calls, and that function list grows
/// whenever a release adds a public numeric function. So a stored `specials`
/// hash can never be "reproduced" by comparing it to the live `specials`
/// pin in [`GOLDEN`] once the two releases cover different function sets —
/// the live aggregate necessarily differs even when every function the old
/// release covered still returns the same bits. Each historical table that
/// had a `specials` pin instead freezes its own recomputation: the exact
/// historical call sequence (copied verbatim, never edited), run against
/// this release's current implementations. If it still folds to the stored
/// hash, every one of those old functions' special-value (and NaN,
/// subnormal, signed-zero) behaviour is unchanged; a function this release
/// added is not exercised by it at all, by construction.
type SpecialsRecompute = fn() -> String;

/// 0.5.0's `specials` sweep, frozen: copied verbatim from
/// `tests/common/specials.rs` at tag `v0.5.0`
/// (`git show v0.5.0:tests/common/specials.rs`), minus the two lines that do
/// not exist until this release (`f2(s, "max", max)`, `f2(s, "min", min)`,
/// `d2(s, "max64", max64)`, `d2(s, "min64", min64)`). Not edited beyond that
/// omission: adding a 0.5.1-only function here would defeat the point (this
/// must call only what 0.5.0 called). [`recompute`] is this release's only
/// caller of it.
mod specials_v0_5_0 {
    use super::common::specials::{Sink, SPECIALS32, SPECIALS64};
    use super::HashSink;
    use alice_det_math::metric::{MetricError, MetricWeights};
    use sha2::{Digest, Sha256};

    fn f1(s: &mut impl Sink, name: &'static str, f: impl Fn(f32) -> f32) {
        s.name(name);
        for x in SPECIALS32 {
            s.v32(f(x));
        }
    }

    fn f2(s: &mut impl Sink, name: &'static str, f: impl Fn(f32, f32) -> f32) {
        s.name(name);
        for x in SPECIALS32 {
            for y in SPECIALS32 {
                s.v32(f(x, y));
            }
        }
    }

    fn f3(s: &mut impl Sink, name: &'static str, f: impl Fn(f32, f32, f32) -> f32) {
        s.name(name);
        for x in SPECIALS32 {
            for y in SPECIALS32 {
                for z in SPECIALS32 {
                    s.v32(f(x, y, z));
                }
            }
        }
    }

    fn d1(s: &mut impl Sink, name: &'static str, f: impl Fn(f64) -> f64) {
        s.name(name);
        for x in SPECIALS64 {
            s.v64(f(x));
        }
    }

    fn d2(s: &mut impl Sink, name: &'static str, f: impl Fn(f64, f64) -> f64) {
        s.name(name);
        for x in SPECIALS64 {
            for y in SPECIALS64 {
                s.v64(f(x, y));
            }
        }
    }

    fn visit(s: &mut impl Sink) {
        use alice_det_math::*;
        let weights = |a: f32, b: f32, c: f32| MetricWeights::new(a.abs(), b.abs(), c.abs());
        f1(s, "sin", sin);
        f1(s, "cos", cos);
        s.name("sin_cos");
        for x in SPECIALS32 {
            let (a, b) = sin_cos(x);
            s.v32(a);
            s.v32(b);
        }
        f1(s, "exp", exp);
        f1(s, "ln", ln);
        f1(s, "log2", log2);
        f1(s, "log10", log10);
        f2(s, "powf", powf);
        s.name("powi");
        for x in SPECIALS32 {
            for n in [-3, -2, -1, 0, 1, 2, 3, i32::MIN, i32::MAX] {
                s.v32(powi(x, n));
            }
        }
        f1(s, "cbrt", cbrt);
        f2(s, "hypot", hypot);
        f1(s, "atan", atan);
        f2(s, "atan2", atan2);
        f1(s, "asin", asin);
        f1(s, "acos", acos);
        f1(s, "tan", tan);
        f1(s, "tanh", tanh);
        f1(s, "round", round);
        f1(s, "sqrt", sqrt);
        f3(s, "smoothstep", metric::smoothstep);
        s.name("metric_norm");
        for w in [
            MetricWeights::new(0.3, 0.5, 0.2).unwrap(),
            MetricWeights::L1,
            MetricWeights::L2,
            MetricWeights::LINF,
        ] {
            for x in SPECIALS32 {
                for y in SPECIALS32 {
                    for z in SPECIALS32 {
                        s.v32(w.norm([x, y, z]));
                    }
                }
            }
        }
        f3(s, "metric_lipschitz", |a, b, c| {
            weights(a, b, c).map_or(-1.0, MetricWeights::lipschitz)
        });
        f3(s, "norm_l1", |x, y, z| metric::norm_l1([x, y, z]));
        f3(s, "norm_l2", |x, y, z| metric::norm_l2([x, y, z]));
        f3(s, "norm_linf", |x, y, z| metric::norm_linf([x, y, z]));
        f3(s, "lerp", metric::lerp);
        f3(s, "clamp", metric::clamp);
        f3(s, "metric_new", |a, b, c| {
            match MetricWeights::new(a, b, c) {
                Ok(w) => super::fold_weights(w),
                Err(MetricError::NotFinite) => -1.0,
                Err(MetricError::NotConvex) => -2.0,
                Err(MetricError::Degenerate) => -3.0,
                Err(_) => -4.0,
            }
        });
        f3(s, "metric_weights", |a, b, c| {
            weights(a, b, c).map_or(-1.0, super::fold_weights)
        });
        f3(s, "metric_minimum", |a, b, c| {
            weights(a, b, c).map_or(-1.0, MetricWeights::minimum)
        });
        f3(s, "metric_axis_extent", |a, b, c| {
            weights(a, b, c).map_or(-1.0, |w| w.axis_extent(a))
        });
        f3(s, "metric_euclidean_radius", |a, b, c| {
            weights(a, b, c).map_or(-1.0, |w| w.euclidean_radius(b))
        });
        s.name("metric_normalised");
        for a in SPECIALS32 {
            for b in SPECIALS32 {
                for c in SPECIALS32 {
                    match weights(a, b, c) {
                        Ok(w) => {
                            let (x, y, z) = w.normalised().weights();
                            s.v32(x);
                            s.v32(y);
                            s.v32(z);
                        }
                        Err(_) => s.v32(-1.0),
                    }
                }
            }
        }
        d1(s, "atan64", atan64);
        d2(s, "atan2_64", atan2_64);
        d1(s, "asin64", asin64);
        d1(s, "acos64", acos64);
        d1(s, "exp64", exp64);
        d1(s, "ln64", ln64);
        d1(s, "log2_64", log2_64);
        d1(s, "log10_64", log10_64);
        d1(s, "tan64", tan64);
        d2(s, "powf64", powf64);
        d1(s, "round64", round64);
        d1(s, "sqrt64", sqrt64);
        d1(s, "sin64", sin64);
        d1(s, "cos64", cos64);
        s.name("sin_cos64");
        for x in SPECIALS64 {
            let (a, b) = sin_cos64(x);
            s.v64(a);
            s.v64(b);
        }
    }

    pub fn recompute() -> String {
        let mut s = HashSink {
            h: Sha256::new(),
            names: Vec::new(),
        };
        visit(&mut s);
        format!("{:x}", s.h.finalize())
    }
}

/// The pin table of 0.6.0 (`a2a11b9b…dbe9`), copied verbatim from `tests/golden.rs` at the
/// 0.6.0 commit (`git show b4e3d94:tests/golden.rs`). Not edited: the proof below depends on it
/// being what that release pinned. Every function in it still returns these exact bits (0.6.1
/// only adds `tanh64` / `hypot64`), so its identifier is a
/// [`alice_det_math::PREVIOUS_SEMANTICS_IDS`] entry.
const GOLDEN_V0_6_0: &[(&str, &str)] = &[
    (
        "smoothstep",
        "184e9e4776478a4608497033c7e9f39e14b6b629b691a9bb9ac2a4313f2de33e",
    ),
    (
        "metric_norm",
        "00f35aac72ff55e1069259196427d703926b47f7b4349fb4cc7f3f7b8effdc60",
    ),
    (
        "metric_lipschitz",
        "ff8c5743709b8a02beb42eb169064e2bd7f290096c9c99ac40551583be3b1f58",
    ),
    (
        "norm_l1",
        "522716e421c074fd03f831a1fb5b51280da91c573c90530b02c522982c677839",
    ),
    (
        "norm_l2",
        "69e09bc27dee17bbdc9eeeb4ab0f984f6539e4c487463c725b34b9f48426dbf9",
    ),
    (
        "norm_linf",
        "d4446c6e32de4948c74bbb5b76daa0e1de4daebe00e44503e720f149b981da18",
    ),
    (
        "lerp",
        "fe010775a3074825cbf8e3b28cf598d7e7d3a598e7c3da185f7f9525c7b6e6a6",
    ),
    (
        "clamp",
        "0839f2e8df960550266d37fc3280ce60ba686bd5f97673bc8a44663ad7a96243",
    ),
    (
        "metric_new",
        "b472f053e4fe43fedcff6e06189ce4ca6cff559cafe0326cdccf449f6596bff5",
    ),
    (
        "metric_weights",
        "f73d181f731d1d95280a5855ffcc3bd7413ccc974395d6be5add6be82e89337a",
    ),
    (
        "metric_minimum",
        "92f1a9f945518c7aeeb964edfd07e8920ca3f45a4f2158c6a6fa07a57e6ecf08",
    ),
    (
        "metric_axis_extent",
        "631977eb6838a15b591abe0c3734da5dc1526735934c3994aeebf6c9b540338a",
    ),
    (
        "metric_euclidean_radius",
        "99caadab73f41d5a4419a900e5995dfeb5a14d657ac2b747f9aab1b7257b8a41",
    ),
    (
        "metric_normalised",
        "0584c61ae6d933378863f0b612e59bbbb36db0c0d7fc958a316568f93bc2cb56",
    ),
    (
        "sin",
        "a9feff05303cfc4107ac3c855048efaa636615e23a477f6ea04075ff684f2517",
    ),
    (
        "cos",
        "376f24f9f5d9ee37dfa51e464bb6ae341cb69978b1a0768bba95f5b16ec96f22",
    ),
    (
        "sin_cos",
        "d0bf4c21d5f13283893fb954b45c2f9127d33d7862e844d7a050b03bfecd778c",
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
        "09275f109b8a4c4c7198e834fdd6f0f2083ab6db85909da5e847e993b5db1bfa",
    ),
    (
        "powi",
        "769743b0123bb1c3926c8ec7445002b3de11fb6476f438dc67139313c2064117",
    ),
    (
        "cbrt",
        "bbdce4d0d01cac1ca2389e7bfdee76fdba318edf3801cdc2a70888611105f3f3",
    ),
    (
        "hypot",
        "ec4c7823be970a04dd167bea5e3ee390f95fbb87fa223889c5519ea8241acdec",
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
        "erf",
        "d7b2c87da116e262612481accc1083b043b398a580aea23bfd020c8b3273d824",
    ),
    (
        "round",
        "3f4ae38799a1c0e4e19ed084c94b07dd2f65c052c7b4a3572a818ed3656aaf41",
    ),
    (
        "sqrt",
        "64877c5a8a670097edf21dd0aeca4faf56ad9925952e7c48cb460ea5b7fb829d",
    ),
    (
        "max",
        "93fd4a229d231ca16606ded184e71469c8dbffb54e62232d6086edf858240c78",
    ),
    (
        "min",
        "49f0b49b870d11dc1be9d2ffbf1f1f887400e93217943daaa885e145d6c9c571",
    ),
    (
        "max64",
        "6ae804e54c77d6cc2d690465511daa4bfa73f3345751c521bba499d49925d89c",
    ),
    (
        "min64",
        "24b06cd31dc3324a15bad0de2605ba8177ef471e30aae60c95104e0a9f009aa4",
    ),
    (
        "clamp64",
        "2f47d28209a3ba74192164d26004239137b9dde7ba13388f5f67d05950e5af65",
    ),
    (
        "argmax",
        "3d72c9a3a4e83ab2f894e9383b249befe3ea359e8b79b63d71bbcc4488380c08",
    ),
    (
        "argmin",
        "b4cd46d934eab486027ba62654ef06b5015344070a2b241c5062d3705ce23edf",
    ),
    (
        "ops_greatest",
        "195a5bffc7b88b2c1afbdfd618569fe47dc6764a39736a130fba34984fb06585",
    ),
    (
        "ops_least",
        "fe579644fa6ccdc2bfdc938509075b9ae3b7d5753be3553239fc71f56852d064",
    ),
    (
        "ops_offer",
        "5fb561d162a8ae64ae64c9dcda9b5e6a6ea6084a5908d15fa2322914d74c6d3d",
    ),
    (
        "ops_into_inner",
        "cf35da849d1984d2d8beaa944a8ddc21e8c25ea0114bed054e8ad180a2ef7df0",
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
        "49fec7cfa239dadfe4fcd1dcb49fc3f09ca5eaa25c5497b254e17e01feaeea5a",
    ),
    (
        "round64",
        "b0cfa27c69e463997beae85c03c962fa14995026c465dba50005833862049284",
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
        "erf64",
        "557e03d68112056ca4226847a5542141e35949ef2079282883b80b2697997146",
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
    (
        "specials",
        "06190bb8fc647ea81c08031a1ad3cf56e1b6a3baed8914b7535614b6da2b7893",
    ),
];

/// No pin was renamed between 0.6.0 and this release.
const RENAMED_SINCE_V0_6_0: &[(&str, &str)] = &[];

/// 0.6.0's `specials` sweep, frozen: copied verbatim from `tests/common/specials.rs` at the 0.6.0
/// commit (`git show b4e3d94:tests/common/specials.rs`), from `fold_weights` on. Not
/// edited: adding a 0.6.1 function here would defeat the point (this must call only what 0.6.0
/// called). [`recompute`] is this release's only caller of it.
mod specials_v0_6_0 {
    #![allow(dead_code)]
    use super::common::specials::{Sink, SPECIALS32, SPECIALS64};
    use super::HashSink;
    use alice_det_math::metric::{MetricError, MetricWeights};
    use sha2::{Digest, Sha256};

    pub fn fold_weights(w: MetricWeights) -> f32 {
        let (l1, l2, linf) = w.weights();
        l1 + 2.0 * l2 + 4.0 * linf
    }

    fn f1(s: &mut impl Sink, name: &'static str, f: impl Fn(f32) -> f32) {
        s.name(name);
        for x in SPECIALS32 {
            s.v32(f(x));
        }
    }

    fn f2(s: &mut impl Sink, name: &'static str, f: impl Fn(f32, f32) -> f32) {
        s.name(name);
        for x in SPECIALS32 {
            for y in SPECIALS32 {
                s.v32(f(x, y));
            }
        }
    }

    fn f3(s: &mut impl Sink, name: &'static str, f: impl Fn(f32, f32, f32) -> f32) {
        s.name(name);
        for x in SPECIALS32 {
            for y in SPECIALS32 {
                for z in SPECIALS32 {
                    s.v32(f(x, y, z));
                }
            }
        }
    }

    fn d1(s: &mut impl Sink, name: &'static str, f: impl Fn(f64) -> f64) {
        s.name(name);
        for x in SPECIALS64 {
            s.v64(f(x));
        }
    }

    fn d2(s: &mut impl Sink, name: &'static str, f: impl Fn(f64, f64) -> f64) {
        s.name(name);
        for x in SPECIALS64 {
            for y in SPECIALS64 {
                s.v64(f(x, y));
            }
        }
    }

    /// Every public scalar function, under its golden pin name. `sin` is passed
    /// in (`sin_under_test`, a name the glob import below cannot shadow) so a
    /// test can substitute a broken one and see the result move.
    pub fn visit(s: &mut impl Sink, sin_under_test: fn(f32) -> f32) {
        use alice_det_math::*;
        let weights = |a: f32, b: f32, c: f32| MetricWeights::new(a.abs(), b.abs(), c.abs());
        f1(s, "sin", sin_under_test);
        f1(s, "cos", cos);
        // both halves, each in full (not folded: a NaN in one must stay visible)
        s.name("sin_cos");
        for x in SPECIALS32 {
            let (a, b) = sin_cos(x);
            s.v32(a);
            s.v32(b);
        }
        f1(s, "exp", exp);
        f1(s, "ln", ln);
        f1(s, "log2", log2);
        f1(s, "log10", log10);
        f2(s, "powf", powf);
        s.name("powi");
        for x in SPECIALS32 {
            for n in [-3, -2, -1, 0, 1, 2, 3, i32::MIN, i32::MAX] {
                s.v32(powi(x, n));
            }
        }
        f1(s, "cbrt", cbrt);
        f2(s, "hypot", hypot);
        f1(s, "atan", atan);
        f2(s, "atan2", atan2);
        f1(s, "asin", asin);
        f1(s, "acos", acos);
        f1(s, "tan", tan);
        f1(s, "tanh", tanh);
        f1(s, "erf", erf);
        f1(s, "round", round);
        f1(s, "sqrt", sqrt);
        f2(s, "max", max);
        f2(s, "min", min);
        f3(s, "smoothstep", metric::smoothstep);
        // a mixed metric, and the named ones (two zero weights each: `0·inf`)
        s.name("metric_norm");
        for w in [
            MetricWeights::new(0.3, 0.5, 0.2).unwrap(),
            MetricWeights::L1,
            MetricWeights::L2,
            MetricWeights::LINF,
        ] {
            for x in SPECIALS32 {
                for y in SPECIALS32 {
                    for z in SPECIALS32 {
                        s.v32(w.norm([x, y, z]));
                    }
                }
            }
        }
        f3(s, "metric_lipschitz", |a, b, c| {
            weights(a, b, c).map_or(-1.0, MetricWeights::lipschitz)
        });
        f3(s, "norm_l1", |x, y, z| metric::norm_l1([x, y, z]));
        f3(s, "norm_l2", |x, y, z| metric::norm_l2([x, y, z]));
        f3(s, "norm_linf", |x, y, z| metric::norm_linf([x, y, z]));
        f3(s, "lerp", metric::lerp);
        f3(s, "clamp", metric::clamp);
        let index = |i: Option<usize>| i.map_or(-1.0, |i| i as f32);
        f3(s, "argmax", |a, b, c| index(argmax(&[a, b, c])));
        f3(s, "argmin", |a, b, c| index(argmin(&[a, b, c])));
        let select = |mut sel: Select<u8>, a: f32, b: f32, c: f32| {
            sel.offer(a, 0);
            sel.offer(b, 1);
            sel.offer(c, 2);
            sel.into_inner()
        };
        f3(s, "ops_greatest", |a, b, c| {
            select(Select::greatest(), a, b, c).map_or(-1.0, |(v, _)| v)
        });
        f3(s, "ops_least", |a, b, c| {
            select(Select::least(), a, b, c).map_or(-1.0, |(v, _)| v)
        });
        f3(s, "ops_offer", |a, b, c| {
            select(Select::greatest(), c, b, a).map_or(-1.0, |(_, i)| f32::from(i))
        });
        f3(s, "ops_into_inner", |a, b, c| {
            select(Select::least(), c, b, a).map_or(-1.0, |(_, i)| f32::from(i))
        });
        f3(s, "metric_new", |a, b, c| {
            match MetricWeights::new(a, b, c) {
                Ok(w) => fold_weights(w),
                Err(MetricError::NotFinite) => -1.0,
                Err(MetricError::NotConvex) => -2.0,
                Err(MetricError::Degenerate) => -3.0,
                Err(_) => -4.0,
            }
        });
        f3(s, "metric_weights", |a, b, c| {
            weights(a, b, c).map_or(-1.0, fold_weights)
        });
        f3(s, "metric_minimum", |a, b, c| {
            weights(a, b, c).map_or(-1.0, MetricWeights::minimum)
        });
        f3(s, "metric_axis_extent", |a, b, c| {
            weights(a, b, c).map_or(-1.0, |w| w.axis_extent(a))
        });
        f3(s, "metric_euclidean_radius", |a, b, c| {
            weights(a, b, c).map_or(-1.0, |w| w.euclidean_radius(b))
        });
        s.name("metric_normalised");
        for a in SPECIALS32 {
            for b in SPECIALS32 {
                for c in SPECIALS32 {
                    match weights(a, b, c) {
                        Ok(w) => {
                            let (x, y, z) = w.normalised().weights();
                            s.v32(x);
                            s.v32(y);
                            s.v32(z);
                        }
                        Err(_) => s.v32(-1.0),
                    }
                }
            }
        }
        d1(s, "atan64", atan64);
        d2(s, "atan2_64", atan2_64);
        d1(s, "asin64", asin64);
        d1(s, "acos64", acos64);
        d1(s, "exp64", exp64);
        d1(s, "ln64", ln64);
        d1(s, "log2_64", log2_64);
        d1(s, "log10_64", log10_64);
        d1(s, "tan64", tan64);
        d1(s, "erf64", erf64);
        d2(s, "powf64", powf64);
        d1(s, "round64", round64);
        d1(s, "sqrt64", sqrt64);
        d2(s, "max64", max64);
        d2(s, "min64", min64);
        s.name("clamp64");
        for x in SPECIALS64 {
            for lo in SPECIALS64 {
                for hi in SPECIALS64 {
                    s.v64(clamp64(x, lo, hi));
                }
            }
        }
        d1(s, "sin64", sin64);
        d1(s, "cos64", cos64);
        s.name("sin_cos64");
        for x in SPECIALS64 {
            let (a, b) = sin_cos64(x);
            s.v64(a);
            s.v64(b);
        }
    }

    pub fn recompute() -> String {
        let mut s = HashSink {
            h: Sha256::new(),
            names: Vec::new(),
        };
        visit(&mut s, alice_det_math::sin);
        format!("{:x}", s.h.finalize())
    }
}

/// 0.6.0's identifier, reproduced: its pin table with no renames and its frozen specials sweep.
const PREVIOUS_TABLES: &[(PinTable, PinTable, Option<SpecialsRecompute>)] = &[(
    GOLDEN_V0_6_0,
    RENAMED_SINCE_V0_6_0,
    Some(specials_v0_6_0::recompute),
)];

/// Every previous identifier is backed by its release's pin table, and this
/// release reproduces every pin in it.
///
/// The tests above recompute every hash in [`GOLDEN`] from this release's
/// code. So if each per-function pin of a previous table is equal to the
/// [`GOLDEN`] entry for the same function, and its `specials` pin (if it has
/// one) is equal to that release's own frozen recomputation
/// ([`SpecialsRecompute`]), this release returns that release's bits on
/// every function the release covered; and if the table folds to the
/// identifier, the identifier is that release's. Both together are what
/// lets a reader accept the identifier as [`SemanticsCheck::VerifiedPrevious`].
#[test]
fn every_previous_identifier_is_backed_by_a_reproduced_pin_table() {
    use alice_det_math::{check_semantics, SemanticsCheck, PREVIOUS_SEMANTICS_IDS, SEMANTICS_ID};
    assert_eq!(
        PREVIOUS_TABLES.len(),
        PREVIOUS_SEMANTICS_IDS.len(),
        "each previous identifier needs exactly one pin table"
    );
    let mut compared = 0usize;
    for ((table, renames, recompute), id) in PREVIOUS_TABLES.iter().zip(PREVIOUS_SEMANTICS_IDS) {
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
            if *name == "specials" {
                let got = recompute.unwrap_or_else(|| {
                    panic!("`specials` is pinned but this table has no frozen recomputation")
                })();
                assert_eq!(
                    &got, hash,
                    "`specials`, recomputed over its frozen function list, returns different \
                     bits than the release that stamped this identifier"
                );
                compared += 1;
                continue;
            }
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
    let pins: usize = PREVIOUS_TABLES.iter().map(|(t, _, _)| t.len()).sum();
    assert_eq!(compared, pins, "pins compared");
}

/// Pins of an old table that this release no longer reproduces, by current name.
fn moved_pins(table: PinTable, renames: PinTable) -> (Vec<&'static str>, usize) {
    let mut moved = Vec::new();
    for (name, hash) in table {
        let current = renames
            .iter()
            .find(|(old, _)| old == name)
            .map_or(*name, |(_, new)| *new);
        let now = GOLDEN
            .iter()
            .find(|(n, _)| *n == current)
            .unwrap_or_else(|| panic!("`{name}` (now `{current}`) is no longer pinned"));
        if now.1 != *hash {
            moved.push(now.0);
        }
    }
    (moved, table.len())
}

/// 0.4.0's identifier is not accepted any more, and the reason is exactly the
/// changes of this release, each visible in its pins: `sin` / `cos` /
/// `sin_cos` (the reduction near their zeros), `round` / `round64` / `cbrt` /
/// `smoothstep` (a NaN argument returns the canonical NaN instead of its own
/// payload) and `metric_lipschitz` (exact for weights whose radical under- or
/// overflowed). Its other 32 pins are still reproduced. The `specials` pin is
/// new in this release and is not in this table. A further change to any
/// function moves this set and fails here.
#[test]
fn the_0_4_0_identifier_is_rejected_because_exactly_these_functions_changed() {
    use alice_det_math::{check_semantics, SemanticsCheck, PREVIOUS_SEMANTICS_IDS, SEMANTICS_ID};
    let id = fold_semantics_id(GOLDEN_V0_4_0).expect("the 0.4.0 table folds");
    assert_eq!(
        id,
        hex32("d2209b30f6f1f45baa1b638bcdfee34ac64773b2e63b9c083b2e77afc691398e").unwrap(),
        "the 0.4.0 table does not fold to the 0.4.0 identifier"
    );
    assert_ne!(id, SEMANTICS_ID);
    assert!(!PREVIOUS_SEMANTICS_IDS.contains(&id));
    assert_eq!(check_semantics(&id), SemanticsCheck::Mismatch);
    let (moved, compared) = moved_pins(GOLDEN_V0_4_0, RENAMED_SINCE_V0_4_0);
    assert_eq!(compared, 40, "pins compared");
    assert_eq!(
        moved,
        [
            "smoothstep",
            "metric_lipschitz",
            "sin",
            "cos",
            "sin_cos",
            "powf",
            "cbrt",
            "hypot",
            "round",
            "powf64",
            "round64"
        ]
    );
    assert!(!GOLDEN_V0_4_0.iter().any(|(n, _)| *n == "specials"));
}

/// The 0.5.0 identifier (`135c407d…`) and the 0.5.1 / 0.5.2 one (`85e08846…`) are not accepted
/// any more, and the reason is exactly this release's change: `clamp` now follows the G2 rule
/// (`min(max(x, lo), hi)`: the G2 order of an equal pair of zeros, a NaN bound is no bound, the
/// high bound wins over an inverted low one). Every other pin of both tables is still
/// reproduced; the aggregate `specials` pin moves with it, and 0.5.0's frozen sweep, rerun on
/// this release's functions, no longer returns 0.5.0's `specials` hash. A further change to any
/// function moves these sets and fails here.
#[test]
fn the_0_5_identifiers_are_rejected_because_exactly_clamp_changed() {
    use alice_det_math::{check_semantics, SemanticsCheck, PREVIOUS_SEMANTICS_IDS, SEMANTICS_ID};
    for (table, renames, id_hex, pins) in [
        (
            GOLDEN_V0_5_0,
            RENAMED_SINCE_V0_5_0,
            "135c407d4249c1c200e4d1be79a12e848ce5c355fc3a918da965849e697ef000",
            52,
        ),
        (
            GOLDEN_V0_5_1,
            RENAMED_SINCE_V0_5_1,
            "85e0884623b154a3b2b264374907b6cbac09307ca3f9a08339e48c567d3cb315",
            56,
        ),
    ] {
        let id = fold_semantics_id(table).expect("the table folds");
        assert_eq!(
            id,
            hex32(id_hex).unwrap(),
            "the table does not fold to its identifier"
        );
        assert_ne!(id, SEMANTICS_ID);
        assert!(!PREVIOUS_SEMANTICS_IDS.contains(&id));
        assert_eq!(check_semantics(&id), SemanticsCheck::Mismatch);
        let (moved, compared) = moved_pins(table, renames);
        assert_eq!(compared, pins, "pins compared");
        assert_eq!(
            moved,
            ["clamp", "sin", "cos", "sin_cos", "powf", "hypot", "powf64", "specials"],
            "{id_hex}"
        );
    }
    let stored = GOLDEN_V0_5_0
        .iter()
        .find(|(n, _)| *n == "specials")
        .unwrap()
        .1;
    assert_ne!(
        specials_v0_5_0::recompute(),
        stored,
        "0.5.0's sweep still reproduces its hash"
    );
}

/// The unpublished identifier `a2e46794…e3cf` (main up to 39507d1, taken by git rev) is not
/// accepted any more, and the reason is exactly this release's changes, each visible in its
/// pins: `sin` / `cos` / `sin_cos` (`|x| ≥ 2^13` takes the double-precision kernels), `powf`
/// (IEEE 754 negative bases), `powf64` (fdlibm `e_pow.c`) and `hypot` (two subnormal
/// operands); the aggregate `specials` pin moves with them. `erf` / `erf64` are new and are
/// not in that table. Every other pin is still reproduced. A further change to any function
/// moves this set and fails here.
#[test]
fn the_a2e46794_identifier_is_rejected_because_exactly_these_functions_changed() {
    use alice_det_math::{check_semantics, SemanticsCheck, PREVIOUS_SEMANTICS_IDS, SEMANTICS_ID};
    let id = fold_semantics_id(GOLDEN_A2E46794).expect("the a2e46794 table folds");
    assert_eq!(
        id,
        hex32("a2e46794a49b62730d8eb75d10e6dbfdd59d6562a985ae94c716fc637a57e3cf").unwrap(),
        "the frozen table does not fold to its identifier"
    );
    assert_ne!(id, SEMANTICS_ID);
    assert!(!PREVIOUS_SEMANTICS_IDS.contains(&id));
    assert_eq!(check_semantics(&id), SemanticsCheck::Mismatch);
    let (moved, compared) = moved_pins(GOLDEN_A2E46794, RENAMED_SINCE_A2E46794);
    assert_eq!(compared, 63, "pins compared");
    assert_eq!(
        moved,
        ["sin", "cos", "sin_cos", "powf", "hypot", "powf64", "specials"]
    );
    assert!(!GOLDEN_A2E46794.iter().any(|(n, _)| n.starts_with("erf")));
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

/// The default NaN an x86-64 operation raises (sign bit set): never the canonical one.
const X86_NAN32: u32 = 0xffc0_0000;
/// [`X86_NAN32`] for `f64`.
const X86_NAN64: u64 = 0xfff8_0000_0000_0000;

// Every digest helper above refuses an output NaN other than the canonical one. On a target
// whose arithmetic raises the canonical NaN, a helper that read output bits raw would pass
// every pin here and still hash a leaked NaN on another target; these feed it one directly.

#[test]
#[should_panic(expected = "non-canonical NaN 0xffc00000")]
fn hash32_refuses_a_non_canonical_nan_output() {
    let _ = hash32(&[1.0], |_| f32::from_bits(X86_NAN32));
}

#[test]
#[should_panic(expected = "non-canonical NaN 0xffc00000")]
fn hash32x2_refuses_a_non_canonical_nan_output() {
    let _ = hash32x2(&[1.0, 2.0], |_, _| f32::from_bits(X86_NAN32));
}

#[test]
#[should_panic(expected = "non-canonical NaN 0xffc00000")]
fn hash32x3_refuses_a_non_canonical_nan_output() {
    let _ = hash32x3(&[1.0, 2.0, 3.0], |_, _, _| f32::from_bits(X86_NAN32));
}

#[test]
#[should_panic(expected = "non-canonical NaN 0xfff8000000000000")]
fn hash64_refuses_a_non_canonical_nan_output() {
    let _ = hash64(&[1.0], |_| f64::from_bits(X86_NAN64));
}

#[test]
#[should_panic(expected = "non-canonical NaN 0xffc00000")]
fn the_sin_cos_fold_refuses_a_non_canonical_sine() {
    let _ = sin_cos_fold32(f32::from_bits(X86_NAN32), 1.0);
}

#[test]
#[should_panic(expected = "non-canonical NaN 0xffc00000")]
fn the_sin_cos_fold_refuses_a_non_canonical_cosine() {
    let _ = sin_cos_fold32(0.0, f32::from_bits(X86_NAN32));
}

#[test]
#[should_panic(expected = "non-canonical NaN 0xfff8000000000000")]
fn the_sin_cos64_fold_refuses_a_non_canonical_sine() {
    let _ = sin_cos_fold64(f64::from_bits(X86_NAN64), 1.0);
}

#[test]
#[should_panic(expected = "non-canonical NaN 0xfff8000000000000")]
fn the_sin_cos64_fold_refuses_a_non_canonical_cosine() {
    let _ = sin_cos_fold64(0.0, f64::from_bits(X86_NAN64));
}

#[test]
#[should_panic(expected = "non-canonical NaN 0xffc00000")]
fn the_specials_sink_refuses_a_non_canonical_f32_output() {
    use common::specials::Sink as _;
    let mut s = HashSink {
        h: Sha256::new(),
        names: Vec::new(),
    };
    s.v32(f32::from_bits(X86_NAN32));
}

#[test]
#[should_panic(expected = "non-canonical NaN 0xfff8000000000000")]
fn the_specials_sink_refuses_a_non_canonical_f64_output() {
    use common::specials::Sink as _;
    let mut s = HashSink {
        h: Sha256::new(),
        names: Vec::new(),
    };
    s.v64(f64::from_bits(X86_NAN64));
}
