# CUDAバックエンド対応

## 概要

WebGPUIは2つのGPUレンダリングバックエンドに対応しています:

1. **wgpu** (デフォルト): クロスプラットフォーム対応、任意のモダンGPU上で動作
2. **CUDA** (オプション): NVIDIAに特化、微細なGPU制御による性能最適化が可能

このガイドではCUDAバックエンドの設定、設定方法、使用方法について説明します。

## なぜCUDAか?

### 利点
- **微細なGPU制御**: NVIDIA CUDA APIへのアクセスにより、カスタムカーネルコンパイルが可能
- **性能最適化の余地**: 標準グラフィクスパイプライン以上のワークロード最適化が可能
- **ハードウェア特化**: NVIDIA GPUアーキテクチャに合わせた最適化が可能
- **計算能力**: 物理演算やポストプロセッシングコンピュートシェーダーなど、将来の機能が可能

### 制限事項
- **ハードウェアロックイン**: NVIDIAの GPU が必須（Maxwell世代以降）
- **プラットフォーム限定**: Linux (x86_64) と Windows (x86_64) のみ対応
- **ツールキット依存**: ビルド時に CUDA Toolkit 11.8+ が必要
- **開発速度**: CUDA Toolkit のインストール必須; wgpuはより高速に開発可能

### CUDAを使うべき場合
- NVIDIAハードウェアを保有し、そのハードウェアで最高性能が必要
- 高度な計算機能が必要（物理演算、ポスト エフェクト）
- NVIDIA GPUを搭載したサーバーへの本番環境デプロイ対象
- CUDA と wgpu のハードウェアベンチマーク比較が必要

### wgpu (デフォルト) を使うべき場合
- macOS での開発（CUDAは利用不可）
- クロスプラットフォームデプロイ （AMD、Intel、Apple ハードウェア）
- 高速プロトタイピング（ツールキットインストール不要）
- ハードウェア特化より、可搬性が重要

## ハードウェア要件

### GPU対応
- **CUDA Compute Capability 3.5以上のNVIDIA GPU**
  - Maxwell世代（GTX 750 Ti、GTX 960、Quadro M）以降
  - すべてのモダンNVIDIAデータセンターGPU（V100、A100、H100等）

### CUDAツールキット バージョン
- **最小: CUDA Toolkit 11.8**
- **推奨: CUDA Toolkit 12.0以降**

### 対応プラットフォーム
- **Linux (x86_64)**: プライマリサポートプラットフォーム
  - NVIDIA CUDA Toolkit for Linux
  - CUDAバージョンに対応したNVIDIA GPUドライバ
- **Windows (x86_64)**: CUDA for Windows経由
  - NVIDIA CUDA Toolkit for Windows
  - Visual Studio 2019/2022 with CUDA support
- **macOS**: 非対応 (NVIDIA CUDA はApple Silicon では利用不可)

