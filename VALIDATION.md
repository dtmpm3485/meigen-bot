# 検証記録

実施日: 2026-09-07。Linux x86_64 / CPython 3.12.13 / Rust 1.98.1 / maturin 1.15.0。

## 実行した確認

- `cargo fmt --check`
- `cargo check --locked --features python`
- `cargo clippy --locked --all-targets --features python -- -D warnings`
- `cargo test --locked`
- `maturin build --release --locked`
- 生成wheelを独立したvenvにインストールし、`tests/python_smoke.py`を実行

Rustテストは6関数すべて成功。検出コーパスは**133文章（検知40・無視93）すべて期待通り**。
コーパスは調整にも使った回帰テストであり、未知の文章での正解率ではありません。

DBの永続化・親フォルダ自動生成・設定検証・スキーマ互換拒否・同時重複登録・3秒のDBロック失敗後の復旧・ユーザー削除・サーバー/チャンネル絞り込みを確認。
クールダウンの期限切れ・ユーザー/サーバー分離・ID/文章の重複防止を確認。

Pythonから `run`, `version`, `analyze` をimportし、Rust拡張を実行。
無効な引数、DB作成エラーのPython例外変換、asyncioから別スレッドでのRust worker起動、Unicode孤立サロゲートの例外処理を確認。

生成したローカルwheel:
`meigen_bot-0.1.0-cp312-cp312-manylinux_2_34_x86_64.whl`

これはCPython 3.12 / Linux x86_64 / glibc 2.34以降用です。Android、Windows、macOS、他のPython向けではありません。
公開用CIはmanylinuxコンテナ内でLinux wheelを別途生成します。

## 指定例の実測スコア

| 文章 | 名言度 |
| --- | --- |
| 学校には遅刻するけどログボには遅刻しない | 91 |
| 金で幸せは買えない。でもガチャは回せる。 | 88 |
| 努力は裏切らない。俺は努力を裏切る。 | 78 |
| 寝る時間がないんじゃない。ゲームする時間が長いだけだ。 | 99 |
| 課金は無駄じゃない。未来の後悔を先に買ってるだけだ。 | 96 |
| 人生は一度きり。でもリセマラは何度でもできる。 | 78 |
| 負けたんじゃない。勝つ前に終わっただけだ。 | 86 |
| 明日やればいいことを今日やる必要はない。 | 79 |

参考として、release拡張をPythonから133文章×100回呼んだ単発測定で、1件平均約4.4μs（Python辞書変換込み）。Discord通信やDB処理は含みません。ハードウェアや入力によって変動し、性能保証値ではありません。

## 実環境が必要な確認

実Discord Botトークンは提供されていないため、Discordへのログイン・実サーバーでのEmbed送信・コマンド操作・Gateway再接続・Ctrl+Cによる接続終了の実地試験は行っていません。
Windows/macOS/ARM64およびCPython 3.10/3.11/3.13はGitHub Actionsの各ジョブでビルド・import確認を行う構成です。成功状態はActionsの結果を参照してください。
Termux/Android・iOS・PyPy・free-threaded CPythonは動作保証対象外です。
PyPIへのアップロードには所有者側でTrusted Publisherの登録が必要です。公開済みとは扱っていません。
