# APIマッピング表（既存API -> 新API）

## 1. この表の位置づけ
この文書は、既存 WebUI エンジンを `webgpui` に置き換えるための確定版マッピング（v0.1）である。

- 互換レイヤー: `webgpui-compat`
- 新規実装先: `webgpui` / `webgpui-core` / `webgpui-app`
- 適用範囲: MVP（container/text/image簡易、主要スタイル、主要イベント）

## 2. 記法
- 既存 API は一般的な WebUI エンジンでよく使われる命名で統一
- 新 API は Rust 向けに型安全な命名を採用
- 状態:
  - `MUST`: MVPで必須
  - `SHOULD`: MVP後半で対応推奨
  - `LATER`: MVP外

## 3. ノード/ツリー API
| 既存API | 新API（互換レイヤー） | 新API（本体） | 状態 | 移行ノート |
|---|---|---|---|---|
| `createNode(type)` | `compat::node_create(kind)` | `webgpui::Node::new(kind)` | MUST | `type` は `NodeKind` へ変換 |
| `appendChild(parent, child)` | `compat::node_append(parent, child)` | `webgpui::Tree::append(parent, child)` | MUST | 返り値なしを `Result<()>` に変更 |
| `insertBefore(parent, child, before)` | `compat::node_insert_before(...)` | `webgpui::Tree::insert_before(...)` | SHOULD | `before` が無効な場合はエラー |
| `removeChild(parent, child)` | `compat::node_remove(parent, child)` | `webgpui::Tree::remove(parent, child)` | MUST | detach後は `NodeId` 無効化 |
| `setText(node, text)` | `compat::text_set(node, text)` | `webgpui::Node::set_text(text)` | SHOULD | MVPは簡易テキストのみ |
| `setImage(node, src)` | `compat::image_set(node, src)` | `webgpui::Node::set_image(source)` | SHOULD | MVPはプレースホルダ描画可 |

## 4. スタイル API
| 既存API | 新API（互換レイヤー） | 新API（本体） | 状態 | 移行ノート |
|---|---|---|---|---|
| `setStyle(node, key, value)` | `compat::style_set(node, key, value)` | `webgpui::Style::set(prop, value)` | MUST | 文字列キーは `StyleProp` enum へ変換 |
| `setStyles(node, object)` | `compat::style_set_many(node, styles)` | `webgpui::Node::set_style(style)` | MUST | 差分のみ更新（dirty化） |
| `getStyle(node, key)` | `compat::style_get(node, key)` | `webgpui::Style::get(prop)` | SHOULD | 計算済み値は将来対応 |
| `setPosition(node, x, y)` | `compat::style_position(node, x, y)` | `webgpui::Style::position(x, y)` | MUST | 単位は logical px へ統一 |
| `setSize(node, w, h)` | `compat::style_size(node, w, h)` | `webgpui::Style::size(w, h)` | MUST | Auto 値は `Option<f32>` で表現 |
| `setMargin(node, l, t, r, b)` | `compat::style_margin(node, ...)` | `webgpui::Style::margin(...)` | MUST | 4値省略記法は互換層で展開 |
| `setPadding(node, l, t, r, b)` | `compat::style_padding(node, ...)` | `webgpui::Style::padding(...)` | MUST | 4値省略記法は互換層で展開 |
| `setBackground(node, color)` | `compat::style_background(node, color)` | `webgpui::Style::background(color)` | MUST | 色文字列を RGBA へ変換 |
| `setBorder(node, width, color)` | `compat::style_border(node, width, color)` | `webgpui::Style::border(width, color)` | MUST | MVPは角丸なし |
| `setOpacity(node, alpha)` | `compat::style_opacity(node, alpha)` | `webgpui::Style::opacity(alpha)` | MUST | 範囲外値は clamp |

## 5. イベント API
| 既存API | 新API（互換レイヤー） | 新API（本体） | 状態 | 移行ノート |
|---|---|---|---|---|
| `addEventListener(node, type, handler)` | `compat::event_on(node, ty, cb)` | `webgpui::Events::on(node, ty, cb)` | MUST | ハンドラは `Send + Sync + 'static` |
| `removeEventListener(node, type, handler)` | `compat::event_off(node, ty, id)` | `webgpui::Events::off(node, ty, id)` | SHOULD | 関数ポインタ比較をID管理へ変更 |
| `dispatchEvent(node, event)` | `compat::event_dispatch(node, evt)` | `webgpui::Events::dispatch(node, evt)` | SHOULD | capture/bubbleはMVP基本のみ |
| `stopPropagation()` | `compat::event_stop_propagation(ctx)` | `EventContext::stop_propagation()` | MUST | 伝播停止を明示 |
| `preventDefault()` | `compat::event_prevent_default(ctx)` | `EventContext::prevent_default()` | SHOULD | デフォルト処理は入力種別依存 |
| `setFocus(node)` | `compat::focus_set(node)` | `webgpui::Input::focus(node)` | MUST | フォーカス喪失イベントを発火 |

