<div align="center">

<img src="assets/zap-logo.svg" alt="Zap" width="128" />

# Zap

**完全に分散化されたターミナル —— あなたの AI、あなたの Agent、あなたの鍵、あなたのマシン。**

Zap は [Warp](https://github.com/warpdotdev/warp) のコミュニティフォークで、完全な
Warp ターミナル体験を維持しながら、**Warp の強制的なクラウド依存を排除**します。
AI プロバイダーレイヤーを開放し、任意のサードパーティ CLI Agent を接続可能にし、
リモートファイルブラウジングとコードレビューを備えた SSH ホストマネージャーを内蔵し、
多数の上流レンダリング問題を修正 —— すべての認証情報、会話、Agent 履歴は
デフォルトで自分のマシンに留まります。

[English](./README.md) · [简体中文](./README.zh-CN.md) · [上流 Warp](https://www.warp.dev)

> 開発初期段階。公式リリース未定。**Warp, Inc. とは無関係です。**

</div>

---

## なぜ Zap か

上流 Warp は AI、アカウント、同期、Agent 履歴を Warp のクラウドに紐付けます。
Zap はそのレイヤーを完全に開放し、**上流クライアントにはない機能を追加**します:

| | 上流 Warp | Zap |
| --- | --- | --- |
| クラウド依存 | Warp バックエンドに強依存(認証 / Drive / 履歴 / Agent) | **完全に分散化、強制クラウド通信なし** |
| AI プロバイダー | Warp ゲートウェイのみ | **任意の OpenAI 互換エンドポイント + 6 種のネイティブプロトコル** |
| サードパーティ Agent | 内蔵 Warp Agent のみ | **任意の CLI Agent — DeepSeek-TUI / Codex / Claude Code / Agy を統合** |
| SSH 管理 | 非内蔵 | **内蔵 SSH ホストマネージャー(接続 / 設定 / tmux)** |
| リモートファイルブラウジング | 非内蔵 | **内蔵 SFTP ファイルブラウザー** |
| リモートコードレビュー | 非対応 | **リモート SSH ホスト上の diff + コードレビューパネル** |
| Markdown レンダリング | 上流ベースライン | **最適化された MD パイプライン — コードブロック、表、日中混在;リモートファイルのプレビュー** |
| 画像表示 | 非内蔵 | **アプリ内画像ビューアー(ローカル + リモート)** |
| テーマ | 単一グローバルテーマ | **ウィンドウごとのテーマ** |
| フォントレンダリング | 上流 cosmic_text デフォルト | **CJK ソフトラップ caret + 太字サブピクセル修正** |
| 認証情報 | クラウドアカウント | **ローカル設定ファイル、デバイス外に出ない** |
| システムプロンプト | サーバー側で組み立て、不透明 | **minijinja テンプレート、完全編集可能** |
| UI 言語 | 英語 | **英語 + 簡体字中国語 + 日本語、拡張可能** |
| Cloud Agent / Computer Use | デフォルトでオン | **デフォルトでオフ(物理的に削除中)** |
| Blocks / Workflows / Keymaps | 維持 | 完全保持、継続的同期 |
| ライセンス | AGPL-3.0 / MIT デュアル | 上流と同じ |

## 上流 Warp がサポートせず、Zap がサポートする機能

これらは Zap がフォークの上に追加した独自機能です:

- **SSH ホストマネージャー** — ターミナル内で直接 SSH ホストとセッションを
  接続・設定・管理(tmux 統合)。外部スイッチャー不要。
- **SFTP ファイルブラウザー** — リモートホスト向けグラフィカルファイルブラウザー。
  ローカルファイルのようにリモートファイルを閲覧・開封。
- **リモートコードレビュー** — diff ビューアーとコードレビューパネルが
  SSH セッション越しに動作。ターミナル内でリモートリポジトリの変更確認、
  ハンクのステージング、diff ナビゲーションが可能。
- **リモート Markdown プレビュー** — リモートホスト上のファイルに対して
  レンダリング済み Markdown プレビューを切り替え表示(ローカルファイルと同様)。
- **アプリ内画像ビューアー** — ターミナル内で直接画像(PNG、JPEG、GIF、SVG、WebP)を
  開封。SFTP 経由でローカル・リモート両方に対応。
- **ウィンドウごとのテーマ** — ウィンドウごとに異なるテーマを設定可能。
  テーマプレビューはアクティブウィンドウのみに適用。
- **サードパーティ CLI Agent** — 任意の CLI Agent を Warp Block モデルに統合。
  ファーストクラスアダプター:
  - **DeepSeek-TUI**(完了通知、テキスト通知マッピング、入力復元を完全統合)
  - **Google Antigravity**(`agy`) — ネイティブサポート
  - **Codex CLI**、**Claude Code**、その他主要 CLI Agent
  - OSC9 / OSC777 経由で Zap の通知センターに統一路由
- **BYOP マルチプロバイダー** — 6 種のネイティブプロトコル(OpenAI / OpenAIResp /
  Anthropic / Gemini / Ollama / DeepSeek)を明示的にバインド。任意の OpenAI 互換
  プロキシがそのまま動作。認証情報はローカルのみ。
- **完全分散化** — Warp アカウント不要、強制ログインなし、クラウド Drive /
  Notebook 同期なし、クラウド Agent 履歴なし。クラウドコードパスは段階的に物理削除中。
- **Markdown レンダリング改善** — AI ブロック内の構造化テーブルレンダリング、
  見出しスケールの設定可能化、コードブロック・表・リスト・日中混在テキストの安定性向上。
- **フォントレンダリングアルゴリズム修正** — CJK ソフトラップ caret オフセット、
  小サイズ漢字の太字など、長年の上流レンダリング問題を修正。
- **エディタサポート強化** — Vue ファイルのインジェクションハイライト(JS/CSS/TS)と
  Vue SFC ファイルのブロックレベルコメント。
- **CLI Agent 設定 UI** — CLI Agent を設定するためのグラフィカル設定パネル。
- **ユーザーメッセージの右寄せ** — チャット履歴でユーザーメッセージを右側に表示し、
  会話の流れを明確化。

## 3 ステップでターミナルを完全に自分の手に

**01 · 任意のプロバイダーを接続**
設定で Base URL と API キーを入力 — 任意の OpenAI Chat Completions 互換
エンドポイントがそのまま動作。認証情報はローカルのみ保存。

**02 · 動的プロンプトを作成**
minijinja ベースのテンプレートエンジンが、現在の作業ディレクトリ、言語、
ロールに基づいてシステムプロンプトをリアルタイムでレンダリング。

**03 · すぐに使用**
ワンクリックでモデル、会話、コマンド提案、サードパーティ Agent を切り替え —
体験は Warp と同一ですが、すべてのレイヤーがあなたのものです。

## 検証済み AI プロバイダー

| プロバイダー | Base URL | 備考 |
| --- | --- | --- |
| **OpenAI** | `https://api.openai.com/v1` | ネイティブプロトコル |
| **Anthropic** | genai ネイティブ経由 | Claude 4.x ファミリー |
| **DeepSeek** | `https://api.deepseek.com/v1` | thinking + tool calling |
| **Gemini** | genai ネイティブ経由 | Google AI Studio |
| **Ollama** | `http://localhost:11434/v1` | ローカル推論、キー不要 |
| **OpenRouter** | `https://openrouter.ai/api/v1` | アグリゲーターゲートウェイ |
| **Qwen / Groq / Together / LM Studio / 任意の OpenAI 互換プロキシ** | — | 設定してすぐ使用 |

## 主要機能

- **BYOP カスタムプロバイダー** — 6 種のネイティブプロトコルを
  [genai](https://github.com/jeremychone/rust-genai) 0.6 上に明示的にバインド
- **サードパーティ CLI Agent** — DeepSeek-TUI / Google Antigravity (`agy`) /
  Codex CLI / Claude Code を OSC9 経由で Blocks と通知センターに統合
- **SSH ホストマネージャー** — ターミナル内で SSH ホストとセッションを管理、tmux 統合
- **SFTP ファイルブラウザー** — グラフィカルなファイルツリーでリモートファイルを閲覧・開封
- **リモートコードレビュー** — リモート SSH リポジトリ上の diff ビューアーとレビューパネル
- **リモート Markdown プレビュー** — リモートファイルのレンダリングプレビュー切替
- **アプリ内画像ビューアー** — ローカルとリモートの画像(PNG/JPEG/GIF/SVG/WebP)を
  ターミナル内で直接開封
- **ウィンドウごとのテーマ** — ウィンドウごとに独立したテーマ、プレビューは他に影響しない
- **SSE ストリーミング** — Warp のファーストパーティパスと同一の増分ブロックレンダリング
- **18 個のローカルツール** — shell / read / edit / search / mcp / drive docs / skills / ask、
  すべてローカル実行
- **システムプロンプトテンプレート** — opencode から移植された 8 種のモデルファミリープロンプト
  (default / anthropic / gpt / beast / gemini / kimi / codex / trinity)
- **models.dev 統合** — 数千の事前読み込みモデルエントリを持つ検索可能な Providers サブページ
- **レンダリング改善** — 最適化された Markdown パイプライン(構造化テーブル、
  設定可能な見出しスケール) + CJK ソフトラップ / 太字修正
- **プライバシー優先** — Cloud Agent / Computer Use / Referral / テレメトリ
  すべてデフォルトで無効
- **Warp 体験を維持** — 継続的に上流とマージ。Blocks、
  Workflows、AI コマンド、Keymaps、テーマすべて保持
- **多言語 UI** — 簡体字中国語 + 日本語 + 英語、コミュニティ拡張可能
- **内蔵テーマ** — VS Code 2026 Dark などのテーマを同梱
- **Vue SFC サポート** — インジェクションハイライト(JS/CSS/TS)とブロックレベルコメント
- **Onkey** — キーボードリマッピング、カスタムキーバインド

## 私たちが目指すもの

Zap は次のようなターミナルを目指します:

1. **中央集権サービスなしで完全に動作** — アカウント不要、強制ログインなし、
   「クラウドが到達可能な時だけ動く」機能は存在しない。
2. **AI と Agent をオープンエコシステムとして扱う** — 単一ベンダーではなく、
   すべての主要 LLM プロバイダーと CLI Agent がファーストクラス市民。
3. **リモートワークをネイティブに** — SSH / tmux / SFTP / リモート diff / リモート
   画像表示が後付けではなく内蔵。
4. **終日使うに値する** — 日中混在、Markdown、コードブロック、フォントレンダリングが
   弱点であってはならない。
5. **上流 Warp との同期を維持** — Warp の工学的成果を享受しながら、
   フォークレベルの自律性を保つ。

これらの目標に共感するなら、一緒に完成させましょう。

## ソースからビルド

```bash
git clone https://github.com/zerx-lab/warp
cd warp
./script/bootstrap   # プラットフォーム固有の依存関係
./script/run         # ビルド & 実行
./script/presubmit   # fmt / clippy / tests
```

生の `cargo` を使う場合は、**必ず OSS バイナリを明示的に指定**してください:

```bash
cargo build --release --bin warp-oss
cargo run   --release --bin warp-oss
```

> フィルタなしで `cargo build --release` / `cargo run --release --bin {warp,stable,dev,preview}`
> を実行しないでください — これらのエントリポイント(`local.rs` / `stable.rs` / `dev.rs` / `preview.rs`)は
> Warp のプライベートな `warp-channel-config` バイナリを通じてチャネル設定を読み込みます。
> そのバイナリはクローズドソースのリポジトリにあります。コンパイルは成功しますが、
> 生成された実行ファイルは起動時にパニックし、`./script/install_channel_config` の実行を求めます。
> そのスクリプトは Warp 従業員のみがアクセスできる SSH リポジトリをクローンします。
> Zap ユーザーは `warp-oss` バイナリのみ必要です。

リポジトリのコードマップとエンジニアリングガイドは [AGENTS.md](AGENTS.md) を参照。

## ライセンス

上流 Warp と同じ:

- `warpui_core` / `warpui` クレート — [MIT](LICENSE-MIT)
- その他すべて — [AGPL-3.0](LICENSE-AGPL)

## ブランチと上流同期

`zerx-lab/warp` は 2 本の長期ブランチを維持します:

| ブランチ | 追跡対象 | 目的 |
| --- | --- | --- |
| `main` | `zerx-lab/warp:main`(デフォルト) | Zap のメイン開発ライン。**すべての PR はここに。** |
| `warp-upstream` | `warpdotdev/warp:master` | 上流 Warp のクリーンミラー。新しいコミットの取り込みに使用。**フォークローカルの変更なし。** |

**コントリビューター向け**

PR は **`main`** に対して出してください。`warp-upstream` には出さないでください。

**メンテナー向け(書き込み権限)**

GitHub Web UI で `main` の **"Sync fork" ボタンをクリックしないでください**。上流の履歴全体が Zap の
メインラインに直接マージされ、大規模なコンフリクトが発生します。ミラーブランチ経由で上流の変更を取り込んでください:

```bash
# 一度だけの設定
git remote add upstream https://github.com/warpdotdev/warp.git

# ミラーを更新
git checkout warp-upstream
git pull                          # upstream/master から fast-forward
git push origin warp-upstream

# 選択したコミットを main に取り込み
git checkout main
git cherry-pick <sha>             # または適時 warp-upstream をマージ
```

## パートナー

<a href="https://github.com/Hmbown/DeepSeek-TUI">
  <img src="assets/DeepSeek-TUI.png" alt="DeepSeek-TUI" width="96" align="left" />
</a>

**[DeepSeek-TUI](https://github.com/Hmbown/DeepSeek-TUI)** — DeepSeek モデルファミリー向け
ターミナル UI。Zap はファーストクラス統合を提供:完了通知、OSC9 テキスト通知マッピング、
入力復元がすべて統合され、DeepSeek-TUI が Zap 内でネイティブ Block として動作します。

任意の Zap ターミナルで `deepseek` を実行するだけで起動 — Block ライフサイクル、
フッターステータス、通知センターがすべてそのまま動作します。

<br clear="left" />

<a href="https://github.com/google/antigravity">
  <img src="assets/agy-icon.png" alt="Google Antigravity" width="96" align="left" />
</a>

**[Google Antigravity](https://github.com/google/antigravity)**(`agy`) — Google の
コーディングタスク向け CLI Agent。Zap はネイティブ統合を提供し、`agy` がファーストクラス
Block として動作。通知は Zap の通知センターを通じてルーティングされます。

<br clear="left" />

> **Windows ユーザー向け注意(DeepSeek-TUI)** — DeepSeek-TUI の `[notifications].method` のデフォルトは
> `auto` で、Windows では内蔵許可リスト(iTerm.app / Ghostty / WezTerm)外の
> `TERM_PROGRAM` に対して `Off` に解決されます。Zap は `WarpTerminal` として識別されるため、
> Zap on Windows でターン完了通知を受け取るには、`~/.deepseek/config.toml` に以下を追加:
>
> ```toml
> [notifications]
> method = "osc9"
>
> [tui]
> notification_condition = "always"  # オプション: 毎ターン通知
> ```

CLI Agent やターミナル関連ツールをメンテナンスしていて、同様のファーストクラス統合を
希望する場合は、issue を作成してください — より多くのパートナーを喜んで接続します。

## OpenWarp / Warp からの移行

プロジェクトが Zap に改名される前から使っていた方(当時の名称は **OpenWarp**)、
または上流 **Warp** から乗り換える方は、
[docs/migrate-from-warp.ja.md](docs/migrate-from-warp.ja.md) を参照して設定を
引き継いでください。

## ロードマップ

[docs/roadmap.ja.md](docs/roadmap.ja.md) を参照してください。

## コントリビューション

コミュニティのコントリビューションを歓迎します。完全なフローは
[CONTRIBUTING.md](CONTRIBUTING.md) を参照。

提出前に[既存の issue を検索](https://github.com/zerx-lab/warp/issues)してください。
セキュリティ脆弱性は [CONTRIBUTING.md#reporting-security-issues](CONTRIBUTING.md#reporting-security-issues)
に従って非公開で報告してください。

## 謝辞

Zap は Warp チームと多くのオープンソースプロジェクトの上に成り立っています:

[Warp](https://github.com/warpdotdev/warp) · [genai](https://github.com/jeremychone/rust-genai) · [opencode](https://github.com/opencode-ai/opencode) · [models.dev](https://models.dev) · [DeepSeek-TUI](https://github.com/Hmbown/DeepSeek-TUI) · [Google Antigravity](https://github.com/google/antigravity) · [Codex CLI](https://github.com/openai/codex) · [Tokio](https://github.com/tokio-rs/tokio) · [NuShell](https://github.com/nushell/nushell) · [Alacritty](https://github.com/alacritty/alacritty) · [Hyper](https://github.com/hyperium/hyper) · [minijinja](https://github.com/mitsuhiko/minijinja) · [cosmic-text](https://github.com/pop-os/cosmic-text)
