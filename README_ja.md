<table>
  <thead>
    <tr>
      <th style="text-align:center"><a href="README_ja.md">日本語</a></th>
      <th style="text-align:center"><a href="README.md">English</a></th>
    </tr>
  </thead>
</table>

# DeepFilterNet3 VST3

DeepFilterNet3 VST3 は、リアルタイムおよびオフラインのノイズリダクションのために公式 DeepFilterNet v0.5.6 モデルを組み込んだ macOS 用オーディオプラグインです。nice-plug を通じて VST3 および CLAP バンドルをエクスポートし、モノラルまたはステレオトラックに対応し、ニューラル推論とサンプルレート変換をホストのオーディオコールバックがノンブロッキングのまま維持できる永続的なワーカー上で処理します。

## プレビュー

<img src="githubreadme/screensho.png" alt="Attenuation Limit と Mix コントロールを備えた DeepFilter ノイズリダクションプラグインウィンドウ" width="480">

## オーディオデモ

プラグインのバイパス時と有効時：

- [エフェクトオフ — 元のシグナル (WAV)](githubreadme/effect-off.wav)
- [エフェクトオン — DeepFilter ノイズリダクション有効 (WAV)](githubreadme/effect-on.wav)

## 目次

- [機能](#features)
- [プレビュー](#preview)
- [オーディオデモ](#audio-demo)
- [技術スタック](#tech-stack)
- [現在の検証スコープ](#current-validation-scope)
- [オーディオ動作](#audio-behavior)
- [レイテンシー](#latency)
- [要件](#requirements)
- [ビルドとモデル選択](#build-and-model-selection)
- [インストール](#install)
- [使用方法](#usage)
- [パラメータ](#parameters)
- [開発とテスト](#development-and-testing)
- [プロジェクト構造](#project-structure)
- [トラブルシューティング](#troubleshooting)
- [既知の制限](#known-limitations)
- [ライセンス](#license)
- [クレジット](#credits)

## 機能

- デフォルトで公式 DeepFilterNet3 低レイテンシーモデルを使用し、公式標準モデルは別のビルド時オプションとして利用可能。
- 単一のモノラル推論ストリームによるモノラルおよびステレオの入出力レイアウト。
- 固定のタイムスタンプ付きワーカーチャンクによる任意のホストブロックサイズへの対応。
- 44.1、48、88.2、96、176.4、192 kHz のホストレートに対応したストリーミング変換。
- サンプルアライメントされたドライ/ウェットミキシングを伴うレイテンシーのレポート。
- リアルタイム、バッファー、オフラインモードで同一の DSP、リサンプラー、タイムライン、およびリセットプロトコル。
- ロックフリーのコールバックトランスポートと、ワーカー結果が遅延した場合のレイテンシーアライメントされたドライフォールバック。
- 安定したプラグインおよびパラメータ ID を持つ VST3 および CLAP エクスポート。

## 技術スタック

| コンポーネント | 役割 |
| :--- | :--- |
| Rust 2021 ワークスペース | プラグイン、DSP ブリッジ、テスト、バンドルタスク |
| [nice-plug 0.2.3](https://codeberg.org/RustAudio/nice-plug) | VST3/CLAP フレームワークとエクスポート |
| [DeepFilterNet 0.5.6](https://github.com/Rikorose/DeepFilterNet/tree/v0.5.6) | 公式組み込みモデルと Tract 推論 |
| [rubato 0.14.1](https://github.com/HEnquist/rubato/tree/v0.14.1) | 永続的な固定サイズのサンプルレート変換 |
| [rtrb 0.3.3](https://github.com/mgeier/rtrb/tree/v0.3.3) | ロックフリーのワーカーキュー |

## 現在の検証スコープ

現在の実装は Apple Silicon 搭載の macOS 26 でビルドおよびテストされています。自動検証には 24 件の Rust テストと、コールバックアロケーションアサーションを伴う pluginval 厳格度レベル 5 が含まれます。pluginval は 44.1、48、96 kHz の処理とオートメーションを実行し、`SUCCESS` で完了しました。

DaVinci Resolve 20 が想定ホストですが、最終的なインタラクティブ再生および Deliver のスモークテストはまだ完了していません。Windows、Linux、Intel macOS のビルドは検証されていません。

## オーディオ動作

組み込みモデルは常に 1 チャンネルを受け取ります：

- モノラル入力はそのまま推論に渡されます。
- ステレオ入力は `(左 + 右) / 2` としてダウンミックスされ推論に使用されます。
- モノラルのウェット結果は両方のステレオ出力にコピーされます。
- 各ステレオチャンネルは、アライメントされたドライ/ウェットミックスの前に独自のドライシグナルを保持します。

プラグインはドライおよびウェット出力の両方をレポートされたレイテンシーまで遅延させます。起動時またはリアルタイムワーカーのアンダーランが発生した場合、影響を受けたサンプルは無音または古いウェットフレームの代わりに、同じ遅延タイムスタンプのドライオーディオを使用します。オフラインモードは同じワーカーパイプラインを使用し、必要なタイムスタンプ付き結果を最大 2 秒間待機することがあります。

サポートされていないサンプルレートまたはホストバッファーのジオメトリ、モデル起動失敗、その他の初期化失敗が発生した場合、報告レイテンシーゼロの変更なし直接バイパスが選択されます。

## レイテンシー

レイテンシーは、ライブモデルのメタデータ、両方のリサンプラー、ノンブロッキング収集と推論のために予約された 2 つのホストクォンタから計算されます。公式低レイテンシーモデルは、48 kHz の FFT サイズ 960、ホップサイズ 480、ゼロルックアヘッド、480 サンプルの固有モデル遅延を報告します。

| ホストレート | ホストクォンタ | 報告レイテンシー | インパルス検証 |
| ---: | ---: | ---: | :--- |
| 44.1 kHz | 441 サンプル | 1,764 サンプル (40 ms) | 1 サンプル以内 |
| 48 kHz | 480 サンプル | 1,440 サンプル (30 ms) | 正確 |
| 96 kHz | 960 サンプル | 3,840 サンプル (40 ms) | 1 サンプル以内 |

0%、50%、100% のミックス値は、報告されたレイテンシーにおいてピークアライメントを維持します。他の宣言されたレートも同じ検証済みの計算式とストリーミングコンバーターのジオメトリを使用します。

## 要件

- 検証済み構成として macOS 26.x 以降を搭載した Apple Silicon Mac。
- nice-plug 0.2.3 をビルドするために Rust 1.87 以降。
- VST3 または CLAP 対応ホスト。

ビルド時に Rust 依存関係とピン留めされた公式 DeepFilterNet v0.5.6 のソース/モデルアーカイブがダウンロードされます。

## ビルドとモデル選択

リポジトリをクローンし、デフォルトの低レイテンシーモデルをビルドします：

```bash
git clone https://github.com/Shuichi346/DeepFilterNet3-VST3.git
cd DeepFilterNet3-VST3
cargo xtask bundle deepfilter-vst --release
```

生成されるバンドル：

```text
target/bundled/deepfilter-vst.vst3
target/bundled/deepfilter-vst.clap
```

デフォルトの低レイテンシーモデルの代わりに公式標準モデルをビルドするには：

```bash
cargo xtask bundle deepfilter-vst --release --no-default-features --features model-standard
```

モデルの機能は互いに排他的です。`model-ll` または `model-standard` のいずれか一方のみを有効にする必要があります。

## インストール

macOS でのユーザーのみの VST3 インストール：

```bash
mkdir -p "$HOME/Library/Audio/Plug-Ins/VST3"
cp -R target/bundled/deepfilter-vst.vst3 "$HOME/Library/Audio/Plug-Ins/VST3/"
```

CLAP ホスト向け：

```bash
mkdir -p "$HOME/Library/Audio/Plug-Ins/CLAP"
cp -R target/bundled/deepfilter-vst.clap "$HOME/Library/Audio/Plug-Ins/CLAP/"
```

インストール後にホストを再起動またはスキャンし直してください。ローカルビルドには Developer ID 署名または Apple 公証は付与されていません。

## 使用方法

1. VST3 または CLAP バンドルをビルドしてインストールし、ホストを再起動またはスキャンし直します。
2. モノラルまたはステレオのオーディオトラックに **DeepFilter Noise Reduction** を追加します。
3. 完全に強調されたシグナルには **Mix** を 100% のままにするか、レイテンシーアライメントされたドライチャンネルをブレンドするために減らします。
4. **Attenuation Limit** を調整してノイズ減衰量の上限を設定します。0 dB に設定するとモデルの状態を進めながらアライメントされた生のオーディオが選択されます。

ホストは初期化時にプラグインが計算したレイテンシーを受け取ります。要求されたホスト構成がサポートされていない場合、プラグインは引き続き使用可能ですが、オーディオをそのまま通過させ、レイテンシーをゼロと報告します。

## パラメータ

| パラメータ | 範囲 | デフォルト | 動作 |
| :--- | ---: | ---: | :--- |
| Attenuation Limit | 0〜100 dB | 100 dB | DeepFilterNet が適用する減衰量を制限します。実質的に 0 dB では、アライメントされた生のパスが選択されながらもモデルは引き続き動作します。 |
| Mix | 0〜100% | 100% | チャンネルごとのレイテンシーアライメントされたドライオーディオとモノラルのウェット結果をブレンドします。 |

## 開発とテスト

nice-plug のコールバックアロケーションアサーションを有効にしてデバッグバンドルをビルドします：

```bash
cargo xtask bundle deepfilter-vst --features nice-plug/assert_process_allocs
```

バウンドされたライブラリとプラグイン検証ゲートを実行します：

```bash
cargo test -p deepfilter-vst --lib && \
/Applications/pluginval.app/Contents/MacOS/pluginval \
  --strictness-level 5 \
  --validate-in-process \
  target/bundled/deepfilter-vst.vst3
```

pluginval に使用する VST3 バンドルは、前のコマンドで生成されたアロケーションアサーション付きのデバッグアーティファクトである必要があります。

リリースバンドルをビルドした後、Apple Silicon リリースパッケージを作成します：

```bash
cargo xtask bundle deepfilter-vst --release
./scripts/package-release.sh
```

このスクリプトは `plugin/Cargo.toml` からバージョンを読み取ります。明示的にバージョンを指定することもできます：

```bash
./scripts/package-release.sh 0.5.0
```

両方のバンドルが有効なアドホック署名を持つ thin arm64 バイナリであることを検証し、以下を作成します：

```text
dist/DeepFilterNR-v0.5.0-macos-arm64.zip
dist/DeepFilterNR-v0.5.0-macos-arm64.zip.sha256
```

ZIP には両方のプラグインバンドル、インストール手順、必要なライセンス通知、およびチェックサムが含まれています。既存のパッケージは上書きされません。スクリプトはインストールまたは公開を行いません。

## プロジェクト構造

```text
plugin/src/lib.rs        プラグインのメタデータ、ライフサイクル、ホストレイアウト、エクスポート
plugin/src/params.rs     Attenuation Limit と Mix パラメータ
plugin/src/bridge.rs     コールバック側のバッファリング、アライメント、フォールバック
plugin/src/dsp.rs        ワーカー DSP コアとレイテンシー計算
plugin/src/model.rs      DeepFilterNet モデルラッパーとメタデータ
plugin/src/resampler.rs  検証済みの永続的なサンプルレート変換
plugin/src/worker.rs     ワーカーのライフサイクル、キュー、リセット、ステータス
xtask/                   VST3/CLAP バンドルコマンド
scripts/                 リリースパッケージングツール
```

`PLANS.md` には実装と検証の根拠が記録されています。`CHANGELOG.md` と `NOTES.md` にはリリースとメンテナンス情報が記録されています。

## トラブルシューティング

ホストが古いローカルビルドを検出し続ける場合は、再インストール前にクリーンしてリリースバンドルを再作成してください：

```bash
cargo clean
cargo xtask bundle deepfilter-vst --release
```

VST3 または CLAP ディレクトリが上記のインストールパスと一致していることを確認し、ホストを再起動またはスキャンし直してください。ローカルでビルドしたバンドルは Developer ID 署名または公証が付与されていないため、macOS ホストのセキュリティ動作は配布された署名済みプラグインと異なる場合があります。

## 既知の制限

- ウェットパスは設計上モノラルです。ステレオの空間的な違いはドライの寄与にのみ残ります。
- リアルタイムスケジューリングの遅延により、強調された出力が欠落した場合に一時的にアライメントされたドライオーディオで代替されることがあります。
- ワーカー/モデルの起動は 10 秒に制限されており、起動失敗時は直接バイパスが選択されます。
- サポートされていないレートまたはバッファー構成は、おおよそのリサンプリングではなく直接バイパスを選択します。
- DaVinci Resolve の再生および Deliver の動作は、最終的な手動スモークチェックがまだ必要です。
- 上記の Apple Silicon macOS 構成のみが検証されています。

## ライセンス

[MIT ライセンス](LICENSE)。再配布されるコンポーネントに必要な通知は [サードパーティ通知](THIRD_PARTY_NOTICES.md) に記載されています。

## クレジット

- [DeepFilterNet](https://github.com/Rikorose/DeepFilterNet) — Hendrik Schröter および貢献者による。
- [nice-plug](https://codeberg.org/RustAudio/nice-plug) — RustAudio 貢献者による。
- [rubato](https://github.com/HEnquist/rubato) — ストリーミングサンプルレート変換のために。
