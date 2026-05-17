# ドライバ実装チェックフロー

> 新しい DriverType を追加するときの最短確認ルートです。別セッションで再開する場合も、この順で辿れば現在地を把握できます。

## 0. 着手前確認

- `driver_type` が確定している
- 対象範囲が明確になっている
  - 登録UI
  - 通信ランタイム
  - 本体取り込み
- 正本ドキュメントを開いている
  - `docs/driver-development.md`
  - `docs/ui-registration.md`
  - `docs/config-spec.md`

## 1. 雛形を置く

- `docs/templates/driver-ui-main-template.rs`
- `docs/templates/driver-ui-app-template.js`
- `docs/templates/driver-runtime-main-template.rs`
- `docs/templates/driver-runtime-grpc-client-template.rs`

確認:

- UI アプリが起動する
- ランタイムが起動する
- まだ未完成でもビルドと起動の入口がある

## 2. 登録UI をつなぐ

- `get_driver_ui_launch_context` を読める
- 新規 / 編集モードを判定できる
- `save_driver_ui_output` へ payload を返せる

確認:

- 新規作成で保存できる
- 編集で既存値が復元される
- 保存失敗時にエラーが見える

## 3. ドライバ固有処理を足す

- 接続テスト
- 候補一覧取得
- ScanGroup 生成
- Tag 生成

確認:

- 取得失敗時に再試行できる
- 保存前レビューが出る
- 必須項目不足が UI 上で分かる

## 4. 本体取り込みを確認する

- `drivers.toml` に反映される
- `tags.toml` に反映される
- タグ管理画面で見える

確認:

- 新規接続先が追加される
- 既存接続先が更新される
- 反映順序の不整合がない

## 5. 通信ランタイムをつなぐ

- gRPC 接続できる
- DriverDefinition を取得できる
- poller / client を組める
- タグ値送信ができる

確認:

- 接続失敗後に再試行する
- 1 件以上のタグ値送信が成功する
- ループが異常終了しても即終了しない

## 6. 配置と探索を確認する

- `driver-ui/<type>/registration-ui.exe`
- `driver-ui/<type>/driver-<type>.exe`

確認:

- タグ管理画面から登録UIを起動できる
- 本体が通信ランタイムを見つけられる

## 7. 最終確認

- `npm run check`
- `cargo test`
- 必要に応じて影響クレート単体テスト

完了条件:

- 新規 / 編集の両モードが動く
- JSON 取り込みが成功する
- 通信ランタイムが値送信できる
- 次セッション向けの引き継ぎメモが残っている

## 8. 引き継ぎ

- `docs/templates/driver-session-handoff-template.md` を使って記録する
- 次セッションで最初に見るファイルを 3 つ残す
- 未確認項目を明記する
