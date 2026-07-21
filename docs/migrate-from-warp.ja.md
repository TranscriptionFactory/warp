# OpenWarp への設定の移行

[English](./migrate-from-warp.md) · [简体中文](./migrate-from-warp.zh-CN.md)

このガイドは、**設定系の構成**(カスタムキーバインド・テーマ・ワークフロー・
MCP 設定など)を以前のインストールから OpenWarp へ引き継ぎたい方向けです。

移行元として想定されるのは 2 つあり、**両者では安全性プロファイルが異なる**ため、
本書では別々のセクションで扱います。両方該当する場合は、**まず Zap から
移行してから** Warp の移行を検討してください:

1. **Zap** —— このプロジェクトが以前使用していたブランド名。
2. **上流 [Warp](https://github.com/warpdotdev/warp)** —— OpenWarp の
   フォーク元プロジェクト。

このガイドは、コマンド履歴・SQLite データベース・Drive オブジェクト・認証情報を
意図的に**扱いません**。これらはマシンに紐付くストア(Keychain / DPAPI /
libsecret)か、スキーマと強く結合したストアにあり、フォーク間でコピーするのは
安全ではありません。

---

## ディスク上の状態のレイアウト

OpenWarp(およびその前身である Zap と上流 Warp)は、ディスク上の状態を
**3 種類のディレクトリ**に分割します:

- **config** —— `settings.toml`、`keybindings.yaml`
- **data** —— `themes/`、`workflows/`、`launch_configurations/`、`tab_configs/`
- **ホームドットファイル** —— `.mcp.json`、`skills/`

macOS では 3 種類すべてが単一のホームドットディレクトリ(`~/.warp/`、`~/.zap/`、
または `~/.openwarp/`)に集約されます。Linux と Windows では**3 つの異なる場所**に
分かれ、Linux は XDG 規約、Windows は `directories` クレートのレイアウトに
従います。以下の移行スクリプトは、プラットフォームごとに各ファイルを正しい
移行先へ配置します。

### OpenWarp 移行先パス

| カテゴリ | macOS | Linux | Windows |
|---|---|---|---|
| config | `~/.openwarp/` | `${XDG_CONFIG_HOME:-~/.config}/openwarp/` | `%LOCALAPPDATA%\openwarp\OpenWarp\config\` |
| data | `~/.openwarp/` | `${XDG_DATA_HOME:-~/.local/share}/openwarp/` | `%APPDATA%\openwarp\OpenWarp\data\` |
| ホームドットファイル | `~/.openwarp/` | `~/.openwarp/` | `%USERPROFILE%\.openwarp\` |

### Zap 移行元パス

| カテゴリ | macOS | Linux | Windows |
|---|---|---|---|
| config | `~/.zap/` | `${XDG_CONFIG_HOME:-~/.config}/zap/` | `%LOCALAPPDATA%\zap\Zap\config\` |
| data | `~/.zap/` | `${XDG_DATA_HOME:-~/.local/share}/zap/` | `%APPDATA%\zap\Zap\data\` |
| ホームドットファイル | `~/.zap/` | `~/.zap/` | `%USERPROFILE%\.zap\` |

### 上流 Warp 移行元パス

| カテゴリ | macOS | Linux | Windows |
|---|---|---|---|
| config | `~/.warp/` | `${XDG_CONFIG_HOME:-~/.config}/warp-terminal/` | `%LOCALAPPDATA%\warp\Warp-Terminal\config\` |
| data | `~/.warp/` | `${XDG_DATA_HOME:-~/.local/share}/warp-terminal/` | `%APPDATA%\warp\Warp-Terminal\data\` |
| ホームドットファイル | `~/.warp/` | `~/.warp/` | `%USERPROFILE%\.warp\` |

> Linux のディレクトリ名 `warp-terminal` は Linux パッケージ名と一致します
> (例:Debian/Ubuntu の `/opt/warpdotdev/warp-terminal/`)。Windows の組織
> フォルダは Warp のパッケージ方法によって異なる場合があります。
> `%APPDATA%\warp\Warp-Terminal`(または `%LOCALAPPDATA%\warp\Warp-Terminal`)が
> 見つからない場合は、実際のインストールが使用している `%APPDATA%` /
> `%LOCALAPPDATA%` の場所を確認してください。

---

## 1. Zap から(既存ユーザー推奨の経路)

Zap は OpenWarp **そのもの**でした —— このプロジェクトは OpenWarp の名前を
復元する前、Zap ブランドで配布されていました。改名は識別子とディスク上のパスを
変更しただけで、**設定ファイルのフォーマットとスキーマは変わっていない**ため、
以下のファイルはそのままコピーできます。

### コピーするファイル

| ファイル / フォルダ | カテゴリ | 制御対象 |
|---|---|---|
| `settings.toml` | config | 公開設定(TOML ベースの設定ファイル)。 |
| `keybindings.yaml` | config | カスタムキーバインド。 |
| `themes/` | data | カスタムテーマ。 |
| `workflows/` | data | カスタムワークフロー。 |
| `launch_configurations/` | data | 起動設定。 |
| `tab_configs/` | data | タブ設定。 |
| `.mcp.json` | ホームドットファイル | MCP サーバー設定。 |
| `skills/` | ホームドットファイル | Agent skills。 |

### 手順

> コピーの前に OpenWarp を終了し、プロセスがファイルを掴んでいない状態にして
> ください。

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

`[ ! -e ... ]` / `-not (Test-Path $to)` のガードは、OpenWarp 側で既に設定した
内容の上書きを防ぎます。Zap 側の値を優先したい場合はガードを外してください。

OpenWarp の動作を確認できたら、上記の Zap ディレクトリを削除してディスク
容量を回収できます。もはやどのプログラムからも使われていません。

---

## 2. 上流 Warp から

上流 Warp は独立した製品で、独自のディスク上のアイデンティティを持ちます
(上の「上流 Warp 移行元パス」の表を参照)。OpenWarp は `Oss` チャンネルで
ビルドされており、独自の app ID(`dev.openwarp.OpenWarp`)とプラットフォーム
ごとの独自レイアウトを持ちます。2 つのインストールは互いのファイルを参照
できません —— これが Warp のアカウント / クラウド状態を OpenWarp の外に保つ
仕組みでもあります。

以下に挙げるテキスト形式のファイルはスキーマが安定・互換なので、コピーは
安全です。**それ以外の状態は違います** —— Warp は OpenWarp と独立に進化して
おり、バイナリ / プライベートストアは Warp の認証や bundle アイデンティティに
紐付いている可能性があります。

### コピーするもの

上と同じ 8 項目です:

| ファイル / フォルダ | カテゴリ | 制御対象 |
|---|---|---|
| `settings.toml` | config | 公開設定(TOML ベースの設定ファイル)。 |
| `keybindings.yaml` | config | カスタムキーバインド。 |
| `themes/` | data | カスタムテーマ。 |
| `workflows/` | data | カスタムワークフロー。 |
| `launch_configurations/` | data | 起動設定。 |
| `tab_configs/` | data | タブ設定。 |
| `.mcp.json` | ホームドットファイル | MCP サーバー設定。 |
| `skills/` | ホームドットファイル | Agent skills。 |

### コピー**しない**もの

- **`user_preferences.json`** ——
  `~/Library/Application Support/dev.warp.Warp/`(macOS)または Linux/Windows の
  対応する状態ディレクトリにあるプライベートストア。ユーザー設定に加えて認証
  トークン、マシン固有 ID、クラウドのキャッシュ状態が混在しています。コピー
  するとアイデンティティの漏洩や OpenWarp の認証状態の混乱を招きます。
  OpenWarp のデフォルトは既にプライバシーに配慮した設定です。
- **`warp.sqlite`**(および `-wal` / `-shm` サイドカー)—— スキーマが上流
  Warp と結合しており、OpenWarp のマイグレーションとの互換性は保証されません。
- **Keychain / DPAPI / libsecret のエントリ** —— Warp の bundle / サービス名に
  紐付いており、OpenWarp には無意味です。

### 手順

> コピーの前に Warp と OpenWarp の両方を終了してください。

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

元の Warp データには一切触れません —— Warp 自体はそのまま動き続けます。

---

## 検証

OpenWarp を起動します。テーマピッカーにカスタムテーマ、キーバインドエディタに
カスタムキーバインド、ワークフローランチャーにワークフローが表示されるはずです。
設定 UI の値は `settings.toml` の内容を反映しているはずです。

何かおかしい場合、原因は上記 8 ファイルのいずれかです —— テキストエディタで
開いて確認するか、削除して OpenWarp にデフォルト値へフォールバックさせて
ください。

## ロールバック

このガイドに破壊的な操作はありません:コピーしたファイルはすべて、OpenWarp が
次回起動時にデフォルトから再生成できるものです。すべて取り消すには:

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

Zap と Warp の移行元ディレクトリは、このガイドでは一切触れられません。
