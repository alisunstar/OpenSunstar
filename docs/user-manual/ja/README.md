# OpenSunstar ユーザーマニュアル（日本語）

**バージョン：** v0.1.0 · **ライセンス：** MIT

> Claude Code、Claude Desktop、Codex、Gemini CLI、OpenCode、OpenClaw、Hermes 向けネイティブデスクトップマネージャー。

---

## 目次

1. [はじめに](#1-はじめに)
2. [接続とプロバイダー](#2-接続とプロバイダー)
3. [Agent 設定](#3-agent-設定)
4. [ポートフォリオ](#4-ポートフォリオ)
5. [プロキシとフェイルオーバー](#5-プロキシとフェイルオーバー)
6. [使用量と予算](#6-使用量と予算)
7. [同期とバックアップ](#7-同期とバックアップ)
8. [設定とデータパス](#8-設定とデータパス)
9. [FAQ](#9-faq)

関連：[v0.1.0 リリースノート](../release-notes/v0.1.0-ja.md) · [ポートフォリオ詳細](../kanban.md)

---

## 1. はじめに

### インストール

[GitHub Releases](https://github.com/alisunstar/OpenSunstar/releases/latest) から各 OS 用パッケージをダウンロード。

| OS | 形式 |
| -- | ---- |
| Windows | `.msi` / Portable `.zip` |
| macOS | `.dmg` / `brew install --cask OpenSunstar` |
| Linux | `.deb` / `.rpm` / `.AppImage` |

### 初回起動

1. 既存 CLI 設定を **default** プロバイダーとしてインポート
2. サイドバー **Simple Connect** で API 接続
3. メイン UI または **システムトレイ** で切り替え
4. ほとんどの CLI は **ターミナル再起動** が必要（Claude Code のみホットスイッチ）

---

## 2. 接続とプロバイダー

### Simple Connect（3 ステップ）

1. プロバイダー選択
2. API キー保存
3. CLI ツールとモデルを選んで適用

**Expert** タブで全プロバイダーを管理。

### 共有設定スニペット

プロバイダー切り替え時にプラグイン等を保持：

- 編集 → 共有設定パネル → 現在のプロバイダーから抽出
- 新規作成時は「共有設定を書き込む」をオン（デフォルト）

### Deep Link

`OpenSunstar://import/...` でプロバイダー、MCP、Prompts、Skills をインポート。

---

## 3. Agent 設定

| 機能 | 内容 |
| ---- | ---- |
| **MCP** | サーバー管理、ディスカバリ、アプリ別同期 |
| **Skills** | GitHub / ZIP / skills.sh / ClawHub / ModelScope |
| **Prompts** | CLAUDE.md 等の Markdown 編集と同期 |
| **Commands / Hooks** | コマンドとフック |
| **Sessions** | 会話履歴の参照と復元 |
| **OpenClaw** | AGENTS.md、SOUL.md 等の編集 |

---

## 4. ポートフォリオ

サイドバー **ポートフォリオ**（项目组合）は、マルチ Git リポジトリの**開発コックピット**です（タスク看板ではありません）。

- ローカル Git リポジトリを追加
- コード行数、**7 日間**コミット、貢献者を表示
- AI 週次レポートとヘルススコア

詳細：[kanban.md](../kanban.md)

---

## 5. プロキシとフェイルオーバー

- ローカルルーティングプロキシ（形式変換、整流器）
- 自動フェイルオーバーキュー、サーキットブレーカー
- アプリ単位のプロキシテイクオーバー

---

## 6. 使用量と予算

- 使用量ダッシュボード（支出、Token、リクエスト）
- プロバイダー別日次/月次 USD 上限とアラート

---

## 7. 同期とバックアップ

- **WebDAV** / **S3 互換** クラウド同期（同時に一つのみ）
- 設定ディレクトリを Dropbox / iCloud 等に変更可能
- SQL 形式のインポート/エクスポート

---

## 8. 設定とデータパス

| パス | 内容 |
| ---- | ---- |
| `~/.OpenSunstar/OpenSunstar.db` | SQLite データベース |
| `~/.OpenSunstar/settings.json` | UI 設定 |
| `~/.OpenSunstar/backups/` | 自動バックアップ |
| `~/.OpenSunstar/skills/` | Skills 保存先 |

言語：简体中文 · 繁體中文 · English · 日本語

---

## 9. FAQ

**切り替え後にターミナル再起動？**  
通常は必要。Claude Code のみホットスイッチ。

**公式ログインに戻す？**  
Official プリセットを追加 → 切り替え → CLI でログアウト/ログイン。

---

[← マニュアル索引](../README.md) · [v0.1.0 リリースノート](../release-notes/v0.1.0-ja.md)
