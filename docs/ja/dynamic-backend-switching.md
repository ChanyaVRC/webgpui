# GPU バックエンド動的切り替え

## 概要

WebGPUIは**ランタイムバックエンド選択**をサポートしており、アプリケーションが両方のバックエンドがコンパイルされている場合、wgpuとCUDAバックエンドの間で動的に選択できます。

これにより以下が可能になります:
- **フォールバック戦略**: CUDAを優先し、利用不可の場合はwgpuにフォールバック
- **ユーザー選択**: CLIフラグ、環境変数、または設定ファイルでバックエンド選択
- **ベンチマーク**: 同じハードウェア上でバックエンド間のパフォーマンスを比較
- **優雅な劣化**: GPU が利用不可を検出して自動的に切り替え
- **ハードウェア検出**: NVIDIA GPUを検出してCUDAを使用。それ以外の場合はwgpuにフォールバック

## コンパイル時機能選択

動的切り替えを有効にするには、両方のバックエンドをバイナリにコンパイルします:

```bash
# ランタイム選択のために両方のバックエンドを有効化
cargo build --features backend-wgpu,backend-cuda

# または Cargo.toml で
[dependencies]
webgpui = { version = "0.1", features = ["backend-wgpu", "backend-cuda"] }
```

単一のバックエンドのみを使用する場合（バイナリサイズを削減）:

```bash
# デフォルト: wgpu のみ
cargo build

# または明示的に単一バックエンド
cargo build --features backend-cuda  # CUDA のみ
cargo build --features backend-wgpu  # wgpu のみ
```

## ランタイムバックエンド検出 API

### 利用可能なバックエンドを確認

```rust
use webgpui_render::BackendSelector;

// コンパイルされたすべてのバックエンドを列挙
let available = BackendSelector::available();
for backend in &available {
    println!("利用可能: {} ({})", backend.name(), backend);
}

// 出力（両方の機能付き）:
// 利用可能: wgpu (wgpu)
// 利用可能: CUDA (CUDA)
```

### 特定のバックエンド利用可能性を確認

```rust
use webgpui_render::BackendSelector;

if BackendSelector::Cuda.is_available() {
    println!("CUDAバックエンドが利用可能");
} else {
    println!("CUDAバックエンドがコンパイルされていないか利用不可");
}

if BackendSelector::Wgpu.is_available() {
    println!("wgpuバックエンドが利用可能");
}
```

### ハードウェアを検出してバックエンドを選択

```rust
use webgpui_render::BackendSelector;

// 例: NVIDIA GPUを検出してCUDAを優先
fn select_backend() -> BackendSelector {
    // NVIDIA GPUを確認 (疑似コード; 実装はシステムに依存)
    if has_nvidia_gpu() && BackendSelector::Cuda.is_available() {
        BackendSelector::Cuda
    } else if BackendSelector::Wgpu.is_available() {
        BackendSelector::Wgpu
    } else {
        panic!("GPU バックエンドが利用可能ではありません!");
    }
}

fn has_nvidia_gpu() -> bool {
    // 実装: nvidia-smi または CUDA ランタイムを確認
    // MVP では、ユーザーが CUDA をリクエストした場合は true と仮定
    true
}
```

## アプリケーション レベルのバックエンド選択

### 例: CLI 引数

```rust
use std::env;
use webgpui_render::BackendSelector;

fn main() {
    // --backend=cuda または --backend=wgpu を解析
    let backend = env::args()
        .find(|arg| arg.starts_with("--backend="))
        .and_then(|arg| {
            match &arg[10..] {
                "cuda" => Some(BackendSelector::Cuda),
                "wgpu" => Some(BackendSelector::Wgpu),
                _ => None,
            }
        })
        .unwrap_or_else(|| {
            // デフォルト: 利用可能な場合はCUDAを優先、それ以外はwgpu
            if BackendSelector::Cuda.is_available() {
                BackendSelector::Cuda
            } else {
                BackendSelector::Wgpu
            }
        });

    println!("{} バックエンドを使用", backend.name());
    
    // バックエンドが実際に利用可能であることを確認
    if !backend.is_available() {
        eprintln!("エラー: {} バックエンドがコンパイルされていません", backend.name());
        std::process::exit(1);
    }

    // 選択されたバックエンドでレンダラーを作成
    // (webgpui-app での実装)
    // let renderer = create_renderer_for_backend(backend)?;
}
```

