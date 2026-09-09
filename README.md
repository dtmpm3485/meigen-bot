# meigen-bot — Discordの名言・迷言を自動検出するRust製Bot

[![PyPI](https://img.shields.io/pypi/v/meigen-bot)](https://pypi.org/project/meigen-bot/)
[![Python](https://img.shields.io/pypi/pyversions/meigen-bot)](https://pypi.org/project/meigen-bot/)
[![License](https://img.shields.io/github/license/dtmpm3485/meigen-bot)](LICENSE)
[![GitHub stars](https://img.shields.io/github/stars/dtmpm3485/meigen-bot?style=social)](https://github.com/dtmpm3485/meigen-bot/stargazers)

**meigen-bot** は、Discordの普段の会話から「名言・迷言っぽい発言」を自動検出する **AI不要・Rust製のDiscord Botライブラリ** です。

会話を止めずに常時判定し、名言を見つけるとBotが自動で反応します。Pythonから1関数で起動でき、ランキング・プロフィール・称号・ランダム表示なども利用できます。

> 💬 人間は、反応しないんじゃない。反応できないんだ
>
> 🤖 **名言を検出しました**
> 「人間は、反応しないんじゃない。反応できないんだ」

## ✨ 特徴

- 💬 **Discordの通常会話から名言・迷言を自動検出**
- 🧠 **AI / 外部API不要** — ルールベースで判定
- 🦀 **Rust製** — Bot本体と判定処理をRustで実装
- 🐍 **Pythonから簡単起動** — `run(token)` だけ
- 🏆 **ランキング・最高得点・プロフィール・称号**
- 🎲 **最近の名言・ユーザー別・ランダム表示**
- 💾 **SQLiteでローカル保存**
- 🔧 **管理者向け検出設定**

## 🚀 すぐに使う

### 1. インストール

```bash
pip install -U meigen-bot
```

### 2. Botを起動

```python
from meigen_bot import run

run("DISCORD_BOT_TOKEN")
```

DBの保存先を変更する場合：

```python
from meigen_bot import run

run("DISCORD_BOT_TOKEN", database="meigen.db")
```

### 3. Discord側の設定

Discord Developer Portalで **Message Content Intent** をONにしてください。

Botの招待時には、以下を利用できるようにしてください。

- `bot`
- `applications.commands`
- メッセージ閲覧
- メッセージ送信
- Embed表示

## 📖 コマンド一覧

| コマンド | 内容 |
|---|---|
| `/meigen` | 最近の名言を表示 |
| `/meigen-user` | 指定ユーザーの名言を表示 |
| `/meigen-random` | ランダムな名言を表示 |
| `/meigen-top` | 歴代最高得点を表示 |
| `/ranking` | 名言数ランキングを表示 |
| `/profile` | 名言プロフィールと称号を表示 |
| `/delete-my-data` | 自分の保存済み名言を削除 |
| `/meigen-settings` | 管理者向け検出設定 |

## ⚙️ どうやって名言を判定する？

生成AIに文章を送る方式ではなく、文章の形や言い回しなどを使った **ルールベース判定** です。

そのため、外部AI APIの契約やAPIキーを用意せずにDiscordの名言Botを動かせます。

## 💾 データ保存

検出した発言の本文・ユーザーID・チャンネルID・スコアなどをSQLiteへ保存します。標準では起動場所に `meigen.db` が作成されます。

自分の保存データは `/delete-my-data` から削除できます。

## 🔎 こんな人向け

- Discordサーバーに**名言Bot / 迷言Bot**を入れたい
- 会話から面白い発言を自動で拾いたい
- AI APIなしで文章判定をしたい
- Rust製Discord BotをPythonから簡単に動かしたい
- サーバー内ランキングやネタ機能を追加したい

## ❓ FAQ

### Discordの会話を自動で監視して名言を見つけられますか？

はい。Botが閲覧できるメッセージを対象に、名言・迷言らしい文章を自動判定します。

### OpenAIなどのAI APIは必要ですか？

必要ありません。meigen-botはAIを使わないルールベース方式です。

### Pythonだけで起動できますか？

はい。パッケージをインストールしたあと、Pythonから `run("DISCORD_BOT_TOKEN")` を呼び出して起動できます。

## 🤖 DiscordネタBotシリーズ

| Bot | 内容 |
|---|---|
| **meigen-bot** | 会話から名言・迷言を自動検出 |
| [senryu-bot](https://github.com/dtmpm3485/senryu-bot) | Discordの会話から川柳を自動検出 |
| [yaju-bot](https://github.com/dtmpm3485/yaju-bot) | Discordの会話に低確率で乱入するネタBot |

## ⭐ 気に入ったら

このBotが面白かった・役に立った場合は、GitHubの **Star ⭐** を付けてもらえると開発の励みになります。

Issue・改善案・バグ報告も歓迎です。

## License

MIT License
