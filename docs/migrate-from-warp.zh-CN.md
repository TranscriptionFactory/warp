# 迁移设置到 OpenWarp

[English](./migrate-from-warp.md) · [日本語](./migrate-from-warp.ja.md)

本文给希望把**设置类配置**(自定义快捷键、主题、工作流、MCP 配置等)从历史安装
带到 OpenWarp 的用户。

可能的"源端"有两种,**两者的安全等级不同**,本文分两节说明。如果两边都有,
**请先迁 Zap,再考虑迁 Warp**:

1. **Zap** —— 本项目此前使用的品牌名。
2. **上游 [Warp](https://github.com/warpdotdev/warp)** —— OpenWarp 分叉自的
   原项目。

本文刻意**不覆盖**命令历史、SQLite 数据库、Drive 对象与任何凭证。它们要么绑定
本机(Keychain / DPAPI / libsecret),要么与 schema 强耦合,跨分支复制并不安全。

---

## 磁盘状态的布局方式

OpenWarp(以及它之前的 Zap 与上游 Warp)把磁盘状态分成**三类目录**:

- **config** —— `settings.toml`、`keybindings.yaml`
- **data** —— `themes/`、`workflows/`、`launch_configurations/`、`tab_configs/`
- **home 点目录** —— `.mcp.json`、`skills/`

macOS 上三类目录合并在同一个 home 点目录下(`~/.warp/`、`~/.zap/` 或
`~/.openwarp/`)。Linux 与 Windows 上它们位于**三个不同位置** —— Linux 遵循
XDG 约定,Windows 遵循 `directories` crate 布局。下面的迁移脚本会按平台把每个
文件放到正确的目标位置。

### OpenWarp 目标路径

| 类别 | macOS | Linux | Windows |
|---|---|---|---|
| config | `~/.openwarp/` | `${XDG_CONFIG_HOME:-~/.config}/openwarp/` | `%LOCALAPPDATA%\openwarp\OpenWarp\config\` |
| data | `~/.openwarp/` | `${XDG_DATA_HOME:-~/.local/share}/openwarp/` | `%APPDATA%\openwarp\OpenWarp\data\` |
| home 点目录 | `~/.openwarp/` | `~/.openwarp/` | `%USERPROFILE%\.openwarp\` |

### Zap 源路径

| 类别 | macOS | Linux | Windows |
|---|---|---|---|
| config | `~/.zap/` | `${XDG_CONFIG_HOME:-~/.config}/zap/` | `%LOCALAPPDATA%\zap\Zap\config\` |
| data | `~/.zap/` | `${XDG_DATA_HOME:-~/.local/share}/zap/` | `%APPDATA%\zap\Zap\data\` |
| home 点目录 | `~/.zap/` | `~/.zap/` | `%USERPROFILE%\.zap\` |

### 上游 Warp 源路径

| 类别 | macOS | Linux | Windows |
|---|---|---|---|
| config | `~/.warp/` | `${XDG_CONFIG_HOME:-~/.config}/warp-terminal/` | `%LOCALAPPDATA%\warp\Warp-Terminal\config\` |
| data | `~/.warp/` | `${XDG_DATA_HOME:-~/.local/share}/warp-terminal/` | `%APPDATA%\warp\Warp-Terminal\data\` |
| home 点目录 | `~/.warp/` | `~/.warp/` | `%USERPROFILE%\.warp\` |

> Linux 目录名 `warp-terminal` 与 Linux 包名一致(如 Debian/Ubuntu 上的
> `/opt/warpdotdev/warp-terminal/`)。Windows 的组织目录可能因 Warp 打包方式而
> 异;如果找不到 `%APPDATA%\warp\Warp-Terminal`(或
> `%LOCALAPPDATA%\warp\Warp-Terminal`),请检查你的 Warp 安装实际使用的
> `%APPDATA%` / `%LOCALAPPDATA%` 位置。

---

## 1. 从 Zap 迁移(现有用户的推荐路径)

Zap **就是** OpenWarp —— 本项目在恢复 OpenWarp 名称之前以 Zap 品牌发布。改名
只改动了标识符与磁盘路径,**配置文件格式与 schema 没有变化**,因此下列文件可以
原样复制。

### 要复制的文件

| 文件或目录 | 类别 | 控制内容 |
|---|---|---|
| `settings.toml` | config | 公开设置(TOML 格式的设置文件)。 |
| `keybindings.yaml` | config | 自定义快捷键。 |
| `themes/` | data | 自定义主题。 |
| `workflows/` | data | 自定义工作流。 |
| `launch_configurations/` | data | 启动配置。 |
| `tab_configs/` | data | 标签页配置。 |
| `.mcp.json` | home 点目录 | MCP 服务器配置。 |
| `skills/` | home 点目录 | Agent skills。 |

### 步骤

> 复制前请先退出 OpenWarp,避免有进程占用文件。

**macOS**

```sh
mkdir -p "$HOME/.openwarp"
for f in settings.toml keybindings.yaml themes workflows launch_configurations tab_configs skills .mcp.json; do
  if [ -e "$HOME/.zap/$f" ] && [ ! -e "$HOME/.openwarp/$f" ]; then
    cp -R "$HOME/.zap/$f" "$HOME/.openwarp/$f"
  fi
done
```

**Linux**

```sh
src_config="${XDG_CONFIG_HOME:-$HOME/.config}/zap"
src_data="${XDG_DATA_HOME:-$HOME/.local/share}/zap"
src_home="$HOME/.zap"

dst_config="${XDG_CONFIG_HOME:-$HOME/.config}/openwarp"
dst_data="${XDG_DATA_HOME:-$HOME/.local/share}/openwarp"
dst_home="$HOME/.openwarp"
mkdir -p "$dst_config" "$dst_data" "$dst_home"

copy() {
  if [ -e "$1/$3" ] && [ ! -e "$2/$3" ]; then
    cp -R "$1/$3" "$2/$3"
  fi
}

copy "$src_config" "$dst_config" settings.toml
copy "$src_config" "$dst_config" keybindings.yaml
copy "$src_data"   "$dst_data"   themes
copy "$src_data"   "$dst_data"   workflows
copy "$src_data"   "$dst_data"   launch_configurations
copy "$src_data"   "$dst_data"   tab_configs
copy "$src_home"   "$dst_home"   .mcp.json
copy "$src_home"   "$dst_home"   skills
```

**Windows(PowerShell)**

```powershell
$src_config = "$env:LOCALAPPDATA\zap\Zap\config"
$src_data   = "$env:APPDATA\zap\Zap\data"
$src_home   = "$env:USERPROFILE\.zap"

$dst_config = "$env:LOCALAPPDATA\openwarp\OpenWarp\config"
$dst_data   = "$env:APPDATA\openwarp\OpenWarp\data"
$dst_home   = "$env:USERPROFILE\.openwarp"
New-Item -ItemType Directory -Force -Path $dst_config, $dst_data, $dst_home | Out-Null

function Copy-IfMissing($srcDir, $dstDir, $name) {
  $from = Join-Path $srcDir $name
  $to   = Join-Path $dstDir $name
  if ((Test-Path $from) -and -not (Test-Path $to)) {
    Copy-Item -Path $from -Destination $to -Recurse
  }
}

Copy-IfMissing $src_config $dst_config settings.toml
Copy-IfMissing $src_config $dst_config keybindings.yaml
Copy-IfMissing $src_data   $dst_data   themes
Copy-IfMissing $src_data   $dst_data   workflows
Copy-IfMissing $src_data   $dst_data   launch_configurations
Copy-IfMissing $src_data   $dst_data   tab_configs
Copy-IfMissing $src_home   $dst_home   .mcp.json
Copy-IfMissing $src_home   $dst_home   skills
```

`[ ! -e ... ]` / `-not (Test-Path $to)` 守卫可避免覆盖你在 OpenWarp 中已经
设置的内容;如果希望 Zap 的值优先,删掉守卫即可。

确认 OpenWarp 一切正常后,可以删除上面的 Zap 目录以回收磁盘空间 —— 它们已不再
被任何程序使用。

---

## 2. 从上游 Warp 迁移

上游 Warp 是独立产品,拥有自己的磁盘身份(见上文"上游 Warp 源路径"表)。
OpenWarp 以 `Oss` channel 构建,拥有自己的 app ID(`dev.openwarp.OpenWarp`)
与独立的各平台布局。两个安装互相看不到对方的文件 —— 这也正是把你的 Warp
账号 / 云端状态挡在 OpenWarp 之外的机制。

下列文本格式文件的 schema 稳定且兼容,复制是安全的。**其他状态则不然** ——
Warp 独立于 OpenWarp 演进,二进制 / 私有存储可能绑定 Warp 的认证与 bundle
身份。

### 要复制的内容

与上一节相同的八项:

| 文件或目录 | 类别 | 控制内容 |
|---|---|---|
| `settings.toml` | config | 公开设置(TOML 格式的设置文件)。 |
| `keybindings.yaml` | config | 自定义快捷键。 |
| `themes/` | data | 自定义主题。 |
| `workflows/` | data | 自定义工作流。 |
| `launch_configurations/` | data | 启动配置。 |
| `tab_configs/` | data | 标签页配置。 |
| `.mcp.json` | home 点目录 | MCP 服务器配置。 |
| `skills/` | home 点目录 | Agent skills。 |

### **不要**复制的内容

- **`user_preferences.json`** —— 位于
  `~/Library/Application Support/dev.warp.Warp/`(macOS)或 Linux/Windows
  对应状态目录下的私有存储。混杂了用户偏好、认证 token、机器绑定 ID 与云端
  缓存状态。复制它可能泄露身份并干扰 OpenWarp 的认证状态。OpenWarp 的默认值
  已经对隐私友好。
- **`warp.sqlite`**(及其 `-wal` / `-shm` 伴随文件)—— schema 与上游 Warp
  耦合,不保证与 OpenWarp 的迁移兼容。
- **Keychain / DPAPI / libsecret 条目** —— 绑定 Warp 的 bundle / service
  名称,对 OpenWarp 无用。

### 步骤

> 复制前请同时退出 Warp 与 OpenWarp。

**macOS**

```sh
mkdir -p "$HOME/.openwarp"
for f in settings.toml keybindings.yaml themes workflows launch_configurations tab_configs skills .mcp.json; do
  if [ -e "$HOME/.warp/$f" ] && [ ! -e "$HOME/.openwarp/$f" ]; then
    cp -R "$HOME/.warp/$f" "$HOME/.openwarp/$f"
  fi
done
```

**Linux**

```sh
src_config="${XDG_CONFIG_HOME:-$HOME/.config}/warp-terminal"
src_data="${XDG_DATA_HOME:-$HOME/.local/share}/warp-terminal"
src_home="$HOME/.warp"

dst_config="${XDG_CONFIG_HOME:-$HOME/.config}/openwarp"
dst_data="${XDG_DATA_HOME:-$HOME/.local/share}/openwarp"
dst_home="$HOME/.openwarp"
mkdir -p "$dst_config" "$dst_data" "$dst_home"

copy() {
  if [ -e "$1/$3" ] && [ ! -e "$2/$3" ]; then
    cp -R "$1/$3" "$2/$3"
  fi
}

copy "$src_config" "$dst_config" settings.toml
copy "$src_config" "$dst_config" keybindings.yaml
copy "$src_data"   "$dst_data"   themes
copy "$src_data"   "$dst_data"   workflows
copy "$src_data"   "$dst_data"   launch_configurations
copy "$src_data"   "$dst_data"   tab_configs
copy "$src_home"   "$dst_home"   .mcp.json
copy "$src_home"   "$dst_home"   skills
```

**Windows(PowerShell)**

```powershell
$src_config = "$env:LOCALAPPDATA\warp\Warp-Terminal\config"
$src_data   = "$env:APPDATA\warp\Warp-Terminal\data"
$src_home   = "$env:USERPROFILE\.warp"

$dst_config = "$env:LOCALAPPDATA\openwarp\OpenWarp\config"
$dst_data   = "$env:APPDATA\openwarp\OpenWarp\data"
$dst_home   = "$env:USERPROFILE\.openwarp"
New-Item -ItemType Directory -Force -Path $dst_config, $dst_data, $dst_home | Out-Null

function Copy-IfMissing($srcDir, $dstDir, $name) {
  $from = Join-Path $srcDir $name
  $to   = Join-Path $dstDir $name
  if ((Test-Path $from) -and -not (Test-Path $to)) {
    Copy-Item -Path $from -Destination $to -Recurse
  }
}

Copy-IfMissing $src_config $dst_config settings.toml
Copy-IfMissing $src_config $dst_config keybindings.yaml
Copy-IfMissing $src_data   $dst_data   themes
Copy-IfMissing $src_data   $dst_data   workflows
Copy-IfMissing $src_data   $dst_data   launch_configurations
Copy-IfMissing $src_data   $dst_data   tab_configs
Copy-IfMissing $src_home   $dst_home   .mcp.json
Copy-IfMissing $src_home   $dst_home   skills
```

你原本的 Warp 数据不会被触碰 —— Warp 自身继续正常工作。

---

## 验证

启动 OpenWarp。主题选择器里应能看到你的自定义主题,快捷键编辑器里是你的键位,
工作流启动器里是你的工作流。设置界面的值应与 `settings.toml` 中的内容一致。

如果哪里不对,问题一定出在上述八个文件之一 —— 用文本编辑器打开检查,或直接
删除让 OpenWarp 回退到默认值。

## 回滚

本指南中没有任何破坏性操作:复制的每个文件 OpenWarp 都会在下次启动时从默认值
重建。要完全撤销:

```sh
# macOS
rm -rf ~/.openwarp
```

```sh
# Linux
rm -rf "${XDG_CONFIG_HOME:-$HOME/.config}/openwarp"
rm -rf "${XDG_DATA_HOME:-$HOME/.local/share}/openwarp"
rm -rf "$HOME/.openwarp"
```

```powershell
# Windows
Remove-Item -Recurse -Force "$env:APPDATA\openwarp"
Remove-Item -Recurse -Force "$env:LOCALAPPDATA\openwarp"
Remove-Item -Recurse -Force "$env:USERPROFILE\.openwarp"
```

Zap 与 Warp 的源目录在本指南中始终不会被触碰。
