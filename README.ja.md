<div align="center">

<img src="assets/OpenWarp-logo.svg" alt="OpenWarp" width="128" />

# OpenWarp

**完全に分散化されたターミナル —— あなたの AI、あなたの Agent、あなたの鍵、あなたのマシン。**

OpenWarp は [Warp](https://github.com/warpdotdev/warp) のコミュニティフォークで、完全な
Warp ターミナル体験を維持しながら、**Warp の強制的なクラウド依存を取り除きます**。
AI プロバイダーレイヤーを開放し、任意のサードパーティ CLI Agent を接続可能にし、
リモートファイルブラウジングとコードレビューを備えた SSH ホストマネージャーを内蔵し、
多数の上流レンダリング問題を修正 —— すべての認証情報、会話、Agent 履歴は
自分のマシンだけに留まります。

[ダウンロード](https://github.com/TranscriptionFactory/warp/releases/latest) · [English](./README.md) · [简体中文](./README.zh-CN.md) · [上流 Warp](https://www.warp.dev)

> 開発初期段階。プレリリースビルドのみ提供 — 荒削りな部分があります。**Warp, Inc. とは無関係です。**

</div>

---

## なぜ OpenWarp か

上流 Warp は AI、アカウント、同期、Agent 履歴を Warp のクラウドに紐付けます。
OpenWarp はそのレイヤーを完全に開放し、**上流クライアントにはない機能を追加**します:

| | 上流 Warp | OpenWarp |
| --- | --- | --- |
| クラウド依存 | Warp バックエンドへの強依存(認証 / Drive / 履歴 / Agent) | **完全分散化、強制的なクラウド通信なし** |
| AI プロバイダー | Warp ゲートウェイのみ | **任意の OpenAI 互換エンドポイント + 6 種のネイティブプロトコル** |
| サードパーティ Agent | 内蔵 Warp Agent のみ | **任意の CLI Agent —— DeepSeek-TUI / Codex / Claude Code / Agy を統合済み** |
| SSH 管理 | 非内蔵 | **内蔵 SSH ホストマネージャー(接続 / 設定 / tmux)** |
| リモートファイルブラウジング | 非内蔵 | **内蔵 SFTP ファイルブラウザ** |
| リモートコードレビュー | なし | **リモート SSH ホスト上の diff + コードレビューパネル** |
| Markdown レンダリング | 上流ベースライン | **調整済み MD パイプライン —— コードブロック、表、CJK 混在;リモートファイルのレンダリングプレビュー** |
| 画像表示 | 非内蔵 | **アプリ内画像ビューア(ローカル + リモート)** |
| テーマ | 単一グローバルテーマ | **ウィンドウごとのテーマ** |
| フォントレンダリング | 上流 cosmic_text デフォルト | **CJK ソフトラップキャレット + 太字サブピクセル修正** |
| 認証情報 | クラウドアカウント | **ローカル設定ファイル、デバイスから出ない** |
| システムプロンプト | サーバー側で組み立て、不透明 | **minijinja テンプレート、完全編集可能** |
| UI 言語 | 英語 | **英語 + 簡体字中国語 + 日本語、拡張可能** |
| Cloud Agent / Computer Use | デフォルトで有効 | **デフォルトで無効(段階的に物理削除中)** |
| Blocks / Workflows / キーマップ | 維持 | 完全に維持、継続的に同期 |
| ライセンス | AGPL-3.0 / MIT デュアル | 上流と同じ |

## 上流 Warp がサポートせず、OpenWarp がサポートする機能

これらは OpenWarp がフォークの上に追加した独自機能です:

- **SSH ホストマネージャー** —— ターミナル内で SSH ホストとセッションを直接接続・
  設定・管理(tmux 統合付き)。外部スイッチャー不要。
- **SFTP ファイルブラウザ** —— リモートホストのグラフィカルなファイルブラウザ。
  リモートファイルをローカル同様に閲覧・オープン。
- **リモートコードレビュー** —— diff ビューアとコードレビューパネルが SSH セッションを
  跨いで動作。ターミナル内でリモートリポジトリの変更をレビューし、hunk をステージし、
  diff をナビゲート。
- **リモート Markdown プレビュー** —— リモートホスト上のファイルもローカル同様に
  レンダリング済み Markdown プレビューを切り替え可能。
- **アプリ内画像ビューア** —— 画像(PNG、JPEG、GIF、SVG、WebP)をターミナル内で直接
  オープン。ローカルと SFTP 経由のリモートの両方で動作。
- **ウィンドウごとのテーマ** —— ウィンドウごとに異なるテーマを設定。テーマプレビューは
  アクティブウィンドウにのみ適用。
- **サードパーティ CLI Agent** —— 任意の CLI Agent を Warp Block モデルに統合。
  ファーストクラスのアダプター:
  - **DeepSeek-TUI**(完了通知、テキスト通知マッピング、入力復元をすべて配線済み)
  - **Google Antigravity**(`agy`)—— ネイティブサポート
  - **Codex CLI**、**Claude Code** などの主要 CLI Agent
  - OSC9 / OSC777 経由で OpenWarp の通知センターに統一ルーティング
- **マルチプロバイダー BYOP** —— 6 種のネイティブプロトコル(OpenAI / OpenAIResp /
  Anthropic / Gemini / Ollama / DeepSeek)を明示的にバインド。任意の OpenAI 互換
  プロキシがそのまま動作。認証情報はローカルに保持。
- **完全分散化** —— Warp アカウントなし、強制ログインなし、クラウド Drive /
  Notebook 同期なし、クラウド Agent 履歴なし。クラウドコードパスは段階的に
  物理削除中。
- **Markdown レンダリング改善** —— AI block 内の構造化テーブルレンダリング、
  設定可能な見出しスケール、コードブロック・表・リスト・日英混在テキストの
  安定性向上。
- **フォントレンダリングアルゴリズム修正** —— CJK ソフトラップキャレットのオフセット、
  小サイズ CJK の太字など、上流の長年のレンダリング問題を修正。
- **エディタサポート強化** —— Vue ファイルのインジェクションハイライト(JS/CSS/TS)と
  Vue SFC のブロックレベルコメント。
- **CLI Agent 設定 UI** —— CLI Agent を設定するグラフィカルな設定パネル。
- **ユーザーメッセージの右寄せ** —— チャットログでユーザーメッセージを右側に表示し、
  会話の流れを明確に。

## 3 ステップでターミナルを完全に自分の手に

**01 · 任意のプロバイダーを接続**
設定で Base URL と API key を貼り付けるだけ —— OpenAI Chat Completions 互換の
エンドポイントならそのまま動作。認証情報はローカルにのみ保存。

**02 · 動的プロンプトを記述**
minijinja ベースのテンプレートエンジンが、カレントディレクトリ・言語・ロールに
基づいてシステムプロンプトをリアルタイムにレンダリング。

**03 · すぐに使う**
モデル、会話、コマンド提案、サードパーティ Agent をワンクリックで切り替え ——
体験は Warp と同一、ただしすべてのレイヤーがあなたのもの。

## 検証済み AI プロバイダー

| プロバイダー | Base URL | 備考 |
| --- | --- | --- |
| **OpenAI** | `https://api.openai.com/v1` | ネイティブプロトコル |
| **Anthropic** | genai ネイティブ | Claude 4.x ファミリー |
| **DeepSeek** | `https://api.deepseek.com/v1` | thinking + tool calling |
| **Gemini** | genai ネイティブ | Google AI Studio |
| **Ollama** | `http://localhost:11434/v1` | ローカル推論、key 不要 |
| **OpenRouter** | `https://openrouter.ai/api/v1` | アグリゲータゲートウェイ |
| **Qwen / Groq / Together / LM Studio / 任意の OpenAI 互換プロキシ** | — | 設定するだけ |

## コア機能

- **BYOP カスタムプロバイダー** —— [genai](https://github.com/jeremychone/rust-genai) 0.6
  上に 6 種のネイティブプロトコルを明示的にバインド
- **サードパーティ CLI Agent** —— DeepSeek-TUI / Google Antigravity(`agy`)/
  Codex CLI / Claude Code を OSC9 経由で Block と通知センターにルーティング
- **SSH ホストマネージャー** —— ターミナル内で SSH ホストとセッションを管理、
  tmux 統合付き
- **SFTP ファイルブラウザ** —— グラフィカルなファイルツリーでリモートファイルを
  閲覧・オープン
- **リモートコードレビュー** —— リモート SSH リポジトリ上の diff ビューアと
  レビューパネル
- **リモート Markdown プレビュー** —— リモートファイルのレンダリングプレビュー切替
- **アプリ内画像ビューア** —— ローカル・リモート画像(PNG/JPEG/GIF/SVG/WebP)を
  ターミナル内で直接オープン
- **ウィンドウごとのテーマ** —— ウィンドウ単位の独立テーマ、他ウィンドウを乱さない
  プレビュー
- **SSE ストリーミング** —— Warp ファーストパーティと同一のインクリメンタル block
  レンダリング
- **18 のローカルツール** —— shell / read / edit / search / mcp / drive docs /
  skills / ask、すべてローカル実行
- **システムプロンプトテンプレート** —— opencode から移植した 8 つのモデルファミリー
  プロンプト(default / anthropic / gpt / beast / gemini / kimi / codex / trinity)
- **models.dev 統合** —— 検索可能な Providers サブページ、数千のモデルエントリを
  プリロード
- **レンダリング改善** —— 調整済み Markdown パイプライン(構造化テーブル、設定可能な
  見出しスケール)+ CJK ソフトラップ / 太字修正
- **プライバシーファースト** —— Cloud Agent / Computer Use / Referral / テレメトリを
  デフォルトですべて無効
- **Warp 体験の維持** —— 上流と継続的にマージ。Blocks、Workflows、AI コマンド、
  キーマップ、テーマをすべて維持
- **ローカライズ UI** —— 簡体字中国語 + 日本語 + 英語、コミュニティで拡張可能
- **内蔵テーマ** —— VS Code 2026 Dark などを同梱
- **Vue SFC サポート** —— インジェクションハイライト(JS/CSS/TS)とブロックレベル
  コメント
- **Onkey** —— カスタムキーバインドのためのキーボードリマッピング

## 目指すもの

OpenWarp は次のようなターミナルを目指します:

1. **中央集権的なサービスなしで完全動作** —— アカウントなし、強制ログインなし、
   「クラウドに繋がらないと使えない」機能なし。
2. **AI と Agent をオープンなエコシステムとして扱う** —— 単一ベンダーではなく、
   主要な LLM プロバイダーと CLI Agent すべてがファーストクラス市民。
3. **リモートワークをネイティブに** —— SSH / tmux / SFTP / リモート diff /
   リモート画像表示を後付けではなく内蔵で。
4. **一日中使うに値する品質** —— CJK 混在、Markdown、コードブロック、フォント
   レンダリングが弱点であってはならない。
5. **上流 Warp と同期し続ける** —— Warp のエンジニアリングの恩恵を受けながら、
   フォークとしての方向性の自律を保つ。

これらの目標に共感するなら、ぜひ一緒に完成させましょう。

## ダウンロード

各リリースのビルド済みバイナリは
[Releases ページ](https://github.com/TranscriptionFactory/warp/releases/latest)にあります:

| プラットフォーム | アセット |
| --- | --- |
| macOS(Apple Silicon) | `OpenWarp-arm64.dmg` |
| macOS(Intel) | `OpenWarp-intel.dmg` |
| Linux(任意のディストリビューション) | `OpenWarp-x86_64.AppImage` |
| Debian / Ubuntu | `openwarp_<version>_amd64.deb` |
| Fedora / RHEL 8+ | `openwarp-<version>.x86_64.rpm` |
| Windows x64 | `OpenWarpSetup.exe` |
| ヘッドレス CLI(macOS / Linux、x86_64 + aarch64) | `openwarp-<os>-<arch>.tar.gz` |

- **AppImage**: `chmod +x OpenWarp-x86_64.AppImage && ./OpenWarp-x86_64.AppImage`
- **deb / rpm**: `sudo apt install ./openwarp_*_amd64.deb` · `sudo dnf install ./openwarp-*.x86_64.rpm`
- **macOS**: ビルドは未署名です。Gatekeeper に止められた場合は下記の
  [macOS Gatekeeper](#macos-gatekeeper) を参照。
- `.tar.gz` は `openwarp-oss` の静的 CLI ビルドです。OpenWarp がリモート SSH ホストに
  自身をインストールする際にも自動取得されます。

### macOS Gatekeeper

macOS が「OpenWarp は壊れている」と報告する場合は、検疫フラグをクリアします:

```bash
xattr -cr /Applications/OpenWarp.app
```

**システム設定 → プライバシーとセキュリティ**から**このまま開く**を選ぶことも
できます。

## ソースからビルド

```bash
git clone https://github.com/TranscriptionFactory/warp
cd warp
./script/bootstrap   # プラットフォーム別の依存関係
./script/run         # ビルドして実行
./script/presubmit   # fmt / clippy / テスト
```

素の `cargo` を使う場合は、**必ず OSS バイナリを明示的に指定してください**:

```bash
cargo build --release --bin openwarp-oss
cargo run   --release --bin openwarp-oss
```

> フィルタなしの `cargo build --release` / `cargo run --release --bin {warp,stable,dev,preview}`
> は実行しないでください —— これらのエントリポイント(`local.rs` / `stable.rs` / `dev.rs` /
> `preview.rs`)は Warp プライベートの `warp-channel-config` バイナリ経由で channel 設定を
> 読み込みます。このバイナリはクローズドソースのリポジトリにあります。コンパイルは通りますが、
> 生成された実行ファイルは起動時に panic し、`./script/install_channel_config` の実行を
> 求めます。そのスクリプトがクローンする SSH リポジトリには Warp 社員しかアクセスできません。
> OpenWarp ユーザーに必要なのは `openwarp-oss` バイナリだけです。

リポジトリのコードマップとエンジニアリングガイドは [AGENTS.md](AGENTS.md) を参照。

## ライセンス

上流 Warp と同じです:

- `warpui_core` / `warpui` クレート —— [MIT](LICENSE-MIT)
- それ以外 —— [AGPL-3.0](LICENSE-AGPL)

## ブランチと上流同期

`TranscriptionFactory/warp` は 2 本の長期ブランチを維持します:

| ブランチ | トラッキング | 目的 |
| --- | --- | --- |
| `main` | `TranscriptionFactory/warp:main`(デフォルト) | OpenWarp のメイン開発ライン。**すべての PR はここに。** |
| `warp-upstream` | `warpdotdev/warp:master` | 上流 Warp の純粋なミラー。新しい commit の取り込みに使用。**フォーク独自の変更は含まない。** |

**コントリビューター向け**

PR は **`main`** に対して開いてください。`warp-upstream` に対しては開かないでください。

**メンテナ向け(書き込み権限あり)**

GitHub Web UI で `main` の **"Sync fork" ボタンをクリックしないでください**。上流の
履歴全体が OpenWarp のメインラインに直接マージされ、大規模なコンフリクトを引き起こします。
上流の変更はミラーブランチ経由で取り込みます:

```bash
# 初回セットアップ
git remote add upstream https://github.com/warpdotdev/warp.git

# ミラーの更新
git checkout warp-upstream
git pull                          # upstream/master から fast-forward
git push origin warp-upstream

# 選択した commit を main に取り込む
git checkout main
git cherry-pick <sha>             # フル同期が妥当な場合は warp-upstream を merge
```

## Featured Partners

<a href="https://github.com/Hmbown/DeepSeek-TUI">
  <img src="assets/DeepSeek-TUI.png" alt="DeepSeek-TUI" width="96" align="left" />
</a>

**[DeepSeek-TUI](https://github.com/Hmbown/DeepSeek-TUI)** —— DeepSeek モデル
ファミリーのターミナル UI。OpenWarp はファーストクラス統合を提供:完了通知、
OSC9 テキスト通知マッピング、入力復元がすべて配線済みで、DeepSeek-TUI が
OpenWarp 内でネイティブ Block として動作します。

任意の OpenWarp ターミナルで `deepseek` を実行するだけで起動 —— Block ライフサイクル、
フッターステータス、通知センターがそのまま動作します。

<br clear="left" />

**[Google Antigravity](https://github.com/google/antigravity)**(`agy`)—— Google の
コーディングタスク向け CLI Agent。OpenWarp はネイティブ統合を提供し、`agy` が
ファーストクラス Block として動作。通知は OpenWarp の通知センターを通じて
ルーティングされます。

<br clear="left" />

> **DeepSeek-TUI の Windows 注意点** —— DeepSeek-TUI の `[notifications].method` は
> デフォルトで `auto` であり、Windows では組み込み許可リスト(iTerm.app / Ghostty /
> WezTerm)以外の `TERM_PROGRAM` に対して `Off` に解決されます。OpenWarp は
> `WarpTerminal` として識別されるため、Windows の OpenWarp でターン完了通知を
> 受け取るには、`~/.deepseek/config.toml` に以下を追加してください:
>
> ```toml
> [notifications]
> method = "osc9"
>
> [tui]
> notification_condition = "always"  # 任意:毎ターン通知
> ```

CLI Agent やターミナル周辺ツールをメンテナンスしていて、同様のファーストクラス
統合を望む場合は issue を開いてください —— 喜んでパートナーを増やします。

## Zap や Warp からの移行

このプロジェクトを以前の **Zap** ブランディングで使っていた方、または上流 **Warp**
から移行する方は、
[docs/migrate-from-warp.md](docs/migrate-from-warp.md) を参照して設定を
移行してください。

## ロードマップ

[docs/roadmap.md](docs/roadmap.md) を参照。

## コントリビューション

コミュニティからの貢献を歓迎します。フローの全体は [CONTRIBUTING.md](CONTRIBUTING.md)
を参照してください。

提出前に[既存の issue を検索](https://github.com/TranscriptionFactory/warp/issues)して
ください。セキュリティ脆弱性は
[CONTRIBUTING.md#reporting-security-issues](CONTRIBUTING.md#reporting-security-issues)
に従って非公開で報告してください。

## 謝辞

OpenWarp は Warp チームと多くのオープンソースプロジェクトの上に成り立っています:

[Warp](https://github.com/warpdotdev/warp) · [genai](https://github.com/jeremychone/rust-genai) · [opencode](https://github.com/opencode-ai/opencode) · [models.dev](https://models.dev) · [DeepSeek-TUI](https://github.com/Hmbown/DeepSeek-TUI) · [Google Antigravity](https://github.com/google/antigravity) · [Codex CLI](https://github.com/openai/codex) · [Tokio](https://github.com/tokio-rs/tokio) · [NuShell](https://github.com/nushell/nushell) · [Alacritty](https://github.com/alacritty/alacritty) · [Hyper](https://github.com/hyperium/hyper) · [minijinja](https://github.com/mitsuhiko/minijinja) · [cosmic-text](https://github.com/pop-os/cosmic-text)
