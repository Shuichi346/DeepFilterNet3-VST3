<table>
  <thead>
    <tr>
      <th style="text-align:center"><a href="README_ja.md">日本語</a></th>
      <th style="text-align:center"><a href="README.md">English</a></th>
    </tr>
  </thead>
</table>

# DeepFilterNet3 VST3

DeepFilterNet3 VST3 は、公式 DeepFilterNet v0.5.6 モデルを組み込んだリアルタイムおよびオフラインのノイズ除去機能を持つ macOS 向けオーディオプラグインです。nice-plug を通じて VST3 および CLAP バンドルをエクスポートし、モノラルおよびステレオトラックに対応しています。ニューラル推論とサンプルレート変換は永続的なワーカースレッド上で処理されるため、ホストのオーディオコールバックはノンブロッキングのまま維持されます。

## プレビュー

<img src="githubreadme/screensho.png" alt="Attenuation Limit と Mix コントロールを備えた DeepFilter ノイズ除去カスタムエディター" width="480">

コンパクトなカスタムエディターは、単位区切りの数値入力による **Attenuation Limit** および **Mix** コントロールを提供します。その他のコントロールやビジュアライゼーションは含まれていません。

## オーディオデモ

プラグインのバイパス時と有効時の比較：

- [エフェクトオフ — 元の信号 (WAV)](githubreadme/effect-off.wav)
- [エフェクトオン — DeepFilter ノイズ除去を有効化 (WAV)](githubreadme/effect-on.wav)

## 目次