### ドライバ互換性
- GPU ドライバは、インストールされた CUDA Toolkit バージョンと互換性が必要
- ドライババージョンマッピングは [NVIDIA CUDA Compatibility](https://docs.nvidia.com/cuda/cuda-toolkit-release-notes/index.html) を確認

## インストール・セットアップ

### 1. CUDA Toolkit のインストール

#### Linux (Ubuntu/Debian)
```bash
# https://developer.nvidia.com/cuda-downloads から CUDA 12.0 をダウンロード
# または apt を使用 (Ubuntu 22.04 の例):
wget https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2204/x86_64/cuda-keyring_1.1-1_all.deb
sudo dpkg -i cuda-keyring_*.deb
sudo apt-get update
sudo apt-get install cuda-toolkit-12-0
```

#### Windows
1. [NVIDIA Developer Site](https://developer.nvidia.com/cuda-downloads) から CUDA Toolkit をダウンロード
2. インストーラを実行、カスタムインストールを選択
3. Visual Studio 統合が選択されていることを確認
4. 環境変数に CUDA パスを追加:
   ```
   CUDA_PATH=C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA\v12.0
   PATH=%CUDA_PATH%\bin;%PATH%
   ```

### 2. CUDA インストールの確認
```bash
# NVIDIA GPUドライバを確認
nvidia-smi

# CUDA Toolkitを確認
nvcc --version
```

成功時の出力例:
```
$ nvidia-smi
+-----------------------------------------------------------------------------+
| NVIDIA-SMI 545.29.06    Driver Version: 545.29.06    CUDA Version: 12.0     |
+-----------------------------------------------------------------------------+
```

### 3. プロジェクトで CUDA フィーチャーを有効化

`Cargo.toml` を更新:
```toml
[dependencies]
webgpui = { path = ".", features = ["backend-cuda"] }
```

またはコマンドラインから:
```bash
cargo build --features backend-cuda
cargo run --features backend-cuda
```

## 設定

### バックエンド利用可能性検出

`BackendSelector` enum（`webgpui-render` 内）はランタイムで利用可能なバックエンドを検出します:

```rust
use webgpui_render::BackendSelector;

// コンパイルされたバックエンドを確認
let available_backends = BackendSelector::available();
for backend in &available_backends {
    println!("利用可能: {}", backend.name());
}

// 特定のバックエンドが利用可能かどうかを確認
if BackendSelector::Cuda.is_available() {
    println!("CUDA が利用可能!");
} else {
    println!("CUDA がコンパイルされていないか利用不可");
}
```

### フィーチャーフラグ

#### コンパイル時の選択
WebGPUI ビルドシステムは、フィーチャーフラグを使用してコンパイル時に GPU バックエンドを選択します:

```toml
# オプション1: wgpuを使用 (デフォルト)
[dependencies]
webgpui = { version = "0.1", features = ["backend-wgpu"] }

# オプション2: CUDAを使用
[dependencies]
webgpui = { version = "0.1", features = ["backend-cuda"] }

# オプション3: 両方を含める (1つのバイナリでは1つのみ有効)
[dependencies]
webgpui = { version = "0.1", features = ["backend-wgpu", "backend-cuda"] }
```

#### フィーチャー要件
- バイナリクレートでは、一度に1つのバックエンドのみを有効にする必要があります
- ライブラリは両方のバックエンドを含めることができます。バイナリクレートが使用するものを選択します
- バックエンド間の自動フォールバックはありません（ビルド設定で明示的に選択が必要）

### ランタイム設定

コンパイル時にフィーチャーを使用してバックエンドを選択した後、アプリは（両方がリンクされている場合）、ランタイムバックエンド選択を公開できます:

```rust
// webgpui-render 内
use webgpui_render::BackendSelector;

// 利用可能なバックエンドを確認
let available = BackendSelector::available();
println!("利用可能なバックエンド: {:?}", available);

// バックエンド選択
let backend = if has_nvidia_gpu { 
    BackendSelector::Cuda 
} else { 
    BackendSelector::Wgpu 
};

// バックエンドが利用可能か確認
if !backend.is_available() {
    eprintln!("選択されたバックエンド {} はコンパイルされていません", backend.name());
}

// webgpui-app 内 (両方のバックエンドがリンクされている場合)
pub enum BackendMode {
    Wgpu,
    Cuda,
}

pub fn init_renderer(mode: BackendMode) -> Result<Box<dyn Renderer>> {
    match mode {
        #[cfg(feature = "backend-wgpu")]
        BackendMode::Wgpu => Ok(Box::new(WgpuRenderer::new()?)),
        
        #[cfg(feature = "backend-cuda")]
        BackendMode::Cuda => Ok(Box::new(CudaRenderer::new()?)),
        
        _ => Err("Backend not compiled in"),
    }
}
```

## パフォーマンス期待値

### ベースラインメトリクス

適切に設定された場合、CUDA と wgpu は同じパフォーマンス目標を達成すべきです:

- **フレーム時間**: 平均 ≤ 16.6ms (60 FPS)、P95 ≤ 20ms
- **ドローコール**: バッチング P1 後、代表的な画面上で ≤ 200
- **メモリ**: フレーム当たりのヒープアロケーション最小化

### CUDA 固有の考慮事項

1. **初回フレーム遅延**: CUDA カーネルコンパイルが初回実行時にスタートアップオーバーヘッドを追加する可能性あり
   - 緩和策: `cudarc` カーネルキャッシングまたは JIT コンパイル戦略を使用
   - アプリスタートアップ時のプリウォームを推奨

2. **GPU同期**: CUDA は明示的な同期が必要。CPU をブロックしないよう `cuStreamSynchronize()` の使用に注意

3. **メモリ管理**: CUDA メモリはシステムメモリとは別。テクスチャとバッファの GPU メモリ予算を計画

### 等価性テスト

CUDA と wgpu の出力は、同じ入力シーンに対して**ピクセル単位で一致する**必要があります:
- ビジュアルスナップショットは byte-exact マッチングで比較
- イベントシーケンスは同一である必要があります
- パフォーマンスは個別に測定（ワークロードによって CUDA の方が速い場合と遅い場合がある）

詳細は [API Swapping Quality Plan](../rendering/api-swapping-quality-plan.md) の等価性テストを参照してください。

## トラブルシューティング

### 問題: "CUDA support disabled; enable 'cuda' feature"
**解決策**: `Cargo.toml` に `features = ["backend-cuda"]` を追加

### 問題: ビルド時に CUDA Toolkit が見つからない
**解決策**:
- Linux: `nvcc` が `PATH` にあることを確認:
  ```bash
  export PATH=/usr/local/cuda/bin:$PATH
  ```
- Windows: `CUDA_PATH` 環境変数が設定されていることと、パスが正しいことを確認

### 問題: NVIDIA GPU が検出されない
**解決策**:
- NVIDIA ドライバを確認: `nvidia-smi`
- GPU が最小 Compute Capability 3.5 を満たしていることを確認
- GPU ドライバを更新: `nvidia-driver-update` (Linux) または Windows デバイスマネージャ

### 問題: "cudarc" クレートビルドが失敗
**解決策**:
- CUDA Toolkit を 11.8+ にアップグレード
- `libcuda.so` (Linux) または `nvcuda.dll` (Windows) がアクセス可能なことを確認
- cudarc バージョンが CUDA Toolkit と一致すること（cudarc 0.12 → CUDA 11.8+）

### 問題: CUDA のパフォーマンスが wgpu より悪い
**考えられる原因**:
- コンピュート軽量 GUI ワークロード向けの異なる GPU 利用パターン
- CPU と GPU 間の同期オーバーヘッド
- 最適でないカーネルコンパイルパラメータ
- **推奨**: `CUDA_PROFILE=1 ./app` でプロファイリングし、wgpu `wgpu_core::Trace` ログと比較

## CUDA と wgpu のベンチマーク

### 環境セットアップ
同一のハードウェアとドライババージョンを準備:

```bash
# ベースラインメトリクスを記録
cargo run --features backend-wgpu --release -- --bench-frames 1000 > wgpu-metrics.txt
cargo run --features backend-cuda --release -- --bench-frames 1000 > cuda-metrics.txt

# 出力を比較
diff wgpu-metrics.txt cuda-metrics.txt
```

### メトリクス比較
予想 される出力形式（[CI Metrics Format](../quality/metrics-format.md) で定義):
```
p0-metrics:
  avg_ms: 14.2
  p95_ms: 18.5
  draw_calls: 145
```

### 許容される差異
- フレーム時間差異: バックエンド間で ±20% は許容
- ドローコールは同一である必要（バッチングはバックエンド非依存）
- ビジュアル出力は pixel-identical である必要（浮動小数点数学の丸め誤差以内）

## ドキュメント参照
- [Requirements](../architecture/requirements.md) - CUDA がベーステクノロジーとしてリストされています
- [Workspace Architecture](../architecture/workspace-architecture.md) - `webgpui-render-cuda` クレート設計
- [API Swapping Quality Plan](../rendering/api-swapping-quality-plan.md) - 等価性テスト戦略
- [CUDA Toolkit Docs](https://docs.nvidia.com/cuda/) - NVIDIA 公式ドキュメント
- [cudarc Crate Docs](https://docs.rs/cudarc/) - セーフな CUDA Rust バインディング

## FAQ

**Q: 同じアプリケーションで CUDA と wgpu の両方を使用できますか?**  
A: はい! 両方のフィーチャーがコンパイルされている場合（`--features backend-wgpu,backend-cuda`）、アプリは `BackendSelector::available()` と `BackendMode` enum を使用してランタイムでどちらのバックエンドを使用するかを選択できます。以下が可能になります:
  - フォールバック戦略（CUDA を優先、利用不可の場合は wgpu にフォールバック）
  - ユーザー選択（CLI フラグまたは設定ファイルでバックエンド選択）
  - ベンチマーク（両方のバックエンドで同じワークロードを実行）
  - 優雅な劣化（GPU が利用不可を検出して自動的に切り替え）

**Q: コード変更なしで CUDA と wgpu で実行できますか?**  
A: はい、`Renderer` トレイトは両方のバックエンドで実装されています。アプリコードはトレイトメソッドのみを呼び出し、バックエンド固有の API は呼び出しません。

**Q: AMD GPU はどうですか?**  
A: AMD GPU は wgpu でサポートされています。AMD 特化最適化については、将来のバックエンドで HIP（AMD の CUDA 同等物）を検討してください。

**Q: CUDA カーネル最適化に貢献できますか?**  
A: はい! CUDA カーネルコードは `crates/webgpui-render-cuda/kernels/` に配置されます（将来）。パフォーマンス改善と等価性テスト検証を含む PR を送信してください。
