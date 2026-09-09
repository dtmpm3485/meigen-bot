# meigen-bot

[![PyPI](https://img.shields.io/pypi/v/meigen-bot)](https://pypi.org/project/meigen-bot/)
[![Python](https://img.shields.io/pypi/pyversions/meigen-bot)](https://pypi.org/project/meigen-bot/)
[![License](https://img.shields.io/github/license/dtmpm3485/meigen-bot)](LICENSE)

Discordの会話から名言・迷言っぽい発言を自動で検出するBotです。Bot本体と判定処理はRustで実装されており、Pythonから起動できます。

![meigen-botの動作例](https://github.com/dtmpm3485/meigen-bot/blob/main/assets/demo.jpg?raw=true)

## 機能

- 通常の会話から名言・迷言を自動検出
- 最近の名言を表示
- ユーザー別の名言を表示
- ランダム表示
- 歴代最高得点
- 名言数ランキング
- プロフィールと称号
- 管理者向けの検出設定
- SQLiteへの保存

## インストール

```bash
pip install -U meigen-bot
```

## 起動

```python
from meigen_bot import run

run("DISCORD_BOT_TOKEN")
```

DBの保存先を指定する場合:

```python
from meigen_bot import run

run("DISCORD_BOT_TOKEN", database="meigen.db")
```

## Discord側の設定

Discord Developer Portalで `Message Content Intent` を有効にしてください。

Botの招待時には `bot` と `applications.commands` を指定し、メッセージの閲覧・送信・Embed表示に必要な権限を付けてください。

## コマンド

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

## データ保存

検出した発言の本文、ユーザーID、チャンネルID、スコアなどをSQLiteに保存します。標準では起動した場所に `meigen.db` が作成されます。

自分の保存データは `/delete-my-data` から削除できます。

## 関連リポジトリ

- [senryu-bot](https://github.com/dtmpm3485/senryu-bot) - Discordの会話から川柳を検出するBot
- [yaju-bot](https://github.com/dtmpm3485/yaju-bot) - Discordの会話にランダムで乱入するBot

## License

MIT License
