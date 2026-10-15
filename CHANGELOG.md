# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- CI: 公開する文面の検査の pin を ALICE-Law f9b9000 (checker sha256 84226ec7…) に進めた 新しい版で HEAD の全 file に指摘 0 件

## [0.5.1] - 2026-10-15

### Added

- `max` / `min` / `max64` / `min64` を追加 `f32::max` / `f32::min` (と `f64` 版) は等しい大きさの ±0 の組の符号を規定せず、target ごとに違う (AArch64 のハードウェア命令は既に +0/-0 を返す、IEEE 754-2019 の `maximumNumber`/`minimumNumber` と同じ、x86_64 の命令はどちらかの引数をそのまま返すため `0.0f32.max(-0.0)` はそこで `-0.0` になる) 他の ALICE-\* crate が内部に private で持っていた同じ比較だけの式をこの crate の公開 API として 1 箇所に集約した 両オペランドが NaN の時はこの crate の既存の canonical-NaN 方針 (`tests/nan_policy.rs`) どおり常に canonical NaN を返す (どちらかの payload をそのまま通さない) oracle: ±inf / ±1 / ±subnormal / ±0 / NaN の 9 値格子の全組 (両 width、324 件) を規則から計算した期待値と bit 一致で比較、加えて異なる payload を持つ NaN の全組合せ (f32 36 件 / f64 25 件) が canonical NaN を返すことを別途比較 変異試験: platform の `f32::max`/`min` がこの実装と x86_64 (Rosetta 経由の build で確認) では食い違うが aarch64 (ハードウェアが既に一致) では食い違わないことを確認、また canonical 化を外すと両方の変異で red になることを確認 `SEMANTICS_ID` が動く (公開数値関数の数が 48 から 52 に増えた、既存の関数の bit は変わっていない) 0.5.0 の識別子 (`135c407d…`) は `PREVIOUS_SEMANTICS_IDS` に残し、0.5.0 が保存した data は `check_semantics` で読める状態を維持した `specials` 集約 pin (関数の集合全体を畳み込む hash) は release ごとに関数が増えると現行の `specials` pin とはそもそも比較できなくなるため、0.5.0 の sweep を凍結した再計算 (`tests/golden.rs` の `specials_v0_5_0` module) を別途用意し、その再計算が 0.5.0 の保存済み hash に一致することを検査するよう `every_previous_identifier_is_backed_by_a_reproduced_pin_table` を拡張した (契約は `src/semantics.rs` に記載、以後の release が関数を追加するたびに同じ凍結を 1 つ足す形)

### Changed

- CI: 上限の無かった 18 job に `timeout-minutes` を付けた (止まった step は共有 runner を GitHub の上限の 6 時間まで占有しうる) 値は直近の成功した run 12 本の所要時間の最大の 2 倍・中央値の 4 倍の大きい方を 5 分単位に切り上げ、cargo を使う job は最低 15 分 (cache が外れた build のため)、他は最低 10 分 既に上限のある job は変えていない `scripts/workflow_timeouts.py` (試験 `scripts/test_workflow_timeouts.py`、ALICE-LOL と byte 一致) が全 job の上限を検査し、job を 1 つも読めなければ失敗する CI (actionlint job) と preflight で走る
- CI: fuzz と security-audit の paths の判定を `changes` job (`scripts/changed_paths.py`) に移した on.push の paths は新しい branch で既存の commit しか持たない時に比較の対象が空になり、`ci/**` の push で走らないことがあった 各 workflow に `Gate` job を置き、走るべき時に重い job が skip / cancel / 失敗なら red にする pattern は `.github/paths/<workflow>.txt` (pull_request の paths と同じ、`changed_paths.py sync` で突合)
- CI: `changed_paths.py sync` が、各 workflow の `Gate` job の needs に `changes` と他の全 job が入っているかを検査する (後から足した job が Gate の外で失敗・skip しても気付かない状態を防ぐ、Gate が 0 個・2 個以上・検査する job が無い場合も fail) `.github/paths/*.txt` の見出しを「結果を変えうる file を全て載せる」に改めた
- CI: `scripts/workflow_concurrency.py` と `scripts/test_workflow_concurrency.py` の冒頭に、ALICE-LOL / ALICE-SDF / ALICE-DetMath で同じ内容 (byte 一致) に保つ旨を書いた 検査の動作は変わらない
- CI: 公開する文面の検査 (`Public text` workflow、`scripts/public_text.sh`) を追加 ALICE-Law の `scripts/public_text_check.py` を `.github/public-text-pin` の revision で取得し、SHA-256 が一致した時だけ使って、HEAD の全 tracked file (path と本文) と送った commit (message・追加行・追加 path) を読む 何も比較できなければ失敗する preflight でも走る `.public-text.toml` は文書の linter とその試験が意図して持つ語を、種類ごとの行数で許可する (行数が変われば失敗)
- CI: 決定論ゲート (G10) を追加 `clippy.toml` の `disallowed-methods` (`Instant::now` / `SystemTime::now` / `RandomState::new`) に加えて `scripts/entropy_guard.py` が `src/**/*.rs` の production code (test module 外) を同じ語彙 (`getrandom` / `OsRng` / `thread_rng` / `rand::random` / `from_entropy` / `UNIX_EPOCH` / `Uuid::new_v4` を含む) で走査する 何も比較できなければ失敗する CI と preflight でも走る 現状は該当なし (0 problems)

