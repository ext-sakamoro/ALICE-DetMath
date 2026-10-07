# alice-det-math

[![CI](https://github.com/ext-sakamoro/ALICE-DetMath/actions/workflows/ci.yml/badge.svg)](https://github.com/ext-sakamoro/ALICE-DetMath/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/alice-det-math.svg)](https://crates.io/crates/alice-det-math)
[![docs.rs](https://docs.rs/alice-det-math/badge.svg)](https://docs.rs/alice-det-math)

[English](README.md)

プラットフォームをまたいで**ビット単位で一致する** `f32` / `f64` の超越関数 — `sin`、`cos`、`exp`、
<!-- claim-test: scalar_outputs_match_recorded_hashes -->
`ln`、`atan2`、`asin`、`acos`、`tan`、`tanh`、`powf`、`cbrt`、`hypot` など — を
IEEE 754 の基本演算だけで組み立てる そのため各関数は x86_64、aarch64、wasm32 を
はじめ IEEE 準拠のどのターゲットでも、入力のビットだけで決まる純関数になる
`no_std`、`alloc` 不要、`unsafe` なし

プラットフォームの `libm` はそうならない `sin(x)` は macOS、glibc、MSVC、wasm の
間で最後の 1 ulp が異なる それを呼ぶロックステップシミュレーションはピア間で
発散し、2 台で評価した符号付き距離場は点が曲面のどちら側にあるかで食い違う
この crate は [alice-physics](https://crates.io/crates/alice-physics) (128 bit
決定論物理) と [alice-sdf](https://crates.io/crates/alice-sdf) (SDF 幾何) の
超越関数層を切り出したもので、両者とそれ以外の利用者が同じ法則を評価できる

より速い `libm` でも、より正確な `libm` でもない 誤差は正しく丸めた値から
1〜2 ulp 以内で、正しく丸められているわけではない また各ターゲットで最速の
演算順ではなく、固定の演算順 (`mul_add` なし) を守る 2 台のマシンが同じビットを
出す必要がある場面で使う 値だけが問題ならプラットフォームの `libm` の方が適している

License: MIT OR Apache-2.0

## 目次

- [インストール](#インストール)
- [使用例](#使用例)
- [機能](#機能)
- [`SEMANTICS_ID`](#semantics_id)
- [利用側の方針](#利用側の方針)
- [精度と保証の範囲](#精度と保証の範囲)
- [no_std](#no_std)
- [最低サポート Rust バージョン](#最低サポート-rust-バージョン)
- [ビルドとテスト](#ビルドとテスト)
- [関連プロジェクト](#関連プロジェクト)
- [ライセンス](#ライセンス)

## インストール

```sh
cargo add alice-det-math
cargo add alice-det-math --features simd            # f32x8 versions
cargo add alice-det-math --no-default-features      # no_std (software sqrt)
```

## 使用例

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

`simd` を有効にした場合:

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

## 機能

- **スカラー**の `f32` カーネル (Cephes の `sinf` / `cosf` / `expf`、musl の `logf`) と
  `f64` カーネル (fdlibm の `atan` / `atan2` / `asin` / `acos` / `exp` / `log`)
  文書化した定義域で正しく丸めた値から 1〜2 ulp 以内
- **`sin64` / `cos64` / `sin_cos64`**: 有限の全引数に対する `f64` の正弦と余弦
  musl の `sin.c` / `cos.c` に、`|x| ≥ 2^20·π/2` では Payne–Hanek 縮約を組み合わせる
  正しく丸めた値から 1 ulp 以内 (libm ではなく `mpmath` を 2400 bit で用いて求めた
  6234 点と比較) `sin_cos64` は縮約を 1 回で済ませ、個別の 2 回の呼び出しとビット一致する
- **`tan64`**: 有限の全引数に対する `f64` の正接 同じ縮約の上で fdlibm の
<!-- claim-test: tan64_within_1_ulp_of_correctly_rounded_reference -->
  `k_tan.c` を用い、正しく丸めた値から 1 ulp 以内 (`mpmath` を 2400 bit で用いて
  求めた 6310 点と比較) プラットフォームの libm とは一切比較しない — Payne–Hanek
  の領域で約 1.0e5 ulp 外れる環境と 2 ulp に収まる環境があり、その差に閾値を
  置くと走らせた機械で結果が変わるため
- **`log2` / `log10` / `log2_64` / `log10_64`**: fdlibm の `e_log10.c` の指数分解
<!-- claim-test: log2_log10_within_1_ulp_of_correctly_rounded -->
  による 2 を底とする対数と 10 を底とする対数 指数が負のとき仮数を 1 未満に
  寄せるので `x = 1` の近傍で桁落ちしない 2 の冪は最小の非正規化数まで指数を
  厳密に返す `log2_64` は 2 ulp 以内、`log10_64` は 1 ulp 以内で、`f32` の入口は
  `f64` カーネルを 1 回丸めたもの (正しく丸めた値との差は実測 0 ulp)
- **SIMD** (`simd` feature): `sin` / `cos` / `sin_cos` / `exp` / `ln` / `round` /
  `sqrt` の `wide::f32x8` 版 スカラー版とレーンごとにビット一致する — 同じ定数、
  同じ演算順、`mul_add` なし
- **`metric`**: 演算順を 1 つに固定した `lerp` / `clamp` / `smoothstep`、
<!-- claim-test: every_nan_these_kernels_create_is_the_canonical_one -->
  3 次元の 3 種のノルム、および `MetricWeights` — `‖·‖₁` / `‖·‖₂` / `‖·‖∞` の
  非負結合として作る計量で、Lipschitz 定数と、半径 `r` の球がユークリッド空間で
  どこまで届くかの厳密な閉形式を持つ
- **libm を使わない `round` / `sqrt`**: musl の `x + 2^23 − 2^23` による丸めと、
  正しく丸めるソフトウェア平方根 ベアメタルのターゲットでも動き、同じビットを返す
- **正準 NaN**: 入力の NaN を算術に通さない (ペイロードと符号の扱いがプラットフォーム
  依存のため) どの関数も正準 NaN を返すか、入力のビットをそのまま返す
- **固定**: `tests/golden.rs` は固定の入力格子に対する全関数の出力の SHA-256 を持つ
  CI は macOS ARM / Intel、Linux x86 / ARM、Windows、wasm32 (wasmtime)、
  SSE2 のみの SIMD フォールバック、AVX2 + FMA の強制有効、`no_std` (ソフトウェア
  `sqrt`) でこれを再現する
- **`SEMANTICS_ID`**: この crate の数値的な挙動を識別する 32 byte の定数 1 つ
<!-- claim-test: semantics_id_matches_the_recorded_constant -->
  (下記参照)

## `SEMANTICS_ID`

`SEMANTICS_ID` は **この crate が算術をどう評価するか**を識別する 32 byte の定数
<!-- claim-test: semantics_id_covers_every_public_numeric_function -->
ここにあるどの関数かが同じ入力に対して違うビットを返すようになったときに変わり、
そのときだけ変わる 値は `tests/golden.rs` の関数ごとのビット固定値を、関数名の
昇順に、名前の長さを 4 byte の big-endian、続いて名前、続いてその 32 byte の
固定値として連結した SHA-256 プラットフォームの `libm`、時刻、環境は一切入らない
ので、固定値が再現する全ターゲットで同じ値になる

用途は、式とそのパラメータから識別子を作る利用側 この値を混ぜると識別子が算術まで
覆うので、式・パラメータ・この値が一致する 2 回の実行は同じビットを計算したことに
なり、この値が違えば、他が完全に一致していても同じビットではない

2 つの test が乖離を防ぐ 1 つは固定値から再計算して定数とのずれで落ちる もう 1 つは
`src/lib.rs` から crate の公開数値関数を読み出し、固定値を持たない関数があれば
落ちる (表の外にある関数は、識別子を動かさずに挙動を変えられてしまうため)

0.4.0 で `atan64` の係数を修正して出力ビットが変わったため、この値も変わった
これは意図した挙動で、この値を自分の識別子に混ぜている利用側は、算術の変更を
黙って引き継ぐのではなく識別子の変化として受け取る

## 利用側の方針

プラットフォームの `libm` メソッドを `clippy.toml` の `disallowed-methods` に
列挙し、clippy を `-D warnings` で走らせる そうすれば浮動小数点数に対する
`x.sin()` の紛れ込みはゲートで落ちる

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

## 精度と保証の範囲

関数ごとの誤差の表と、ビット一致の保証の正確な範囲 (ターゲットが IEEE 754 の
基本演算を守ること SSE2 のない x87 と fast-math ビルドは範囲外) は crate の
ドキュメント (`cargo doc --open`) を参照

閾値を assert している `f64` 関数の参照値はすべて `tests/data/` にコミットしてあり、
`f64` の閾値はどれもプラットフォームの `libm` に依存しない (`atan64` のみ例外)
次のコマンドで再生成できる

```sh
uv run --with mpmath python3 scripts/gen_sin_cos64_reference.py
uv run --with mpmath python3 scripts/gen_tan64_reference.py
uv run --with mpmath python3 scripts/gen_exp64_reference.py
uv run --with mpmath python3 scripts/gen_log64_reference.py
uv run --with mpmath python3 scripts/gen_powf64_reference.py
uv run --with mpmath python3 scripts/gen_atan64_reference.py
```

## no_std

既定の `std` feature を外すと `#![no_std]` になり、`alloc` も不要 このとき
`sqrt` はハードウェア命令と同じビットを返す、正しく丸めるソフトウェア平方根になる
CI はライブラリを `thumbv7em-none-eabihf` 向けに (`simd` の有無の両方で)
ビルドし、ソフトウェア経路で golden ハッシュを検査する

## 最低サポート Rust バージョン

Rust 1.85 (`Cargo.toml` の `rust-version`) CI で、既定の feature、`std,simd`、
`thumbv7em-none-eabihf` 向けの `no_std` rlib の 3 通りでライブラリを検査する
開発と CI は `rust-toolchain.toml` で固定したツールチェーンを使う

## ビルドとテスト

```sh
cargo test                                          # unit, accuracy, golden, doc tests
cargo test --features std,simd                      # + SIMD parity
cargo test --no-default-features --features simd    # software sqrt path
cargo clippy --all-targets --features std,simd -- -D warnings
python3 scripts/docs_lint.py --check                # public documents and CHANGELOG
scripts/preflight.sh                                # every CI step with the same arguments
scripts/preflight.sh --quick                        # without the test suites
```

## 関連プロジェクト

この crate を通して法則を評価し、2 台のマシンが結果をビット単位で一致させる
crate この crate は ALICE の中核の継ぎ目にあたる 符号付き距離場
([ALICE-SDF](https://github.com/ext-sakamoro/ALICE-SDF)) と、その中を動く物体
([ALICE-Physics](https://github.com/ext-sakamoro/ALICE-Physics)) は、同じ方法で
`sin` を計算して初めて曲面について一致する また法則の検証器
([ALICE-LOL](https://github.com/ext-sakamoro/ALICE-LOL)) は、そもそも再現できる
場についてしか何かを証明できない

> **解決後の依存グラフで版を 1 つに揃えること** 1 つの依存ツリーにこの crate の
> 版が 2 つあると、同じ関数の実装が 2 つあることになり、保証は失われる
> `cargo tree -i alice-det-math` で確認する

| プロジェクト | ビット一致の超越関数が必要な理由 | リンク |
|---------|----------------------------------------|-------|
| **ALICE-SDF** | 2 台で評価した符号付き距離場は、点が曲面のどちら側にあるかで一致しなければならない | [crates.io](https://crates.io/crates/alice-sdf) · [docs.rs](https://docs.rs/alice-sdf) · [GitHub](https://github.com/ext-sakamoro/ALICE-SDF) |
| **ALICE-Physics** | ロックステップ / ロールバックのシミュレーションは、`sin` が 1 ulp 違った時点でピア間で発散する | [crates.io](https://crates.io/crates/alice-physics) · [docs.rs](https://docs.rs/alice-physics) · [GitHub](https://github.com/ext-sakamoro/ALICE-Physics) |
| **ALICE-LOL** | SDF の法則検査は ALICE-SDF を経由し、ALICE-SDF はこの crate で評価する 次のリリースからは `research_law` モジュールも、多変数の研究法則 (`exp`、`ln`、`sqrt`、累乗、`sin`、`cos`) をここの `f64` 関数で評価し、別のマシンで再計算した法則が同じビットになる | [crates.io](https://crates.io/crates/alice-lol) · [docs.rs](https://docs.rs/alice-lol) · [GitHub](https://github.com/ext-sakamoro/ALICE-LOL) |

[ALICE-Zip](https://github.com/ext-sakamoro/ALICE-Zip) はこの crate に依存して
いない 生成器は現在 `libm` を使っている

## ライセンス

MIT OR Apache-2.0 係数とアルゴリズムは Cephes (Stephen L. Moshier)、fdlibm
(Sun Microsystems)、musl による 出典はソースを参照
