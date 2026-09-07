# meigen-bot

普段のDiscord会話から「名言っぽい迷言」を拾う、AI不使用のRust製Botライブラリ。
Pythonはネイティブ拡張の関数を公開するだけで、検出・スコア・Discord接続・SQLite処理はRustで動きます。

```python
from meigen_bot import run
run("DISCORD_BOT_TOKEN")
```

「学校には遅刻するけどログボには遅刻しない」のような発言に、名言度・深そう度・ネタ度・説得力・人生終わってる度を載せたEmbedで返信します。
評価は遊びのための文章特徴スコアです。人間の価値・能力・精神状態を判定しません。

## インストール

PyPIにリリースが公開された後は、対応環境で以下を実行します。

```sh
python -m pip install --upgrade meigen-bot
```

公開前、またはソースから入れる場合はRustツールチェーンとCコンパイラを用意し、以下を実行します。

```sh
python -m pip install git+https://github.com/dtmpm3485/meigen-bot.git
```

パッケージ名は **meigen-bot**、import名は **meigen_bot** です。公開状況は[PyPI](https://pypi.org/project/meigen-bot/)で確認してください。

## 使い方

```python
from meigen_bot import run, version

print(version())
run(token="DISCORD_BOT_TOKEN", database="meigen.db")
```

実運用ではトークンをソースやGitに保存しないでください。

```python
import os
from meigen_bot import run
run(os.environ["DISCORD_BOT_TOKEN"])
```

`run()`はBot終了まで待機する同期関数です。GILを解放し、別OSスレッドに専用Tokio Runtimeを作ります。
既存のTokio Runtimeへ入れ子で`block_on`しません。メインスレッドのCtrl+Cを検出するとシャードを停止し、`KeyboardInterrupt`を返します。
asyncioアプリでは`await asyncio.to_thread(run, token)`を使えます。ただしそのasyncioタスクのキャンセルだけではBotを停止できません。独立したプロセスでの実行を推奨します。

オフラインで判定を試すには追加API `analyze(text)` が使えます。Pythonでも判定処理はRustです。

```python
from meigen_bot import analyze
print(analyze("努力は裏切らない。俺は努力を裏切る。"))
```

## Discord Developer Portal

1. [Developer Portal](https://discord.com/developers/applications)でApplicationを作成し、Botを設定します。
2. Bot画面の **Message Content Intent** をONにします。規模によってはDiscordへの申請が必要です。
3. Botトークンを取得します。Client SecretやPublic Keyとは別物です。
4. OAuth2のURL Generatorで `bot` と `applications.commands` を選びます。
5. Bot権限に **View Channels / Send Messages / Embed Links / Read Message History** を付けてサーバーへ招待します。
6. Pythonファイルから`run()`を呼びます。起動時にスラッシュコマンドを登録します。反映に時間がかかることがあります。

Bot自体にAdministrator権限は不要です。Server Members Intent / Presence Intentは要求しません。
通常テキスト・アナウンスチャンネルが対象です。DM、グループDM、Bot・Webhook投稿、システムメッセージは無視します。
このバージョンではスレッド・フォーラム投稿内の会話を検出しません。編集イベントによる再評価もしません。

## コマンド

すべてサーバー内で使用します。検索・ランキング・プロフィールは、実行者が**現在閲覧と履歴閲覧できるチャンネル**だけを対象にし、返信は本人だけに表示します。
削除されたチャンネルは検索対象から外れます。

| コマンド | 内容 |
| --- | --- |
| `/meigen` | 最近の名言5件 |
| `/meigen-user user:` | 指定ユーザーの最近の名言5件 |
| `/meigen-random` | ランダムに1件 |
| `/meigen-top` | 歴代最高得点から5件 |
| `/ranking` | 名言数上位10人 |
| `/profile [user:]` | 件数・最高点・平均点・得意ジャンル・称号。省略時は自分 |
| `/delete-my-data confirm:True` | このBotのDBにある全サーバーの自分の保存名言を削除 |
| `/meigen-settings` | 管理者専用の設定表示・変更 |

### 管理者設定

サーバー所有者またはAdministrator権限のあるユーザーだけが変更できます。UI表示制限に加え実行時にも権限を検証します。
Discord標準のBoolean選択・ユーザー/チャンネル選択・数値入力UIを使います。
省略した項目は変更しません。

| オプション | 内容 | 初期値 |
| --- | --- | --- |
| `enabled` | 検出ON/OFF | True |
| `threshold` | 65〜95 | 75 |
| `cooldown` | 同一ユーザーの待機秒数、0〜86400 | 60 |
| `output` | 出力先テキストチャンネル | 元チャンネル |
| `reset-output` | Trueで出力先を解除 | False |
| `exclude` | 指定チャンネルを除外一覧に追加 | なし |
| `include` | 指定チャンネルの除外を解除 | なし |

例: `/meigen-settings threshold:85 cooldown:120`
サーバー全体にも3秒の間隔を設けます。同じメッセージIDはDB主キーと600秒のメモリ記録で重複を防ぎます。
同じサーバーの同一文章はNFKC・大小文字・空白を正規化して300秒間重複保存しません。
クールダウン情報は60秒ごとと判定時に掃除し、件数上限を設けています。再起動時にクールダウンはリセットされますが、DBのID/文章重複チェックは残ります。

非公開会話の転送を避けるため、別チャンネルへの自動転送は`@everyone`が元チャンネルを閲覧・履歴閲覧でき、元チャンネルの上書きに閲覧拒否が一つもない場合だけ行います。
条件を満たさない元チャンネルは、その場に返信します。これは保守的な判定で、一部の公開チャンネルも転送せず返信する場合があります。

## 検出エンジン

`meigen_core::detector::analyze_message(&str)`はDiscordと独立しています。
正規表現を`LazyLock`で一度だけ初期化し、300文字までの文章を解析します。

- 対比・比較・言い切り・説明構造を検出。
- 前後の節の共通3文字以上の語句、または漢字を含む2文字の語幹から対句を推定。
- 否定/肯定の違いと語句再出現・修正構文から逆説を推定。
- テーマ辞書で複数ジャンル分類。単なるテーマ単語だけでは閾値に届きません。
- 短文・挨拶・質問・URLのみ・絵文字のみ・メンションのみ・コマンド・コード・連打・長文を除外。
- 乱数やAIを使わず、同じ入力には同じスコアを返します。ランダム名言検索のみSQLiteの`random()`を使います。

`AnalysisResult`は `is_meigen`, `score`, `deep_score`, `funny_score`, `persuasion_score`, `degenerate_score`, `genres`, `reasons` を返します。
`is_meigen`は標準閾値75で判定し、Botはサーバーごとの設定閾値で反応します。

| 点数 | 表示 |
| --- | --- |
| 0〜64 | 無視 |
| 65〜74 | 名言候補（設定閾値を下げた場合のみ返信） |
| 75〜84 | 名言 |
| 85〜94 | 強い名言 |
| 95〜100 | 伝説の名言 |

称号は最高点・件数・多いジャンル・サブスコア平均から条件分岐します。
言語の意味を完全に理解するものではなく、皮肉や逆説の見逃し・日常文の誤検出はあり得ます。
同梱コーパスは回帰テスト用であり、実際のサーバー全体での精度を保証するベンチマークではありません。

## SQLiteとプライバシー

標準は**起動時の作業ディレクトリの `meigen.db`**です。`database=`で別のパスを指定でき、親フォルダも自動作成します。
SQLiteライブラリはRustに同梱し、PythonからDB操作しません。WALモードでは`meigen.db-wal`と`meigen.db-shm`も生成されます。
バックアップはBot停止後に取得するかSQLiteのバックアップ手段を使ってください。

保存対象は閾値以上で、設定・クールダウンを通過した発言だけです。
メッセージID、サーバー/チャンネル/ユーザーID、ユーザー名、**本文**、5スコア、ジャンル、発言時刻、重複判定用正規化本文、登録時刻を保存します。
設定表にはサーバーID、ON/OFF、閾値、クールダウン、出力先、除外チャンネルを保存します。
`PRAGMA user_version`でスキーマを管理し、新しすぎるDBはエラーにして起動を中止します。

管理者は導入前に参加者へ本文保存と削除方法を案内してください。自動保存の保持期限はありません。
`/delete-my-data confirm:True`はこのDBに保存された**本人の全サーバー分の名言**を削除します。
元発言、BotがDiscordへ送信済みのEmbed、管理者の別バックアップは削除しません。元発言をDiscordで削除してもDBの保存は自動削除されません。
新しい発言は再び保存対象になります。保存を止める場合は管理者が検出OFFまたはチャンネル除外を設定します。

メモリの重複判定用正規化文章は最大300秒残り、定期掃除で除去します。
削除時はSQLiteのsecure_deleteとWALチェックポイントを使いますが、SSD・OSバックアップ等の物理的消去は保証しません。
DBファイルをGitや公開ストレージに置かないでください。本文やトークンは通常ログへ出しません。
実行時の通信はDiscord API/Gatewayだけです。AI・外部分析・テレメトリへの送信はありません。

## 安定性・負荷

SQLite専用スレッド＋上限128件のキューで同期DB処理をイベントループから分離しています。
DBロックの待機は3秒、同時メッセージ処理は16件、同時コマンド処理は8件までです。
高負荷時はキューを無制限に伸ばさず一部の自動検出をスキップします。
メッセージ保存・設定更新・検索・削除はRust側のロックで整列し、削除直前に処理中だった発言の再登録を防ぎます。
Embed送信に失敗しても保存は維持し、再送による二重投稿を避けます。検索コマンドから確認できます。
Discordの切断・再接続・レート制限はSerenityが扱います。無効トークン等の起動エラーはPython例外として返します。
Rustのpanicは通常unwindで捕捉しますが、メモリ不足・OSによる強制終了などは回復保証の対象外です。

## 対応環境とABI

CIのwheel生成・import確認対象は以下です。各OSの成功状況は[Actions](https://github.com/dtmpm3485/meigen-bot/actions)で確認してください。

| 環境 | 配布設計 |
| --- | --- |
| CPython 3.10 / 3.11 / 3.12 / 3.13 | バージョン別wheel |
| Linux x86_64 / ARM64 | manylinux wheel |
| Windows x86_64 | Windows wheel |
| macOS Intel / Apple Silicon | アーキテクチャ別wheel |
| Termux / Android | 公式wheelなし・動作保証なし |
| iOS / PyPy / free-threaded CPython | 対象外 |

PyO3 0.23.5を使用し、**abi3を有効にしていません**。`cp310`〜`cp313`ごとにビルドします。
Python拡張用の`extension-module` featureはmaturinからのみ有効にし、通常のRustテストでは有効にしません。
Linux wheelをAndroidに改名してインストールするような手順はサポートしません。
TermuxのBionic libcやPythonのシンボル公開条件はmanylinuxと異なります。ソースビルドが成功しても本プロジェクトで実機確認するまでは未検証です。
`PyExc_*`や`dlopen failed`が出た場合、異なる環境向けwheelの流用や古い拡張の残存を確認してください。`LD_PRELOAD`で強引に回避することは推奨しません。

## ソースからビルド

Rust stable、CPython 3.10〜3.13、Cコンパイラが必要です。LinuxではPython開発ヘッダーも用意してください。

```sh
git clone https://github.com/dtmpm3485/meigen-bot.git
cd meigen-bot
python -m venv .venv
```

Linux/macOS: `source .venv/bin/activate`。Windows PowerShell: `.venv\Scripts\Activate.ps1`。

```sh
python -m pip install "maturin>=1.8,<2"
maturin develop
python tests/python_smoke.py
cargo fmt --check
cargo check --locked --features python
cargo clippy --locked --all-targets --features python -- -D warnings
cargo test --locked
maturin build --release --locked
```

`target/wheels/`にwheelが生成されます。SQLite用の外部サーバーは不要です。
Cargo.lockをコミットし、CI/リリースでは`--locked`を使います。
同梱コーパス・DBロック・重複登録・永続化・削除・チャンネル絞り込み・クールダウンをDiscord接続なしでテストします。

## GitHub ActionsとPyPI公開

`ci.yml`はfmt/check/clippy/testを実行し、OS/CPU/CPythonの組み合わせごとにwheelを生成して実際にimportします。
`publish.yml`は`v0.1.0`形式のタグで同じ検証・wheel生成を行い、成功した成果物だけPyPIへ送ります。

初回はPyPIのPublishing設定でTrusted Publisherを登録します。

- PyPIプロジェクト名: `meigen-bot`（未公開ならpending publisher）
- Owner: `dtmpm3485`
- Repository: `meigen-bot`
- Workflow filename: `publish.yml`
- Environment: `pypi`

GitHubのEnvironment `pypi`も作成します。必要ならその環境に承認ルールを設定できます。
PyPIに同名の他人のプロジェクトが既に存在する場合は、この名前では公開できません。
設定後、Cargo.tomlのバージョンに一致するタグを作ります。

```sh
git tag v0.1.0
git push origin v0.1.0
```

ローカルから`maturin publish --release --locked`も使用できます。その場合はmaturinの認証案内に従います。
トークンをコードやworkflowに直書きしないでください。公開済みバージョンは上書きできません。

## トラブルシューティング

- **反応しない**: Message Content Intent、招待権限、ON/OFF、閾値、除外、クールダウンを確認。`analyze()`で文章の点数を確認。
- **コマンドが見えない**: applications.commandsスコープ、サーバーのアプリ権限、起動ログ、登録反映待ちを確認。
- **返信できない**: 出力先のSend Messages/Embed Links権限。保存自体が成功していれば`/meigen`で確認できます。
- **database is locked**: 同じDBへ別Botを重複起動していないか確認。ネットワークドライブでの運用を避けます。
- **DB作成失敗**: `database=`のパスと書き込み権限、ディスク空きを確認。
- **ImportError / PyExc_* / wrong architecture**: `python -m pip --version`で実行Pythonを確認し、そのPython用wheelを再インストール。AndroidにLinux用wheelを使わない。
- **Python 3.14以降で入らない**: このリリースの対応範囲は3.10〜3.13です。
- **Ctrl+C**: `run()`をメインスレッドから起動。終了処理には数秒かかることがあります。
- **ログ**: 標準はBotのINFOと依存ライブラリのWARN。`RUST_LOG=meigen_core=debug,warn`等で調整できます。

## 主要依存の公式資料

[Serenity](https://docs.rs/serenity/0.12.5/serenity/) / [PyO3 0.23.5](https://pyo3.rs/v0.23.5/) / [maturin](https://www.maturin.rs/) / [rusqlite](https://docs.rs/rusqlite/0.32.1/rusqlite/) / [Tokio](https://docs.rs/tokio/)

MIT License。実行した検証結果と未検証範囲は[VALIDATION.md](VALIDATION.md)を参照してください。