### Fixed

- CI (`clippy.toml`、G10): `std::time::Instant::elapsed` / `std::time::SystemTime::elapsed` を disallowed-methods に追加 どちらも `::now()` と同じく wall clock を読むが、どちらの層も検査していなかった (`.elapsed(` の字面検査は `elapsed` という名の値全般を拾い過ぎるため、型解決する clippy 側だけに持たせた) `duration_since` はすでに保持した値どうしの引き算で clock を読まないため対象外 非 test code の `.elapsed()` 呼び出しは 0 件 canary: clippy.toml にこの 2 entry があることを確認する test
- CI (`scripts/entropy_guard.py`、G10): 2 つの穴を修正 (1) `RandomState::new` だけを検査しており `RandomState::default()` (同じ OS 乱数シードのハッシャーに届く) を素通りさせていた `RandomState` の裸の語を検査する形に変更 (clippy 側の `disallowed-methods` は `Default::default` をトレイトメソッドとして実装型だけでは指せないため、この script が唯一の検査層だったので `disallowed-types` で型そのものを ban する層も追加、`HashMap::new` 内部の無名使用は非該当のまま確認済) (2) `#[cfg(test)]` module 判定のブレース深さ計算が文字列しか除去しておらず、`'{'` のような char literal がブレース深さを狂わせて test module 判定が早期終了し、直後の production code を見逃す経路があった 文字列・raw string・char literal・ブロックコメントを 1 pass で正しく除去する sanitizer に置換 (3) `<std::time::Instant>::now()` のような qualified-self 呼び出しが `Instant::now` のパターンに `>` を挟むため素通りしていた (`as Trait` を挟む形も含む) 型名と `::method` の間に `>` / `as Trait` を許容するよう正規表現を拡張 canary test 6 本追加 (元の script に対して赤を確認) 比較件数は変わらず 8 file / 10 pattern / 0 problems

## [0.5.0] - 2026-10-11

**破壊的変更:** `SEMANTICS_ID` が 0.4.0 の
`d2209b30f6f1f45baa1b638bcdfee34ac64773b2e63b9c083b2e77afc691398e` から
`135c407d4249c1c200e4d1be79a12e848ce5c355fc3a918da965849e697ef000` に変わる 理由は 2 つ (1) 下に挙げる関数の出力 bit が変わる (scalar と `simd` の両方) (2) 識別子が覆う関数が 37 から 48 に増えた (`metric` module の 11 本、下の Fixed) `PREVIOUS_SEMANTICS_IDS` は空で、0.4.0 の識別子は `check_semantics` で `Mismatch` になる (0.4.0 で記録した残差・law の識別子・snapshot を、この版の算術で黙って再利用しない) 識別子を自分の識別子に混ぜている消費者は再記録が要る 開発中の main に一時記録した `a6fe7dc833d2a35012fc11eaf8e253eca6bfdb76db2931f056132f44bf79d6af` は公開していない