- [機能](#features)
- [プレビュー](#preview)
- [オーディオデモ](#audio-demo)
- [技術スタック](#tech-stack)
- [現在の検証スコープ](#current-validation-scope)
- [オーディオの動作](#audio-behavior)
- [レイテンシ](#latency)
- [必要要件](#requirements)
- [ビルドとモデル選択](#build-and-model-selection)
- [インストール](#install)
- [使い方](#usage)
- [パラメーター](#parameters)
- [開発とテスト](#development-and-testing)
- [プロジェクト構成](#project-structure)
- [トラブルシューティング](#troubleshooting)
- [既知の制限事項](#known-limitations)
- [ライセンス](#license)
- [クレジット](#credits)

## 機能

- デフォルトでは公式 DeepFilterNet3 低レイテンシモデルを採用。公式標準モデルはビルド時のオプションとして別途利用可能。
- モノラルおよびステレオ入出力レイアウトに対応し、モノラル推論ストリームを1つ使用。
- 固定・タイムスタンプ付きワーカーチャンクによる任意のホストブロックサイズへの対応。
- 44.1、48、88.2、96、176.4、192 kHz のホストレートに対するストリーミング変換。
- サンプル同期されたドライ/ウェットミキシングによるレイテンシ通知。
- リアルタイム・バッファード・オフラインモードで共通の DSP、リサンプラー、タイムライン、リセットプロトコルを使用。
- ロックフリーのコールバックトランスポートと、ワーカー結果が遅延した際のレイテンシ同期ドライフォールバック。
- Attenuation Limit および Mix スライダーのみを持つコンパクトな英語カスタムエディター。
- 安定したプラグイン ID およびパラメーター ID による VST3 および CLAP エクスポート。

## 技術スタック

| コンポーネント | 役割 |
| :--- | :--- |
| Rust 2021 ワークスペース | プラグイン、DSP ブリッジ、テスト、バンドルタスク |
| [nice-plug 0.2.3](https://codeberg.org/RustAudio/nice-plug) | VST3/CLAP フレームワークおよびエクスポート |
| [nice-plug-egui 0.3.0](https://codeberg.org/RustAudio/nice-plug/src/branch/main/crates/nice-plug-egui) / [egui 0.35.0](https://github.com/emilk/egui/tree/0.35.0) | 2スライダー埋め込みカスタムエディター |
| [DeepFilterNet 0.5.6](https://github.com/Rikorose/DeepFilterNet/tree/v0.5.6) | 公式組み込みモデルおよび Tract 推論 |
| [rubato 0.14.1](https://github.com/HEnquist/rubato/tree/v0.14.1) | 固定サイズ永続サンプルレート変換 |
| [rtrb 0.3.3](https://github.com/mgeier/rtrb/tree/v0.3.3) | ロックフリーワーカーキュー |

## 現在の検証スコープ

現在の実装は Apple Silicon 搭載の macOS 26 でビルドおよびテストされています。自動検証には 25 件の Rust テストと、コールバックアロケーションアサーションを有効にした pluginval 厳格度レベル 5 が含まれます。pluginval は、アイドル時および処理中の両方でカスタムエディターを開き、エディターオートメーションおよび 44.1、48、96 kHz の処理を実行し、`SUCCESS` で完了しました。

DaVinci Resolve 21 でのユーザー確認テストにて、以前のバンドルによるデリバーエクスポートが正常に完了しています。ただし、そのバンドルはカスタムエディター実装前のものであるため、現在のビルドの UI 検証とはなっていません。より広範な再現性、インタラクション、レイテンシ、マルチレートの Resolve スモークテストマトリックスはまだ完了していません。Windows、Linux、および Intel macOS ビルドの検証は行われていません。

## オーディオの動作

組み込みモデルは常に1チャンネルを受け取ります：

- モノラル入力は直接推論に渡されます。
- ステレオ入力は `(左 + 右) / 2` としてダウンミックスされ、推論に渡されます。
- モノラルのウェット結果は両方のステレオ出力にコピーされます。
- 各ステレオチャンネルは、同期されたドライ/ウェットミックス前に独自のドライ信号を保持します。

プラグインはドライおよびウェット出力の両方を通知されたレイテンシ分だけ遅延させます。起動時またはリアルタイムワーカーのアンダーランが発生した場合、影響を受けたサンプルは無音や古いウェットフレームの代わりに、同じ遅延タイムスタンプのドライオーディオを使用します。オフラインモードも同じワーカーパイプラインを使用し、必要なタイムスタンプ付き結果を待つために最大2秒間待機することがあります。

サポートされていないサンプルレートやホストバッファーのジオメトリ、モデル起動失敗、その他の初期化失敗時は、報告レイテンシをゼロにした変更なしのダイレクトバイパスを選択します。

## レイテンシ

レイテンシはライブモデルメタデータ、両リサンプラー、およびノンブロッキングの収集と推論のために予約された2つのホスト量子から計算されます。公式低レイテンシモデルは、48 kHz FFT サイズ 960、ホップサイズ 480、ルックアヘッドなし、480 サンプルの固有モデル遅延を報告します。

| ホストレート | ホスト量子 | 報告レイテンシ | インパルス検証 |
| ---: | ---: | ---: | :--- |
| 44.1 kHz | 441 サンプル | 1,764 サンプル (40 ms) | 1 サンプル以内 |
| 48 kHz | 480 サンプル | 1,440 サンプル (30 ms) | 正確 |
| 96 kHz | 960 サンプル | 3,840 サンプル (40 ms) | 1 サンプル以内 |

Mix 値 0%、50%、100% は、報告されたレイテンシにてピーク同期を維持します。その他の宣言されたレートも同じ検証済み式とストリーミングコンバーターのジオメトリを使用します。

## 必要要件

- 検証済み構成として、macOS 26.x 以降を搭載した Apple Silicon Mac。
- nice-plug 0.2.3 のビルドに Rust 1.87 以降。
- VST3 または CLAP 対応のホスト。

ビルド時に Rust 依存関係および固定バージョンの公式 DeepFilterNet v0.5.6 ソース/モデルアーカイブがダウンロードされます。

## ビルドとモデル選択

リポジトリをクローンし、デフォルトの低レイテンシモデルでビルドします：

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

デフォルトの低レイテンシモデルの代わりに公式標準モデルをビルドする場合：

```bash
cargo xtask bundle deepfilter-vst --release --no-default-features --features model-standard
```

モデルのフィーチャーは相互に排他的です。`model-ll` または `model-standard` のいずれか一方のみを有効にする必要があります。

## インストール

macOS でのユーザー専用 VST3 インストール：

```bash
mkdir -p "$HOME/Library/Audio/Plug-Ins/VST3"
cp -R target/bundled/deepfilter-vst.vst3 "$HOME/Library/Audio/Plug-Ins/VST3/"
```

CLAP ホストの場合：

```bash
mkdir -p "$HOME/Library/Audio/Plug-Ins/CLAP"
cp -R target/bundled/deepfilter-vst.clap "$HOME/Library/Audio/Plug-Ins/CLAP/"
```

インストール後、ホストを再起動またはプラグインを再スキャンしてください。ローカルビルドには Developer ID 署名や Apple 公証は付与されていません。

## 使い方

1. VST3 または CLAP バンドルをビルドしてインストールし、ホストを再起動または再スキャンします。
2. モノラルまたはステレオのオーディオトラックに **DeepFilter Noise Reduction** を追加します。
3. プラグインエディターを開きます。**Attenuation Limit** と **Mix** スライダーのみ含まれています。
4. 完全にエンハンスされた信号を得るには **Mix** を 100% のままにするか、レイテンシ同期されたドライチャンネルをブレンドするために値を下げます。
5. **Attenuation Limit** を調整してノイズ減衰量の上限を設定します。0 dB に設定すると、モデルの状態を進めつつ同期されたそのままのオーディオが選択されます。

ホストは初期化時にプラグインが計算したレイテンシを受け取ります。要求されたホスト構成がサポートされていない場合、プラグインは引き続き利用可能ですが、オーディオをそのまま通過させ、ゼロレイテンシを報告します。

コンパクトなカスタムエディターとホストが生成するパラメーターパネルは、どちらも同じ2つのオートメーション可能なパラメーターを制御します。ホストオートメーションおよび外部パラメーター変更はスライダーと同期を保ちます。

## パラメーター

| パラメーター | 範囲 | デフォルト | 動作 |
| :--- | ---: | ---: | :--- |
| Attenuation Limit | 0〜100 dB | 100 dB | DeepFilterNet が適用する減衰量を制限します。実質的に 0 dB に設定すると、モデルは動作し続けながら同期されたそのままのパスが選択されます。 |
| Mix | 0〜100% | 100% | レイテンシ同期されたチャンネルごとのドライオーディオとモノラルのウェット結果をブレンドします。 |

## 開発とテスト

nice-plug のコールバックアロケーションアサーションを有効にしてデバッグバンドルをビルドします：

```bash
cargo xtask bundle deepfilter-vst --features nice-plug/assert_process_allocs
```

ライブラリおよびプラグインの検証ゲートを実行します：

```bash
cargo test -p deepfilter-vst --lib && \
/Applications/pluginval.app/Contents/MacOS/pluginval \
  --strictness-level 5 \
  --validate-in-process \
  target/bundled/deepfilter-vst.vst3
```

pluginval に使用する VST3 バンドルは、前のコマンドで生成したアロケーションアサーション付きデバッグ成果物を使用してください。

リリースバンドルのビルド後に Apple Silicon 向けリリースパッケージを作成します：

```bash
cargo xtask bundle deepfilter-vst --release
./scripts/package-release.sh
```

スクリプトは `plugin/Cargo.toml` からバージョンを読み取ります。明示的なバージョンを渡すこともできます：

```bash
./scripts/package-release.sh 0.5.0
```

スクリプトは両方のバンドルが有効なアドホック署名を持つ thin arm64 バイナリであることを確認し、以下を生成します：

```text
dist/DeepFilterNR-v0.5.0-macos-arm64.zip
dist/DeepFilterNR-v0.5.0-macos-arm64.zip.sha256
```

ZIP には両方のプラグインバンドル、インストール手順、必要なライセンス通知、およびチェックサムが含まれます。既存のパッケージは上書きされません。スクリプトはインストールや公開は行いません。

## プロジェクト構成

```text
plugin/src/lib.rs        プラグインメタデータ、ライフサイクル、ホストレイアウト、エクスポート
plugin/src/params.rs     Attenuation Limit および Mix パラメーター
plugin/src/editor.rs     固定サイズの英語2スライダーカスタムエディター
plugin/src/bridge.rs     コールバック側のバッファリング、アライメント、フォールバック
plugin/src/dsp.rs        ワーカー DSP コアおよびレイテンシ計算
plugin/src/model.rs      DeepFilterNet モデルラッパーおよびメタデータ
plugin/src/resampler.rs  検証済み永続サンプルレート変換
plugin/src/worker.rs     ワーカーライフサイクル、キュー、リセット、ステータス
xtask/                   VST3/CLAP バンドルコマンド
scripts/                 リリースパッケージングツール
```

`PLANS.md` には実装と検証の証跡が記録されています。`CHANGELOG.md` および `NOTES.md` にはリリースおよびメンテナンス情報が記録されています。

## トラブルシューティング

ホストが古いローカルビルドを検出し続ける場合は、再インストール前にクリーンしてリリースバンドルを再作成してください：

```bash
cargo clean
cargo xtask bundle deepfilter-vst --release
```

VST3 または CLAP ディレクトリが上記のインストールパスと一致していることを確認し、ホストを再起動または再スキャンしてください。ローカルビルドのバンドルは Developer ID 署名や公証が付与されていないため、macOS のホストセキュリティの挙動が署名済みの配布プラグインと異なる場合があります。

## 既知の制限事項

- ウェットパスは設計上モノラルです。ステレオの空間的な差異はドライ成分にのみ残ります。
- リアルタイムスケジューリングの遅延により、エンハンスされた出力が欠落した際に一時的に同期されたドライオーディオに切り替わることがあります。
- ワーカー/モデルの起動は最大10秒に制限されており、起動に失敗した場合はダイレクトバイパスが選択されます。
- サポートされていないレートまたはバッファー構成では、近似リサンプリングではなくダイレクトバイパスが選択されます。
- DaVinci Resolve 21 でのデリバーエクスポートの成功はユーザーにより確認されていますが、完全な再現性、インタラクション、レイテンシ、マルチレートのホストマトリックスは未検証です。
- 上記で説明した Apple Silicon macOS 構成のみが検証されています。

## ライセンス

[MIT ライセンス](LICENSE)。再配布コンポーネントの必要な通知は [サードパーティ通知](THIRD_PARTY_NOTICES.md) に記載されています。

## クレジット

- [DeepFilterNet](https://github.com/Rikorose/DeepFilterNet) — Hendrik Schröter および貢献者による。
- [nice-plug](https://codeberg.org/RustAudio/nice-plug) — RustAudio 貢献者による。
- [rubato](https://github.com/HEnquist/rubato) — ストリーミングサンプルレート変換のために使用。
