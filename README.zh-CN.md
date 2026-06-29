<div align="center">

<img src="assets/zap-logo.svg" alt="Zap" width="128" />

# Zap

**完全去中心化的终端 —— 你的 AI、你的 Agent、你的密钥、你的机器。**

Zap 是 [Warp](https://github.com/warpdotdev/warp) 的社区分支,在保留完整 Warp
终端体验的同时,**彻底移除了 Warp 的强制云端依赖**。它开放了 AI 提供商层,允许接入任意
第三方 CLI Agent,内置 SSH 主机管理器(带远程文件浏览与代码审查),并修复了多项上游
渲染问题 —— 所有凭据、对话和 Agent 历史默认留在你自己的机器上。

[English](./README.md) · [日本語](./README.ja.md) · [上游 Warp](https://www.warp.dev)

> 早期开发中,暂无正式发布。**与 Warp, Inc. 无关。**

</div>

---

## 为什么选择 Zap

上游 Warp 将 AI、账号、同步和 Agent 历史绑定在 Warp 云端。
Zap 完全打开了这一层,并**加入了上游客户端不具备的能力**:

| | 上游 Warp | Zap |
| --- | --- | --- |
| 云端依赖 | 强依赖 Warp 后端(认证 / Drive / 历史 / Agent) | **完全去中心化,无强制云端调用** |
| AI 提供商 | 仅 Warp 网关 | **任意 OpenAI 兼容端点 + 6 种原生协议** |
| 第三方 Agent | 仅内置 Warp Agent | **任意 CLI Agent — DeepSeek-TUI / Codex / Claude Code / Agy 均已接入** |
| SSH 管理 | 无内置 | **内置 SSH 主机管理器(连接 / 配置 / tmux)** |
| 远程文件浏览 | 无内置 | **内置 SFTP 文件浏览器** |
| 远程代码审查 | 不支持 | **远程 SSH 主机上的 diff + 代码审查面板** |
| Markdown 渲染 | 上游基线 | **优化后的 MD 管线 — 代码块、表格、中英混排;远程文件渲染预览** |
| 图片查看 | 无内置 | **应用内图片查看器(本地 + 远程)** |
| 主题 | 单一全局主题 | **每窗口独立主题** |
| 字体渲染 | 上游 cosmic_text 默认 | **CJK 软换行 caret + 加粗子像素修复** |
| 凭据 | 云端账号 | **本地配置文件,不离开设备** |
| 系统提示词 | 服务端拼装,不透明 | **minijinja 模板,完全可编辑** |
| 界面语言 | 英文 | **英文 + 简体中文 + 日语,可扩展** |
| Cloud Agent / Computer Use | 默认开启 | **默认关闭(正在物理移除)** |
| Blocks / Workflows / Keymaps | 保留 | 完整保留,持续同步 |
| 许可证 | AGPL-3.0 / MIT 双许可 | 与上游相同 |

## 上游 Warp 不支持但 Zap 支持的功能

以下是 Zap 在分支之上新增的独有能力:

- **SSH 主机管理器** — 在终端内直接连接、配置和管理 SSH 主机与
  会话(集成 tmux)。无需外部切换工具。
- **SFTP 文件浏览器** — 图形化远程文件浏览器,像操作本地文件一样
  浏览和打开远程文件。
- **远程代码审查** — diff 查看器和代码审查面板支持跨 SSH 会话。
  在终端内查看变更、暂存代码块、导航远程仓库的 diff。
- **远程 Markdown 预览** — 对远程主机上的文件切换渲染 Markdown 预览,
  与本地文件体验一致。
- **应用内图片查看器** — 直接在终端内打开图片(PNG、JPEG、GIF、SVG、WebP)。
  同时支持本地和远程文件(通过 SFTP)。
- **每窗口独立主题** — 每个窗口可设置不同主题;主题预览仅影响当前窗口。
- **第三方 CLI Agent** — 将任意 CLI Agent 接入 Warp Block 模型。
  深度适配:
  - **DeepSeek-TUI**(完成通知、文本通知映射、输入恢复均已打通)
  - **Google Antigravity**(`agy`) — 原生支持
  - **Codex CLI**、**Claude Code** 及其他主流 CLI Agent
  - 通过 OSC9 / OSC777 统一路由至 Zap 通知中心
- **BYOP 多提供商** — 6 种原生协议(OpenAI / OpenAIResp /
  Anthropic / Gemini / Ollama / DeepSeek)显式绑定;任意 OpenAI 兼容
  代理开箱即用。凭据仅存本地。
- **完全去中心化** — 无需 Warp 账号,不强制登录,无云端 Drive /
  Notebook 同步,无云端 Agent 历史。云端代码路径正在分阶段物理移除。
- **Markdown 渲染优化** — AI blocks 内结构化表格渲染,可配置标题
  字号缩放,代码块、表格、列表和中英混排文本稳定性更好。
- **字体渲染算法修复** — CJK 软换行 caret 偏移、小号汉字加粗
  等长期存在的上游渲染瑕疵。
- **增强的编辑器支持** — Vue 文件注入高亮(JS/CSS/TS)和
  Vue SFC 文件块级注释。
- **CLI Agent 设置界面** — 用于配置 CLI Agent 的图形化设置面板。
- **用户消息右对齐** — 聊天记录中用户消息右对齐,对话流更清晰。

## 三步把终端完全掌握在自己手里

**01 · 接入任意提供商**
在设置中填入 Base URL 和 API 密钥 — 任意 OpenAI Chat Completions 兼容端点
开箱即用。凭据仅存本地。

**02 · 编写动态提示词**
基于 minijinja 的模板引擎根据当前工作目录、语言和角色实时渲染系统提示词。

**03 · 即刻使用**
一键切换模型、对话、命令建议和第三方 Agent — 体验与 Warp 一致,但每一层都归你所有。

## 已验证的 AI 提供商

| 提供商 | Base URL | 备注 |
| --- | --- | --- |
| **OpenAI** | `https://api.openai.com/v1` | 原生协议 |
| **Anthropic** | 通过 genai 原生 | Claude 4.x 系列 |
| **DeepSeek** | `https://api.deepseek.com/v1` | thinking + tool calling |
| **Gemini** | 通过 genai 原生 | Google AI Studio |
| **Ollama** | `http://localhost:11434/v1` | 本地推理,无需密钥 |
| **OpenRouter** | `https://openrouter.ai/api/v1` | 聚合网关 |
| **千问 / Groq / Together / LM Studio / 任意 OpenAI 兼容代理** | — | 配置即用 |

## 核心功能

- **BYOP 自定义提供商** — 6 种原生协议,基于
  [genai](https://github.com/jeremychone/rust-genai) 0.6 显式绑定
- **第三方 CLI Agent** — DeepSeek-TUI / Google Antigravity (`agy`) /
  Codex CLI / Claude Code 通过 OSC9 路由至 Blocks 和通知中心
- **SSH 主机管理器** — 在终端内管理 SSH 主机和会话,集成 tmux
- **SFTP 文件浏览器** — 在图形化文件树中浏览和打开远程文件
- **远程代码审查** — 远程 SSH 仓库上的 diff 查看器和审查面板
- **远程 Markdown 预览** — 远程文件可切换渲染预览
- **应用内图片查看器** — 直接在终端内打开本地和远程图片(PNG/JPEG/GIF/SVG/WebP)
- **每窗口独立主题** — 每个窗口独立主题,预览不干扰其他窗口
- **SSE 流式传输** — 增量 Block 渲染,与 Warp 原生路径一致
- **18 个本地工具** — shell / read / edit / search / mcp / drive docs / skills / ask,
  全部本地执行
- **系统提示词模板** — 从 opencode 移植的八套模型提示词
  (default / anthropic / gpt / beast / gemini / kimi / codex / trinity)
- **models.dev 集成** — 可搜索的 Providers 子页面,预置数千条模型条目
- **渲染优化** — 优化后的 Markdown 管线(结构化表格、
  可配置标题字号) + CJK 软换行 / 加粗修复
- **隐私优先** — Cloud Agent / Computer Use / Referral / 遥测
  全部默认关闭
- **Warp 体验保留** — 持续合并上游;Blocks、
  Workflows、AI 命令、Keymaps 和主题全部保留
- **多语言界面** — 简体中文 + 日语 + 英文,社区可扩展
- **内置主题** — VS Code 2026 Dark 等主题
- **Vue SFC 支持** — 注入高亮(JS/CSS/TS)和块级注释
- **Onkey** — 键盘重映射,自定义快捷键

## 我们追求的目标

Zap 想要成为这样的终端:

1. **完全无需中心化服务运行** — 不需要账号,不强制登录,
   不存在"只有云端可达时才能用"的功能。
2. **将 AI 和 Agent 视为开放生态**,而不是单一供应商 — 每个
   主流 LLM 提供商和 CLI Agent 都是一等公民。
3. **让远程工作原生化** — SSH / tmux / SFTP / 远程 diff / 远程
   图片查看都是内置功能,不是后来拼凑的。
4. **配得上全天使用** — 中英混排、Markdown、代码块和字体渲染
   永远不应该成为短板。
5. **与上游 Warp 保持同步** — 受益于 Warp 的工程成果,
   同时保持分支层面的自主方向。

如果你认同这些目标,来帮我们一起完成。

## 从源码构建

```bash
git clone https://github.com/zerx-lab/warp
cd warp
./script/bootstrap   # 平台特定依赖
./script/run         # 构建并运行
./script/presubmit   # fmt / clippy / tests
```

如果偏好裸 `cargo`,**请务必显式指定 OSS 二进制**:

```bash
cargo build --release --bin warp-oss
cargo run   --release --bin warp-oss
```

> 不要不加过滤地运行 `cargo build --release` / `cargo run --release --bin {warp,stable,dev,preview}`
> — 这些入口点(`local.rs` / `stable.rs` / `dev.rs` / `preview.rs`)通过 Warp 私有的
> `warp-channel-config` 二进制加载渠道配置,该二进制位于闭源仓库中。编译可以成功,
> 但生成的二进制在启动时会 panic,提示运行 `./script/install_channel_config`。
> 该脚本克隆一个仅 Warp 员工可访问的 SSH 仓库。Zap 用户只需 `warp-oss` 二进制。

详见 [AGENTS.md](AGENTS.md) 获取仓库代码地图和工程指南。

## 许可证

与上游 Warp 相同:

- `warpui_core` / `warpui` crate — [MIT](LICENSE-MIT)
- 其他全部 — [AGPL-3.0](LICENSE-AGPL)

## 分支与上游同步

`zerx-lab/warp` 维护两条长期分支:

| 分支 | 追踪对象 | 用途 |
| --- | --- | --- |
| `main` | `zerx-lab/warp:main`(默认) | Zap 的主开发线。**所有 PR 都合入这里。** |
| `warp-upstream` | `warpdotdev/warp:master` | 上游 Warp 的纯净镜像,用于拉取新提交。**不含任何分支本地修改。** |

**给贡献者**

PR 请提交到 **`main`**。不要提交到 `warp-upstream`。

**给维护者(有写入权限)**

**不要在 GitHub 网页 UI 上点击 `main` 的"Sync fork"按钮**。这会将整个上游历史直接合并进 Zap 的主线,引发大规模冲突。请通过镜像分支拉取上游变更:

```bash
# 一次性设置
git remote add upstream https://github.com/warpdotdev/warp.git

# 刷新镜像
git checkout warp-upstream
git pull                          # 从 upstream/master 快进
git push origin warp-upstream

# 将选定提交引入 main
git checkout main
git cherry-pick <sha>             # 或适时将 warp-upstream 整体合并
```

## 合作伙伴

<a href="https://github.com/Hmbown/DeepSeek-TUI">
  <img src="assets/DeepSeek-TUI.png" alt="DeepSeek-TUI" width="96" align="left" />
</a>

**[DeepSeek-TUI](https://github.com/Hmbown/DeepSeek-TUI)** — DeepSeek 模型系列的
终端 UI。Zap 提供深度集成:完成通知、OSC9 文本通知映射、输入恢复均已打通,
使 DeepSeek-TUI 在 Zap 中以原生 Block 方式运行。

在任意 Zap 终端中输入 `deepseek` 即可启动 — Block 生命周期、底部状态栏
和通知中心全部开箱即用。

<br clear="left" />

<a href="https://github.com/google/antigravity">
  <img src="assets/agy-icon.png" alt="Google Antigravity" width="96" align="left" />
</a>

**[Google Antigravity](https://github.com/google/antigravity)**(`agy`) — Google 的
CLI Agent,用于编程任务。Zap 提供原生集成,`agy` 以一等公民 Block 形式运行,
通知通过 Zap 通知中心路由。

<br clear="left" />

> **Windows 用户注意(DeepSeek-TUI)** — DeepSeek-TUI 的 `[notifications].method` 默认值为 `auto`,
> 在 Windows 上对于不在其内置白名单(iTerm.app / Ghostty / WezTerm)中的
> `TERM_PROGRAM`,会解析为 `Off`。Zap 标识为 `WarpTerminal`,因此要在
> Zap on Windows 中接收回合完成通知,请在 `~/.deepseek/config.toml` 中添加:
>
> ```toml
> [notifications]
> method = "osc9"
>
> [tui]
> notification_condition = "always"  # 可选:每个回合都通知
> ```

如果你维护 CLI Agent 或终端相关工具,并希望获得类似深度集成,请提交 issue —
我们很乐意接入更多合作伙伴。

## 从 OpenWarp 或 Warp 迁移

如果你在项目改名 Zap 之前就一直在用(那时还叫 **OpenWarp**),
或者你是从上游 **Warp** 切过来的,参见
[docs/migrate-from-warp.zh-CN.md](docs/migrate-from-warp.zh-CN.md) 把设置带过来。

## 路线图

见 [docs/roadmap.zh-CN.md](docs/roadmap.zh-CN.md)。

## 贡献

欢迎社区贡献。完整流程见 [CONTRIBUTING.md](CONTRIBUTING.md)。

提交前,请先[搜索已有 issue](https://github.com/zerx-lab/warp/issues)。
安全漏洞请按 [CONTRIBUTING.md#reporting-security-issues](CONTRIBUTING.md#reporting-security-issues)
私下报告。

## 鸣谢

Zap 站在 Warp 团队和众多开源项目的肩膀上:

[Warp](https://github.com/warpdotdev/warp) · [genai](https://github.com/jeremychone/rust-genai) · [opencode](https://github.com/opencode-ai/opencode) · [models.dev](https://models.dev) · [DeepSeek-TUI](https://github.com/Hmbown/DeepSeek-TUI) · [Google Antigravity](https://github.com/google/antigravity) · [Codex CLI](https://github.com/openai/codex) · [Tokio](https://github.com/tokio-rs/tokio) · [NuShell](https://github.com/nushell/nushell) · [Alacritty](https://github.com/alacritty/alacritty) · [Hyper](https://github.com/hyperium/hyper) · [minijinja](https://github.com/mitsuhiko/minijinja) · [cosmic-text](https://github.com/pop-os/cosmic-text)