- `sin` / `cos` / `sin_cos`: 零点 `k·π/2` の近くの引数還元 (下の Fixed)
- `sin` / `sin_cos` / `tan`: `-0` に `-0` を返す (従来は `+0`)
- NaN の出力はすべて正準 NaN (`f32::NAN` = `0x7FC0_0000`、`f64::NAN` = `0x7FF8_0000_0000_0000`) NaN の引数を payload ごと返していた `round` / `round64` / `cbrt` / `metric::clamp` / `metric::smoothstep` / `metric::norm_l1` / `metric::norm_linf` / `MetricWeights::axis_extent` / `MetricWeights::euclidean_radius` (と `simd` の `round` / `clamp` / `smoothstep`) が変わる
- 多引数の関数は、どの位置の引数が NaN でも正準 NaN を返す (IEEE 754 の特例 `pow(+1, NaN) = 1`、`pow(NaN, ±0) = 1`、`hypot(±inf, NaN) = inf` を除く) 従来は `metric::norm_linf` が先頭 2 つの位置の NaN を落とし (`norm_linf([NaN, 1, 1]) = 1`)、`metric::clamp` が NaN の境界を、`metric::smoothstep` が NaN の端を無視していた (`simd` の `clamp` / `smoothstep` も同じ)
- `MetricWeights::lipschitz`: 重みが小さい (`w1 + winf < 2^-60`) か大きい (`> 2^60`) 時も閉形式どおり `MetricWeights::new` は Lipschitz 定数が正規数でない重みを `MetricError::Degenerate` で拒否する (従来は `(0, 0, 2^-149)` 等を受け付け、`normalised` が `0/0` を返した)
- `powf` / `powf64`: `x = 1` は `y` によらず `1` (IEEE 754、`y` が NaN・無限大・`f64::MAX` でも)

golden の pin は新設の `specials` を加えて 52 本 0.4.0 の pin table と比べて動くのは `sin` / `cos` / `sin_cos` / `round` / `round64` / `cbrt` / `smoothstep` / `metric_lipschitz` の 8 本で、残る 32 本は同じ bit (`tests/golden.rs` が検査する) `specials` は 0.4.0 の table に無い新しい pin で、旧 pin を動かさない

### ライセンス

- `MIT OR Apache-2.0` から **`Apache-2.0` のみ**に変えた (0.5.0 から) `LICENSE-MIT` を削除し、`LICENSE-APACHE` に加えて `NOTICE` (著作権表示と Cephes / fdlibm / musl の出典) と `TRADEMARK_NOTICE` (ALICE の名称の扱い、ライセンスとは独立) を置いた 0.4.0 までの公開版は `MIT OR Apache-2.0` のまま 依存する側が `alice-det-math` を permissive として扱う点は変わらない (Apache-2.0 は MIT / Apache / AGPL の crate から引ける) `scripts/license_check.py` が `Cargo.toml` の宣言・同梱 file・`cargo package --list`・README の記載の一致を検査する (CI と preflight、検査 0 件で失敗)

### Added

