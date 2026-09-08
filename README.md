# meigen-bot

Discordの普段の会話から、名言・迷言っぽい発言を自動検出するRust製Botです。AIは使用しません。

## インストール

```bash
pip install meigen-bot
```

## 使い方

```python
from meigen_bot import run

run("DISCORD_BOT_TOKEN")
```

DBの保存先を変更する場合：

```python
from meigen_bot import run

run("DISCORD_BOT_TOKEN", database="meigen.db")
```

Discord Developer Portalで **Message Content Intent** をONにしてください。
Botの招待時には `bot` と `applications.commands`、メッセージ閲覧・送信・Embed表示権限が必要です。

## コマンド一覧

- `/meigen` - 最近の名言を表示
- `/meigen-user` - 指定ユーザーの名言を表示
- `/meigen-random` - ランダムな名言を表示
- `/meigen-top` - 歴代最高得点を表示
- `/ranking` - 名言数ランキングを表示
- `/profile` - 名言プロフィールと称号を表示
- `/delete-my-data` - 自分の保存済み名言を削除
- `/meigen-settings` - 管理者向け検出設定

## データ保存

検出した発言の本文・ユーザーID・チャンネルID・スコアなどを、起動場所の `meigen.db` に保存します。
自分の保存データは `/delete-my-data` で削除できます。
