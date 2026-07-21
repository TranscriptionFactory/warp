<div align="center">

<img src="assets/OpenWarp-logo.svg" alt="OpenWarp" width="128" />

# OpenWarp

**完全去中心化的终端 —— 你的 AI、你的 Agent、你的密钥、你的机器。**

OpenWarp 是 [Warp](https://github.com/warpdotdev/warp) 的社区分支,在保留完整
Warp 终端体验的同时,**剥离了 Warp 对云端的强制依赖**。它完全开放 AI provider
层,支持接入任意第三方 CLI Agent,内置 SSH 主机管理器、远程文件浏览与远程
code review,并修复了一系列上游渲染问题 —— 所有凭证、会话与 Agent 历史都只保留
在你自己的机器上。

[下载](https://github.com/TranscriptionFactory/warp/releases/latest) · [English](./README.md) · [日本語](./README.ja.md) · [上游 Warp](https://www.warp.dev)

> 项目处于早期开发阶段,目前仅提供预发布构建,可能存在粗糙之处。**与 Warp, Inc. 无关联。**

</div>

---

## 为什么选择 OpenWarp

上游 Warp 将 AI、账号、同步与 Agent 历史都绑定在 Warp 云端。OpenWarp 完全打开了
这一层,并**加入了上游客户端不具备的能力**:

| | 上游 Warp | OpenWarp |
| --- | --- | --- |
| 云端依赖 | 强依赖 Warp 后端(认证 / Drive / 历史 / Agent) | **完全去中心化,无强制云端调用** |
| AI provider | 仅 Warp 网关 | **任意 OpenAI 兼容端点 + 6 种原生协议** |
| 第三方 Agent | 仅内置 Warp Agent | **任意 CLI Agent —— DeepSeek-TUI / Codex / Claude Code / Agy 均已接入** |
| SSH 管理 | 无内置 | **内置 SSH 主机管理器(连接 / 配置 / tmux)** |
| 远程文件浏览 | 无内置 | **内置 SFTP 文件浏览器** |
| 远程 code review | 无 | **远程 SSH 主机上的 diff + code review 面板** |
| Markdown 渲染 | 上游基线 | **调优的 MD 渲染管线 —— 代码块、表格、中西文混排;远程文件渲染预览** |
| 图片查看 | 无内置 | **应用内图片查看器(本地 + 远程)** |
| 主题 | 单一全局主题 | **按窗口独立主题** |
| 字体渲染 | 上游 cosmic_text 默认行为 | **CJK 软换行光标 + 加粗次像素修复** |
| 凭证 | 云端账号 | **本地配置文件,绝不离开设备** |
| 系统提示词 | 服务端拼装,不透明 | **minijinja 模板,完全可编辑** |
| 界面语言 | 英语 | **英语 + 简体中文 + 日语,可扩展** |
| Cloud Agent / Computer Use | 默认开启 | **默认关闭(并在逐步物理移除)** |
| Blocks / Workflows / 键位 | 保留 | 完整保留,持续同步 |
| 许可证 | AGPL-3.0 / MIT 双许可 | 与上游一致 |

## 上游 Warp 不支持、OpenWarp 支持的功能

以下是 OpenWarp 在分支之上新增的独有能力:

- **SSH 主机管理器** —— 在终端内直接连接、配置和管理 SSH 主机与会话(带 tmux
  集成),无需外部切换工具。
- **SFTP 文件浏览器** —— 远程主机的图形化文件浏览器,像本地文件一样浏览和打开
  远程文件。
- **远程 code review** —— diff 查看器与 code review 面板跨 SSH 会话工作。在终端
  内 review 远程仓库的改动、stage hunk、浏览 diff。
- **远程 Markdown 预览** —— 远程主机上的文件同样可以切换渲染后的 Markdown 预览。
- **应用内图片查看器** —— 在终端中直接打开图片(PNG、JPEG、GIF、SVG、WebP),
  本地文件与 SFTP 远程文件均可。
- **按窗口主题** —— 每个窗口设置不同主题;主题预览只作用于当前窗口。
- **第三方 CLI Agent** —— 把任意 CLI Agent 纳入 Warp Block 模型。一等公民适配:
  - **DeepSeek-TUI**(完成通知、文本通知映射、输入恢复全部打通)
  - **Google Antigravity**(`agy`)—— 原生支持
  - **Codex CLI**、**Claude Code** 及其他主流 CLI Agent
  - 通过 OSC9 / OSC777 统一路由到 OpenWarp 的通知中心
- **多 provider BYOP** —— 6 种原生协议(OpenAI / OpenAIResp / Anthropic /
  Gemini / Ollama / DeepSeek)显式绑定;任意 OpenAI 兼容代理开箱即用。凭证
  保留在本地。
- **完全去中心化** —— 无 Warp 账号、无强制登录、无云端 Drive / Notebook 同步、
  无云端 Agent 历史。云端代码路径正在分阶段物理移除。
- **Markdown 渲染改进** —— AI block 内结构化表格渲染、可配置标题缩放,代码块、
  表格、列表与中英混排文本的稳定性提升。
- **字体渲染算法修复** —— CJK 软换行光标偏移、小号中文加粗等长期存在的上游
  渲染问题。
- **编辑器增强** —— Vue 文件注入高亮(JS/CSS/TS)与 Vue SFC 块级注释。
- **CLI Agent 设置界面** —— 配置 CLI Agent 的图形化设置面板。
- **用户消息右对齐** —— 会话记录中用户消息靠右显示,对话脉络更清晰。

## 三步把终端完全握在自己手里

**01 · 接入任意 provider**
在设置中粘贴 Base URL 和 API key —— 任何兼容 OpenAI Chat Completions 的端点
开箱即用。凭证只存储在本地。

**02 · 编写动态提示词**
minijinja 驱动的模板引擎根据当前工作目录、语言与角色实时渲染系统提示词。

**03 · 立即使用**
一键切换模型、会话、命令建议与第三方 Agent —— 体验与 Warp 完全一致,但每一层
都属于你。

## 已验证的 AI provider

| Provider | Base URL | 说明 |
| --- | --- | --- |
| **OpenAI** | `https://api.openai.com/v1` | 原生协议 |
| **Anthropic** | genai 原生 | Claude 4.x 系列 |
| **DeepSeek** | `https://api.deepseek.com/v1` | thinking + tool calling |
| **Gemini** | genai 原生 | Google AI Studio |
| **Ollama** | `http://localhost:11434/v1` | 本地推理,无需 key |
| **OpenRouter** | `https://openrouter.ai/api/v1` | 聚合网关 |
| **Qwen / Groq / Together / LM Studio / 任意 OpenAI 兼容代理** | — | 配置即用 |

## 核心特性

- **BYOP 自定义 provider** —— 基于 [genai](https://github.com/jeremychone/rust-genai) 0.6
  显式绑定 6 种原生协议
- **第三方 CLI Agent** —— DeepSeek-TUI / Google Antigravity(`agy`)/ Codex CLI /
  Claude Code 经 OSC9 路由进 Block 与通知中心
- **SSH 主机管理器** —— 在终端内管理 SSH 主机与会话,带 tmux 集成
- **SFTP 文件浏览器** —— 图形化文件树浏览和打开远程文件
- **远程 code review** —— 远程 SSH 仓库上的 diff 查看器与 review 面板
- **远程 Markdown 预览** —— 远程文件的渲染预览开关
- **应用内图片查看器** —— 在终端中直接打开本地和远程图片(PNG/JPEG/GIF/SVG/WebP)
- **按窗口主题** —— 每个窗口独立主题,预览不干扰其他窗口
- **SSE 流式输出** —— 与 Warp 第一方路径一致的增量 block 渲染
- **18 个本地工具** —— shell / read / edit / search / mcp / drive docs / skills / ask,
  全部本地执行
- **系统提示词模板** —— 从 opencode 移植的八套模型家族提示词
  (default / anthropic / gpt / beast / gemini / kimi / codex / trinity)
- **models.dev 集成** —— 可搜索的 Providers 子页面,预载数千条模型条目
- **渲染改进** —— 调优的 Markdown 管线(结构化表格、可配置标题缩放)+ CJK
  软换行 / 加粗修复
- **隐私优先** —— Cloud Agent / Computer Use / Referral / 遥测默认全部关闭
- **保留 Warp 体验** —— 与上游持续合并;Blocks、Workflows、AI 命令、键位与
  主题全部保留
- **本地化界面** —— 简体中文 + 日语 + 英语,社区可扩展
- **内置主题** —— 内含 VS Code 2026 Dark 等主题
- **Vue SFC 支持** —— 注入高亮(JS/CSS/TS)与块级注释
- **Onkey** —— 自定义键位的键盘重映射

## 我们的目标

OpenWarp 想成为这样一款终端:

1. **完全脱离中心化服务运行** —— 无账号、无强制登录,没有"只有连上云端才能用"
   的功能。
2. **把 AI 与 Agent 当作开放生态**,而非单一厂商 —— 每个主流 LLM provider 和
   CLI Agent 都是一等公民。
3. **让远程工作原生化** —— SSH / tmux / SFTP / 远程 diff / 远程图片查看全部
   内置,而非外挂。
4. **配得上全天候使用** —— 中西文混排、Markdown、代码块与字体渲染永远不该是
   短板。
5. **与上游 Warp 保持同步** —— 享受 Warp 的工程投入,同时保持分支方向上的
   自主权。

如果你认同这些目标,欢迎来一起完成它。

## 下载

每个版本的预编译安装包都在
[Releases 页面](https://github.com/TranscriptionFactory/warp/releases/latest):

| 平台 | 资源 |
| --- | --- |
| macOS(Apple Silicon) | `OpenWarp-arm64.dmg` |
| macOS(Intel) | `OpenWarp-intel.dmg` |
| Linux(任意发行版) | `OpenWarp-x86_64.AppImage` |
| Debian / Ubuntu | `openwarp_<version>_amd64.deb` |
| Fedora / RHEL 8+ | `openwarp-<version>.x86_64.rpm` |
| Windows x64 | `OpenWarpSetup.exe` |
| 无头 CLI(macOS / Linux,x86_64 + aarch64) | `openwarp-<os>-<arch>.tar.gz` |

- **AppImage**:`chmod +x OpenWarp-x86_64.AppImage && ./OpenWarp-x86_64.AppImage`
- **deb / rpm**:`sudo apt install ./openwarp_*_amd64.deb` · `sudo dnf install ./openwarp-*.x86_64.rpm`
- **macOS**:构建未签名;如遇 Gatekeeper 拦截,见下方
  [macOS Gatekeeper](#macos-gatekeeper)。
- `.tar.gz` 是 `openwarp-oss` 的静态 CLI 构建;OpenWarp 在向远程 SSH 主机安装
  自身时也会自动拉取这些文件。

### macOS Gatekeeper

如果 macOS 提示 OpenWarp 已损坏,清除隔离标记即可:

```bash
xattr -cr /Applications/OpenWarp.app
```

也可以打开**系统设置 → 隐私与安全性**,选择**仍要打开**。

## 从源码构建

```bash
git clone https://github.com/TranscriptionFactory/warp
cd warp
./script/bootstrap   # 平台相关依赖
./script/run         # 构建并运行
./script/presubmit   # fmt / clippy / 测试
```

如果偏好直接使用 `cargo`,**务必显式指定 OSS 二进制**:

```bash
cargo build --release --bin openwarp-oss
cargo run   --release --bin openwarp-oss
```

> 不要不带过滤地运行 `cargo build --release` / `cargo run --release --bin {warp,stable,dev,preview}`
> —— 这些入口(`local.rs` / `stable.rs` / `dev.rs` / `preview.rs`)通过 Warp 私有的
> `warp-channel-config` 二进制加载 channel 配置,该二进制位于闭源仓库。编译可以通过,
> 但生成的可执行文件启动时会 panic,提示运行 `./script/install_channel_config`,而该脚本
> 克隆的 SSH 仓库只有 Warp 员工能访问。OpenWarp 用户只需要 `openwarp-oss` 二进制。

代码地图与工程指南见 [AGENTS.md](AGENTS.md)。

## 许可证

与上游 Warp 一致:

- `warpui_core` / `warpui` crate —— [MIT](LICENSE-MIT)
- 其余部分 —— [AGPL-3.0](LICENSE-AGPL)

## 分支与上游同步

`TranscriptionFactory/warp` 维护两条长期分支:

| 分支 | 跟踪 | 用途 |
| --- | --- | --- |
| `main` | `TranscriptionFactory/warp:main`(默认) | OpenWarp 的主开发线。**所有 PR 都提交到这里。** |
| `warp-upstream` | `warpdotdev/warp:master` | 上游 Warp 的纯净镜像,用于拉取新 commit。**不含任何分支本地改动。** |

**贡献者**

PR 请提交到 **`main`**,不要提交到 `warp-upstream`。

**维护者(有写权限)**

**不要在 GitHub 网页端对 `main` 点击 "Sync fork" 按钮** —— 那会把上游完整历史直接
合入 OpenWarp 主线并触发大规模冲突。请通过镜像分支拉取上游改动:

```bash
# 一次性设置
git remote add upstream https://github.com/warpdotdev/warp.git

# 刷新镜像
git checkout warp-upstream
git pull                          # 从 upstream/master fast-forward
git push origin warp-upstream

# 把选定的 commit 带入 main
git checkout main
git cherry-pick <sha>             # 需要整体同步时也可 merge warp-upstream
```

## 合作伙伴

<a href="https://github.com/Hmbown/DeepSeek-TUI">
  <img src="assets/DeepSeek-TUI.png" alt="DeepSeek-TUI" width="96" align="left" />
</a>

**[DeepSeek-TUI](https://github.com/Hmbown/DeepSeek-TUI)** —— DeepSeek 模型家族的
终端 UI。OpenWarp 提供一等公民集成:完成通知、OSC9 文本通知映射与输入恢复全部
打通,DeepSeek-TUI 作为原生 Block 在 OpenWarp 中运行。

在任意 OpenWarp 终端中运行 `deepseek` 即可启动 —— Block 生命周期、底部状态栏与
通知中心开箱即用。

<br clear="left" />

**[Google Antigravity](https://github.com/google/antigravity)**(`agy`)—— Google 的
编程 CLI Agent。OpenWarp 提供原生集成,`agy` 作为一等公民 Block 运行,通知经由
OpenWarp 的通知中心路由。

<br clear="left" />

> **DeepSeek-TUI Windows 提示** —— DeepSeek-TUI 的 `[notifications].method` 默认为 `auto`,
> 在 Windows 上对内置白名单(iTerm.app / Ghostty / WezTerm)之外的 `TERM_PROGRAM`
> 会解析为 `Off`。OpenWarp 的标识是 `WarpTerminal`,因此要在 Windows 的 OpenWarp 中
> 收到回合完成通知,请在 `~/.deepseek/config.toml` 中添加:
>
> ```toml
> [notifications]
> method = "osc9"
>
> [tui]
> notification_condition = "always"  # 可选:每个回合都通知
> ```

如果你维护某个 CLI Agent 或终端周边工具,希望获得同样的一等公民集成,欢迎开
issue —— 我们乐于接入更多伙伴。

## 从 Zap 或 Warp 迁移

如果你此前使用的是本项目的 **Zap** 品牌版本,或从上游 **Warp** 迁移而来,请参阅
[docs/migrate-from-warp.md](docs/migrate-from-warp.md) 迁移你的设置。

## 路线图

见 [docs/roadmap.md](docs/roadmap.md)。

## 参与贡献

欢迎社区贡献。完整流程见 [CONTRIBUTING.md](CONTRIBUTING.md)。

提交 issue 前请先[搜索既有 issue](https://github.com/TranscriptionFactory/warp/issues)。
安全漏洞请按
[CONTRIBUTING.md#reporting-security-issues](CONTRIBUTING.md#reporting-security-issues)
私下报告。

## 致谢

OpenWarp 站在 Warp 团队与众多开源项目的肩膀上:

[Warp](https://github.com/warpdotdev/warp) · [genai](https://github.com/jeremychone/rust-genai) · [opencode](https://github.com/opencode-ai/opencode) · [models.dev](https://models.dev) · [DeepSeek-TUI](https://github.com/Hmbown/DeepSeek-TUI) · [Google Antigravity](https://github.com/google/antigravity) · [Codex CLI](https://github.com/openai/codex) · [Tokio](https://github.com/tokio-rs/tokio) · [NuShell](https://github.com/nushell/nushell) · [Alacritty](https://github.com/alacritty/alacritty) · [Hyper](https://github.com/hyperium/hyper) · [minijinja](https://github.com/mitsuhiko/minijinja) · [cosmic-text](https://github.com/pop-os/cosmic-text)