### 例: 環境変数

```rust
use std::env;
use webgpui_render::BackendSelector;

fn select_backend_from_env() -> BackendSelector {
    env::var("WEBGPUI_BACKEND")
        .ok()
        .and_then(|backend_name| {
            match backend_name.to_lowercase().as_str() {
                "cuda" => Some(BackendSelector::Cuda),
                "wgpu" => Some(BackendSelector::Wgpu),
                _ => None,
            }
        })
        .and_then(|backend| {
            if backend.is_available() {
                Some(backend)
            } else {
                eprintln!(
                    "要求されたバックエンド {} は利用不可。デフォルトを使用しています",
                    backend.name()
                );
                None
            }
        })
        .unwrap_or_else(|| {
            // デフォルトフォールバック
            BackendSelector::Wgpu
        })
}

// 使用法:
// WEBGPUI_BACKEND=cuda cargo run
// WEBGPUI_BACKEND=wgpu cargo run
```

### 例: 設定ファイル

```rust
use webgpui_render::BackendSelector;

#[derive(Debug, serde::Deserialize)]
struct Config {
    backend: String,
}

fn select_backend_from_config(config_path: &str) -> Result<BackendSelector, Box<dyn std::error::Error>> {
    let config_text = std::fs::read_to_string(config_path)?;
    let config: Config = toml::from_str(&config_text)?;

    match config.backend.to_lowercase().as_str() {
        "cuda" => {
            if BackendSelector::Cuda.is_available() {
                Ok(BackendSelector::Cuda)
            } else {
                Err("CUDA バックエンドがリクエストされましたが利用不可".into())
            }
        }
        "wgpu" => {
            if BackendSelector::Wgpu.is_available() {
                Ok(BackendSelector::Wgpu)
            } else {
                Err("wgpu バックエンドがリクエストされましたが利用不可".into())
            }
        }
        other => Err(format!("不明なバックエンド: {}", other).into()),
    }
}

// config.toml
// [webgpui]
// backend = "cuda"  # または "wgpu"
```

## フォールバック戦略

優先バックエンドが利用不可の場合は自動フォールバックを実装:

```rust
use webgpui_render::BackendSelector;

fn select_backend_with_fallback(prefer_cuda: bool) -> BackendSelector {
    if prefer_cuda && BackendSelector::Cuda.is_available() {
        BackendSelector::Cuda
    } else if BackendSelector::Wgpu.is_available() {
        BackendSelector::Wgpu
    } else if prefer_cuda && BackendSelector::Cuda.is_available() {
        // 上記で既に CUDA を試行
        panic!("GPU バックエンドが利用可能ではありません!");
    } else {
        panic!("GPU バックエンドが利用可能ではありません!");
    }
}

// よりシンプルなバージョン
fn select_backend_smart() -> BackendSelector {
    [BackendSelector::Cuda, BackendSelector::Wgpu]
        .into_iter()
        .find(|b| b.is_available())
        .expect("GPU バックエンドが利用可能ではありません!")
}
```

## 機能マトリックス

| シナリオ | コンパイルフラグ | バックエンド数 | ランタイム選択 |
|---------|------------------|----------------|----------------|
| 開発 (wgpu のみ) | `--features backend-wgpu` | 1 | 不可 (wgpu に強制) |
| 開発 (CUDA のみ) | `--features backend-cuda` | 1 | 不可 (CUDA に強制) |
| サーバー (両方利用可能) | `--features backend-wgpu,backend-cuda` | 2 | CLI/env/設定経由 |
| リリース (最適化済み) | `--features backend-wgpu` (デフォルト) | 1 | 不可 (最小バイナリ) |

## パフォーマンスに関する考慮事項

### バイナリサイズ
- 単一バックエンド: ~X MB
- 両方のバックエンド: ~X MB (コード重複は最小限; 両方のバックエンドが同じ`Renderer`トレイトを使用)
- バックエンド切り替えはゼロのランタイムオーバーヘッド (分岐はフィーチャーゲートによるコンパイル時)