- `PREVIOUS_SEMANTICS_IDS` / `SemanticsCheck` / `check_semantics`: 保存したデータの識別子を照合する 結果は 3 値 (`Verified` / `VerifiedPrevious(id)` / `Mismatch`) で、過去の識別子を黙って同じとみなさない 用途ごとに 2 つの述語: `is_reproducible()` (保存した残差・law の識別子・snapshot を読む側) と `is_current()` (識別子を key として比べる側と書き手、書き手は常に `SEMANTICS_ID` を押す) 過去の識別子は、その版の pin table を `tests/golden.rs` に置き、table がその識別子に畳まれることと全 pin がこの版で再現されることを試験が示した時だけ足す (識別子の数と table の数の一致も検査) この版は上の関数の bit を変えたので、0.4.0 の識別子は入れない (空) `tests/golden.rs` は 0.4.0 の pin table をそのまま持ち、この版と違う pin がちょうど上の関数のものであることを示す module は浮動小数点の値を計算せず、float が現れると試験が落ちる 変異 5 種 (どちらかの table の hash の変更、table の無い識別子の追加、module 内の float、改名の取りこぼし) が red
- `consts::PIO2_4` と `consts::SPLIT_MASK` (JIT / shader などで同じ法則を出力する生成器向け) 3 片の還元を自前で出力している生成器は、この版の bit と一致しなくなる
- golden の pin `specials`: 全公開 scalar 関数 48 本を IEEE の特殊値 13 個 (`±0`、`±` 最小の非正規数、`±` 最小の正規数、`±1`、`±` 最大の有限値、`±inf`、NaN、2 引数は全対、3 引数は全 3 組) で評価した出力の hash 従来の格子は `+0` を通るが `-0` を通らず、他の特殊値には偶然しか当たらなかった 符号付きゼロ・非正規数・非有限値・NaN の bit がこの pin で識別子に入る 評価の列挙は `tests/common/specials.rs` で、公開関数の一覧と一致することを検査する (関数を足すとここにも足す必要がある) aarch64 と x86_64 で同じ hash (`2a9c0afc…`) を確認した
- `tests/nan_policy.rs`: 上の正準 NaN の規則を、全公開関数の特殊値・payload 付き NaN の引数・`simd` の各 lane・ゼロ重みの metric の `0·inf` で検査する
- `tests/nan_in_nan_out.rs`: 多引数の全公開関数 (scalar / `f64` / `simd`) で NaN を各位置に置き、他の引数を特殊値で振って正準 NaN を検査する
- `tests/signed_zero.rs` / `tests/pow_ieee_one.rs` / `tests/metric_degenerate.rs`: 上の各変更の検査 (どれも修正前の code で red を確認済)

### Changed

- The coverage check reads the `pub fn` / `pub const fn` definitions out of
  every source file the crate compiles, rather than the re-export lists, and
  asserts the surface size (48) and the pin count (52: one per function plus
  `specials`) exactly, so a parser
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
- CI: branch への push で走る workflow (ci / security-audit) の concurrency を fuzz と同じ形にした main では commit ごとに group を分けて run を取り消さず、取り消すのは branch の古い run だけ (以前は main への後の push が main の run や待機中の run を取り消し、結果が cancelled のまま残りえた) `scripts/workflow_concurrency.py` (試験 `scripts/test_workflow_concurrency.py`) が全 workflow をこの形か確かめ、CI と preflight で走る
- CI: fuzz の time-boxed run を gate にした (`continue-on-error` を外した) crash も、1 件も入力を実行しなかった run も red になり、target ごとの実行回数・秒数・結果を job summary に出す (`scripts/fuzz_runs.py`、試験 `scripts/test_fuzz_runs.py` を fuzz job で走らせる) crash の入力は artifact に残し、直したら `fuzz/seeds/<target>/` に置いて replay の gate に加える branch では同時 2 target まで、job に上限時間 45 分を付けた
- CI: branch (`ci/**`) への push でも 3 workflow (ci / security-audit / fuzz) を走らせる main に入れる前に同じ検査を branch で通すため
- CI: wasm32-wasip1 を wasmtime で実行する lane で、`nan_policy` / `nan_in_nan_out` / `signed_zero` の試験も走らせる (正準 NaN と符号付きゼロの規則を wasm でも検査する)
- CI: fuzz の workflow の token の権限を `contents: read` に絞った `scripts/fuzz_runs.py` は AddressSanitizer / LeakSanitizer の報告 (`==N==ERROR: AddressSanitizer` / `SUMMARY: AddressSanitizer: … leaked`) も crash として読む (以前は libFuzzer の行だけを見て、`Done N runs` の行があれば ok としていた) `scripts/workflow_concurrency.py` は、全 job が 1 台の self-hosted runner で走る workflow に限り、理由の comment がある時だけ ref ごとの group を認める (この repo の workflow には該当が無い)

### Fixed

