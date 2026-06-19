# Tauri 2.0 デバッグガイド

## ブレークポイント機能の有効化

このプロジェクトは Tauri 2.0 を使用して実装されており、以下の修正によってブレークポイント機能が有効化されています。

## 修正内容

### 1. Cargo.toml の設定

- **デバッグプロファイルの追加**: 開発ビルド時に最適化をオフにし、デバッグ情報を含める

  - `opt-level = 0` - 最適化なし（デバッグしやすくするため）
  - `debug = true` - デバッグシンボル含有
  - `split-debuginfo = "packed"` - デバッグ情報を効率的にパック
  - `incremental = true` - インクリメンタルコンパイル対応

- **ロギング依存関係の追加**:
  - `log = "0.4"` - ロギングフレームワーク
  - `env_logger = "0.10"` - 環境変数ベースのロギング設定

### 2. .cargo/config.toml の新規作成

Cargo のプロジェクト別設定ファイルを作成し、デバッグ情報の最適な生成を設定：

```toml
[profile.dev]
split-debuginfo = "packed"
debug = true
opt-level = 0
```

### 3. tauri.conf.json の更新

WebView でのデバッグを有効にするため、ウィンドウ設定に `"devtools": true` を追加

### 4. .vscode/launch.json の更新

VS Code デバッグ設定を更新：

- **Launch App Debug**: Rust バックエンド用のデバッグ設定

  - 正しい実行ファイルパス: `taureria.exe`
  - WebView2 用のリモートデバッグポート: `1422`
  - 環境変数設定:
    - `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` - リモートデバッグポート指定
    - `RUST_BACKTRACE=1` - スタックトレース表示
    - `RUST_LOG=debug` - ログレベル指定

- **Attach to Webview**: Chrome ブラウザプロトコルでフロントエンドをデバッグ

### 5. .vscode/settings.json の新規作成

Rust-analyzer とデバッガーの設定を追加：

- Rust コード補完の最適化
- Clippy リント設定
- LLDB ブレークポイント設定
- コンソール表示設定

## デバッグの使い方

### VS Code でのデバッグ開始

1. **単体での Rust バックエンド デバッグ**:

   - デバッグサイドバーから "Launch App Debug" を選択して実行

2. **フロントエンド + バックエンド 統合デバッグ**:
   - "Tauri Dev All" コンパウンド設定を選択して実行
   - これにより Rust と JavaScript の両方のブレークポイントが有効化

### ブレークポイントの設定

#### Rust コードでのブレークポイント

- エディタの左マージンをクリックして赤い円を表示
- `src-tauri/src/` 内のファイルに設定可能
- デバッグ実行時に自動的に認識

#### JavaScript/TypeScript コードでのブレークポイント

- `src/` 内のファイルに設定可能
- DevTools で も設定可能（F12 キーで開く）

### 環境変数の利用

開発時に詳細なログを出力する場合：

```bash
# PowerShell の場合
$env:RUST_LOG = "debug"
$env:RUST_BACKTRACE = "1"

# または Cargo で実行
cargo run --manifest-path=src-tauri/Cargo.toml
```

## トラブルシューティング

### デバッガが接続できない場合

1. ポート 1422 が使用されていないか確認
2. WebView2 がインストールされているか確認
3. `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` が正しく設定されているか確認

### ブレークポイントが機能しない場合

1. デバッグビルドが実行されているか確認 (`--no-default-features` なし)
2. `tauri.conf.json` に `"devtools": true` が設定されているか確認
3. デバッグシンボルが生成されているか確認: `target/debug/*.pdb` ファイルの存在

### ログが出力されない場合

1. `env_logger` の初期化が `main.rs` に追加されているか確認
2. `RUST_LOG` 環境変数が設定されているか確認

## 参考資料

- [Tauri 2.0 公式デバッグガイド](https://tauri.app/develop/debugging/)
- [VS Code Rust デバッグ設定](https://github.com/rust-lang/vscode-rust)
- [WebView2 デバッグプロトコル](https://docs.microsoft.com/en-us/microsoft-edge/webview2/concepts/debugging)