### 初期化時間
- wgpu: 通常 100〜500ms (GPU ドライバとシステムに依存)
- CUDA: 通常 50〜200ms (JIT カーネルコンパイルは初回実行時に 100〜500ms を追加する場合あり)
- 統合コード: 両方がリンクされていることによるオーバーヘッドはほぼゼロ

### 切り替えオーバーヘッド
- バックエンドの切り替えにはアプリの再起動が必要 (実行中インスタンス内でのホットスワップはサポートされていない)
- ランタイム切り替えが必要な場合は、ラッパープロセスまたは個別バイナリの使用を検討

## バックエンド選択のテスト

### ユニットテスト

```rust
#[cfg(test)]
mod tests {
    use webgpui_render::BackendSelector;

    #[test]
    fn backends_are_available() {
        let available = BackendSelector::available();
        assert!(!available.is_empty());
    }

    #[test]
    fn wgpu_available() {
        assert!(BackendSelector::Wgpu.is_available());
    }

    #[test]
    #[cfg(feature = "backend-cuda")]
    fn cuda_available_when_feature_enabled() {
        assert!(BackendSelector::Cuda.is_available());
    }

    #[test]
    #[cfg(not(feature = "backend-cuda"))]
    fn cuda_unavailable_when_feature_disabled() {
        assert!(!BackendSelector::Cuda.is_available());
    }
}
```

### 統合テスト

```bash
# wgpu バックエンドをテスト
cargo test --features backend-wgpu

# CUDA バックエンドをテスト
cargo test --features backend-cuda

# 両方のバックエンドをテスト
cargo test --features backend-wgpu,backend-cuda

# デフォルト (wgpu) をテスト
cargo test
```

### CI 設定

```yaml
# .github/workflows/test.yml
strategy:
  matrix:
    backend:
      - wgpu
      - cuda
      - both
env:
  WEBGPUI_BACKEND: ${{ matrix.backend }}
```

## 他のソリューションとの比較

### オプション A: コンパイル時のみ (元の設計)
- ✅ 最小バイナリサイズ
- ✅ 最も明確なコードパス
- ❌ ランタイムの柔軟性なし
- ❌ 利用不可なバックエンドを検出できない (ランタイム障害まで)

### オプション B: ランタイム選択 (現在の設計)
- ✅ 柔軟なランタイム選択
- ✅ グレースフルフォールバックサポート
- ✅ ベンチマーク機能
- ✅ スマートハードウェア検出
- ✅ フィーチャーゲートにより未使用バックエンドを除外
- ❌ 両方のバックエンド有効時、バイナリがやや大きい
- ❌ 検出 API の小さいコードオーバーヘッド

### オプション C: プラグインシステム
- ✅ 最も柔軟
- ✅ 外部ライブラリからバックエンドをロード可能
- ❌ はるかに複雑
- ❌ より大きなランタイムオーバーヘッド
- ❌ 依存性管理の複雑さ

**推奨**: オプション B（現在の設計）は柔軟性、コードの単純性、ランタイムパフォーマンスのバランスが取れている

## 将来の拡張

1. **ホット切り替え**: アプリを再起動せずにバックエンド切り替えをサポート（状態シリアライゼーションが必要）
2. **ウィンドウごとのバックエンド**: 異なるバックエンドを使用している異なるウィンドウ
3. **パフォーマンステレメトリ**: 自動プロファイリングと推奨バックエンド選択
4. **クラウドデプロイメント**: クラウドプロバイダーの GPU 利用可能性に基づいてバックエンド選択
5. **モバイルサポート**: 現在のバックエンドと並行してモバイル固有のバックエンド（Metal、Vulkan）を追加

## 参考資料
- [CUDA バックエンド対応](cuda-backend.md) - 完全な CUDA セットアップおよび設定ガイド
- [ワークスペース構成案](workspace-architecture.md) - フィーチャーフラグポリシーの詳細
- [要件定義](requirements.md) - アーキテクチャ決定の根拠