- `sin` / `cos` / `sin_cos` が零点 `k·π/2` の近くで誤差 2 ulp の保証を外れていた 文書化した定義域 `|x| ≤ 100` で最大 14 ulp (`cos(-4.712389)`、`sin(-75.398224)`)、`|x| ≤ 8192` では `sin` 478 ulp / `cos` 974 ulp (scalar と `simd` は同じ bit) 原因は引数還元 `x − k·π/2` の 3 片目 `k·PIO2_3` の丸め誤差と、π/2 を 3 片で打ち切った残り (零点の近くでは結果 `±sin(r)` の相対誤差がそのまま還元の相対誤差になる) 多項式は無関係 (正確な `r` を与えると 0.22 ulp)
  - `k·PIO2_3` を Dekker の two-product で正確に作り (fused multiply-add は使わない、分割は bit mask で overflow しない)、4 片目 `PIO2_4 = π/2 − (PIO2_1 + PIO2_2 + PIO2_3)` を引く scalar と `simd` は同じ演算を同じ順で行い bit 一致
  - 変わる出力は 9,000,000 入力 (`[-100, 100]` 4M / `[-1, 1]` 1M / `[-8192, 8192]` 4M の等間隔) のうち 672 (0.007%) この修正で動く golden の pin は `sin` / `cos` / `sin_cos` の 3 つだけ
  - `tests/accuracy_trig_zeros.rs`: `|k| ≤ 63` (`|x| ≤ 100`) と `|k| ≤ 5215` (`|x| ≤ 8192`) の全零点の前後 64 ulp を scalar / `sin_cos` / `simd` で 2 ulp 以内と検査 (修正前の code で red を確認済) fuzz の seed 3 件 (`fuzz/seeds/fuzz_accuracy/`) も修正前の code で 14 / 6 / 14 ulp を出す
  - 定義域の表記は `|x| ≤ 100` のまま `|x| ≤ 8192` までは実測で最大 2 ulp
- `sin` / `sin_cos` / `tan` が `-0` に `+0` を返していた Cephes の多項式 `r + p` で `p` の符号が `-(+0)(-0) = +0` になり、`-0 + +0 = +0` になっていた fdlibm / musl と同じく `±0` は引数をそのまま返す (0 以外の入力の bit は変わらない) fuzz_accuracy は奇関数の `±0` の符号を検査する (ulp の比較は `-0 == +0` を区別できない) seed `fuzz/seeds/fuzz_accuracy/sin_neg_zero`
- `MetricWeights::lipschitz` の根号 `(w1 + winf)^2 + 2 w1^2` が `w1 + winf < 2^-63` で非正規数化・`0` になり、`2^64` 超で overflow していた 閉形式と比べて 2 ulp を超えて外れる入力は格子 300,003 点で 92,673 点 範囲 `[2^-60, 2^60]` の外では重みを `2^±100` 倍して評価し戻す (2 の冪の拡縮は積と `sqrt` と交換し正確) 範囲内は従来の式のまま bit 一致 (格子・乱数 200 万・両端の前後 64 ulp で検査) 精度は範囲内と同じ 2 ulp
- `MetricWeights::normalised` / `powf64(1, ±f64::MAX)` が算術の NaN (符号が aarch64 と x86_64 で異なる) を返していた 上の 2 つの変更で、どちらも target によらない bit になる 新設の `specials` pin を両 target で計算して見つかった
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
- crate の doc の精度表で、code span の中の `|x|` / `|y|` が表の区切りとして読まれ、nightly の rustdoc (docs.rs が使う) では定義域の列が捨てられていた 表の行の code span 内の `|` を `\|` に直した (stable の rustdoc は警告しないので CI の doc job では見えなかった)
- `SEMANTICS_ID` の doc の text block (docs.rs に出る値) が、公開していない途中の値 `a6fe7dc…` のままで定数 `135c407d…` と違っていた 値を直し、`tests/semantics_id_doc.rs` が doc block の値と定数の一致を検査する (修正前の source で red を確認済)

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

[0.5.1]: https://github.com/ext-sakamoro/ALICE-DetMath/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/ext-sakamoro/ALICE-DetMath/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/ext-sakamoro/ALICE-DetMath/compare/v0.3.2...v0.4.0
[0.3.2]: https://github.com/ext-sakamoro/ALICE-DetMath/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/ext-sakamoro/ALICE-DetMath/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/ext-sakamoro/ALICE-DetMath/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/ext-sakamoro/ALICE-DetMath/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/ext-sakamoro/ALICE-DetMath/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/ext-sakamoro/ALICE-DetMath/releases/tag/v0.1.0
