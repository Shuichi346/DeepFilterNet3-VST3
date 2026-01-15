# DeepFilter VST3 Plugin

[DeepFilterNet3](https://github.com/Rikorose/DeepFilterNet) を使用したリアルタイムノイズ除去VST3プラグインです。

## 特徴

- **AIベースのノイズ除去**: DeepFilterNet3ニューラルネットワークによる高品質なノイズ抑制
- **設定不要**: 適用するだけで自動的にノイズを除去
- **リアルタイム処理**: 48kHzでのリアルタイム処理に対応

## 動作要件

- **サンプルレート**: 48kHz（必須）
- **対応OS**: macOS (Apple Silicon / Intel), Windows, Linux
- **対応DAW**: DaVinci Resolve, Logic Pro, Ableton Live, Reaper, Cubase など VST3対応DAW

## インストール

### ビルド済みプラグイン

[Releases](https://github.com/YOUR_USERNAME/deepfilter-vst/releases) からダウンロードしてください。

#### macOS

**システム全体にインストール:**
```bash
sudo cp -r deepfilter-vst.vst3 /Library/Audio/Plug-Ins/VST3/
```

**または ユーザー専用:**
```bash
cp -r deepfilter-vst.vst3 ~/Library/Audio/Plug-Ins/VST3/
```

#### Windows

`deepfilter-vst.vst3` フォルダを以下のパスにコピーしてください。
```text
C:\Program Files\Common Files\VST3\
```

#### Linux

```bash
cp -r deepfilter-vst.vst3 ~/.vst3/
```

---

### ソースからビルド

**必要なもの:**
- Rust (1.70以上)

**ビルド手順:**

```bash
git clone https://github.com/YOUR_USERNAME/deepfilter-vst.git
cd deepfilter-vst
cargo xtask bundle deepfilter-vst --release
```

**ビルド成果物:** `target/bundled/deepfilter-vst.vst3`

## 使用方法

1. プラグインをインストールします。
2. DAWのプロジェクト設定（サンプルレート）を **48kHz** に設定します。
3. オーディオトラックに「DeepFilter Noise Reduction」を適用します。
4. 完了！（パラメータ調整は通常不要です）

## パラメータ

| パラメータ | 説明 | デフォルト |
| :--- | :--- | :--- |
| **Attenuation Limit** | ノイズ抑制量 (dB) | 100 |
| **Mix** | Dry/Wet比率 | 100% |

## ライセンス

MIT License

## クレジット

- [DeepFilterNet](https://github.com/Rikorose/DeepFilterNet) - Hendrik Schröter
- [nih-plug](https://github.com/robbert-vdh/nih-plug) - Robbert van der Helm