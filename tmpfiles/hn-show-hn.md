# Hacker News — Show HN

**Title** (79 chars):

Show HN: OpenWarp – a Warp terminal fork with no account and bring-your-own LLM

**URL:** https://github.com/TranscriptionFactory/warp

**Text:**

OpenWarp is a community fork of Warp's open-source code that drops the
account requirement and cloud dependency. Paste a base URL and an API key,
or point it at Ollama or LM Studio, and the AI features work. Credentials,
conversations, and agent history stay on your machine. It speaks six
native protocols (OpenAI Chat and Responses, Anthropic, Gemini, DeepSeek,
Ollama) and works with any OpenAI-compatible proxy.

Lineage: Warp, Inc. open-sourced their client. zerx-lab forked it to open
up the provider layer; you may remember that fork from
https://news.ycombinator.com/item?id=47970622. This repo forks the zerx-lab
fork, which has since taken its own branding and direction, and continues
under the OpenWarp name. We trade patches with them; several of the fixes
below landed there too.

New since that earlier post:

- Binary releases: macOS (arm64 and Intel dmg), Linux (AppImage, deb, rpm,
  static CLI tarballs including aarch64), and a Windows installer. You can
  try it without building from source.

- We delete cloud code paths from the tree instead of switching them off.
  Billing, referral, cloud sync, and the cloud-agent subsystems are gone.

- Kitty graphics protocol on macOS and Linux, including Unicode
  placeholders (so images work through tmux) and animation.

- Remote work over SSH: the code-review panel diffs, commits, pushes, and
  opens PRs against a repo on the remote host. Images and Markdown files
  on the remote open in-app. The SSH host manager and SFTP browser are
  zerx-lab's work.

- Security hardening: SSRF guards on webfetch and MCP OAuth discovery, and
  shell-escaping fixes. The MCP client no longer sends your credentials to
  redirect targets.

- Third-party CLI agents (Claude Code, Codex, and others) run inside Warp's
  block model.

- We port fixes from Warp's repo: 53 across the last two syncs.

On the naming question from the last thread: we are not affiliated with
Warp, Inc. Warp did open-source their client. This fork exists because
upstream requires an account, on a paid tier, to use your own model keys.
We kept upstream's dual AGPL-3.0/MIT license.

Still early; expect rough edges. Feedback and issues welcome.

---

# Reddit — r/LocalLLaMA (primary)

**Title:**

OpenWarp: a fork of the Warp terminal with no account required — bring your
own keys or run fully local with Ollama

**Body:**

Warp open-sourced their terminal client a while back, but the upstream build
still requires an account (and a paid plan) to use your own LLM providers.
OpenWarp is a community fork that removes that: no login, no cloud calls, an
open provider layer. It builds on the zerx-lab community fork, which has
since taken its own branding; this repo continues under the OpenWarp name,
and we trade patches with them. We also port fixes from Warp's repo: 53
across the last two syncs.

In practice:

- **Local models work out of the box.** Point it at Ollama
  (`http://localhost:11434/v1`) or LM Studio; no API key needed.
- **Any OpenAI-compatible endpoint works**, with native protocol support
  for OpenAI, Anthropic, Gemini, and DeepSeek. We verified OpenRouter,
  Groq, and Together.
- **Keys stay on your machine** in a local config file, along with
  conversations and agent history. We delete the cloud code paths from the
  codebase instead of toggling them off.
- **You edit the system prompt yourself.** It is a minijinja template;
  upstream assembles theirs server-side.
- **Prompt caching is on.** Anthropic requests carry cache breakpoints and
  OpenAI requests carry a stable cache key. Per-provider extra headers
  reach title generation and suggestions too, so gateways that
  authenticate with a custom header work.
- **MCP credentials stay with the server you gave them to.** The MCP
  client refuses redirects, and OAuth discovery blocks loopback and
  private addresses.
- **CLI agents run in the terminal UI.** Claude Code, Codex, and others run
  inside Warp's block model. The terminal supports the kitty graphics
  protocol on macOS and Linux, so tools that draw images inline work.
- **Remote boxes are covered.** A built-in SSH host manager and SFTP
  browser, plus a code-review panel that diffs, commits, pushes, and opens
  PRs on the remote host.

Binary releases cover macOS (Apple Silicon and Intel), Linux
(AppImage/deb/rpm), and Windows.

Repo: https://github.com/TranscriptionFactory/warp
Releases: https://github.com/TranscriptionFactory/warp/releases

Not affiliated with Warp, Inc. We kept upstream's license (AGPL-3.0/MIT).
Early days; bug reports welcome.

---

# Reddit — r/commandline (variant)

**Title:**

OpenWarp – a de-clouded fork of the Warp terminal: no account, BYO AI
provider (or none), built-in SSH manager and SFTP browser

**Body:**

OpenWarp is a fork of Warp's open-source client with the account requirement
and cloud dependency stripped out. It keeps Warp's terminal fundamentals
(blocks, editor-style input, command palette) and drops the mandatory login.

- No account, no vendor-cloud telemetry. AI is optional and bring-your-own:
  any OpenAI-compatible endpoint, or Ollama for local models.
- Kitty graphics protocol on macOS and Linux, including Unicode
  placeholders (images work through tmux) and animation
- Built-in SSH host manager (with tmux integration) and SFTP file browser
- Code-review panel that diffs, commits, pushes, and opens PRs on remote
  hosts over SSH
- In-app image viewer and Markdown preview, local and over SFTP
- Drag a tab onto a pane to split it, move a pane to its own tab, drag
  tabs between windows
- Per-window themes; Warp's blocks, workflows, and keybindings preserved
- Security fixes: SSRF guards on webfetch and MCP OAuth discovery, shell
  escaping, and no MCP credentials sent to redirect targets
- Fixes ported from Warp's repo: 53 across the last two syncs
- Binaries for macOS/Linux/Windows: https://github.com/TranscriptionFactory/warp/releases

AGPL-3.0/MIT, unchanged from upstream. Not affiliated with Warp, Inc.
Pre-release quality; feedback welcome.