## 6. ライフサイクル/実行 API
| 既存API | 新API（互換レイヤー） | 新API（本体） | 状態 | 移行ノート |
|---|---|---|---|---|
| `mount(root)` | `compat::app_mount(root)` | `webgpui::App::mount(root)` | MUST | 初回レイアウト + 初回描画を実行 |
| `unmount()` | `compat::app_unmount()` | `webgpui::App::unmount()` | SHOULD | リソース解放順を固定 |
| `update(node, patch)` | `compat::node_update(node, patch)` | `webgpui::Tree::update(node, patch)` | MUST | dirty範囲を返す設計へ |
| `requestRender()` | `compat::render_request()` | `webgpui::Renderer::request_frame()` | MUST | 更新なし時は coalescing |
| `setVSync(enabled)` | `compat::render_vsync(enabled)` | `webgpui::Renderer::set_vsync(enabled)` | MUST | 環境依存で反映遅延あり |
| `resize(width, height)` | `compat::viewport_resize(w, h)` | `webgpui::Renderer::resize(size)` | MUST | DPI 変化も同時処理 |

## 7. 計測/デバッグ API
| 既存API | 新API（互換レイヤー） | 新API（本体） | 状態 | 移行ノート |
|---|---|---|---|---|
| `getFPS()` | `compat::metrics_fps()` | `webgpui::Profiler::fps()` | SHOULD | 移動平均窓を統一 |
| `getFrameTime()` | `compat::metrics_frame_time()` | `webgpui::Profiler::frame_time()` | SHOULD | p95 を標準で出す |
| `enableOverlay(flag)` | `compat::debug_overlay(flag)` | `webgpui::Profiler::set_overlay(flag)` | LATER | MVPはログ出力優先 |

## 8. 非対応・差分（MVP時点）
| 既存API | 方針 | 代替手段 |
|---|---|---|
| `setFilter(node, cssFilter)` | LATER | 当面は未対応、画像前処理で代替 |
| `setTransition(node, ...)` | LATER | アプリ側タイムライン管理で代替 |
| `setGridLayout(node, ...)` | LATER | MVPは簡易レイアウトのみ |

## 9. 移行テンプレート
```rust
// Before (legacy)
// let root = createNode("container");
// setStyle(root, "background", "#20242a");
// addEventListener(root, "click", on_click);

// After (webgpui-compat)
let root = compat::node_create(NodeKind::Container)?;
compat::style_set(root, "background", "#20242a")?;
let _listener_id = compat::event_on(root, EventType::Click, on_click)?;
compat::app_mount(root)?;
```

## 10. 凍結ルール（確定運用）
- 本表 v0.1 は `MUST` 行を凍結対象とする
- `MUST` の破壊的変更は minor ではなく major でのみ許可
- 差分追加時は必ず migration note を同時更新する

## 11. 高速化用 独自APIマッピング（互換レイヤー非経由）
この節は、互換性より性能を優先する経路を定義する。

| 目的 | 既存API（代表） | 独自API（新） | 効果 | 注意点 |
|---|---|---|---|---|
| フレーム開始/終了 | `requestRender()` + 内部自動処理 | `webgpui::FastPath::begin_frame_fast(ctx)` / `end_frame_fast()` | フレーム境界の余分な処理を削減 | 呼び出し順序を守る必要 |
| バッチ送信 | `appendChild` + `setStyle` の積み上げ | `webgpui::FastPath::submit_batch(batch_key, instances)` | draw call 削減 | 高水準APIより責務が増える |
| 差分更新 | `update(node, patch)` | `webgpui::FastPath::mark_dirty_rect(node, rect)` | 再描画領域を最小化 | dirty管理の正確性が必要 |
| 一時バッファ | 内部で都度確保 | `webgpui::FastPath::allocate_transient_buffer(size)` | アロケーション削減 | メモリ再利用ルール必須 |
| パイプライン準備 | 初回描画時に遅延生成 | `webgpui::FastPath::prewarm_pipeline(desc)` | 初回スタッタリング抑制 | 起動時コスト増加 |
| テキスト準備 | 初回文字出現時に生成 | `webgpui::FastPath::prewarm_glyph_cache(font, charset)` | 入力直後のカクつき低減 | 文字集合の設計が必要 |

### 11.1 採用ルール
- 既存画面の初期移行では独自APIを使わない
- profiler でボトルネックが確認できた箇所のみ独自APIへ置換する
- 独自API適用時は before/after の計測ログを必須化する

## 12. テスト保証ルール（確定）
- `MUST` 行の API には Compat/FastPath 同等性テストを必ず作成する
- 同等性テストは最低限「戻り値」「副作用」「イベント順序」「描画結果」を比較する
- API 仕様変更時は、変更対象 API の同等性テストを同一PRで更新する
- 同等性テストが fail の状態では `MUST` API のマージを禁止する

### 12.1 参照
- APIスワップ品質保証計画: `api-swapping-quality-plan.md`
