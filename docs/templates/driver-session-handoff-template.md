# ドライバ実装 引き継ぎテンプレート

> 新しいセッションへ作業を引き継ぐ際は、このテンプレートをそのまま埋めて使う。

## 基本情報

- 対象 `driver_type`:
- 対象範囲:
  - [ ] 登録UI
  - [ ] 通信ランタイム
  - [ ] 本体取り込み
  - [ ] ドキュメント更新
- 参照中の正本ドキュメント:
  - `docs/driver-development.md`
  - `docs/ui-registration.md`
  - `docs/config-spec.md`

## 今回完了したこと

-
-
-

## まだ未完了のこと

-
-
-

## 変更ファイル

-
-
-

## 手動確認済み

- [ ] 登録UI が起動する
- [ ] `get_driver_ui_launch_context` が読める
- [ ] `save_driver_ui_output` が呼べる
- [ ] 新規モードで保存できる
- [ ] 編集モードで既存値が復元される
- [ ] 通信ランタイムが起動する
- [ ] gRPC 接続が成功する
- [ ] タグ値を 1 件以上送信できる

## 未確認 / 要確認

-
-
-

## 発生した問題とメモ

-
-
-

## 次セッションで最初に見るファイル

1.
2.
3.

## 次の最小タスク

1.
2.
3.

## 完了条件の見込み

- [ ] タグ管理画面から登録UIを起動できる
- [ ] 新規 / 編集の両モードで JSON を返却できる
- [ ] `driver-<type>` が起動できる
- [ ] gRPC 経由で値送信できる
- [ ] 影響範囲の確認コマンドが通る

## 確認コマンド

- ルート: `npm run check`
- Rust: `cargo test`
- 必要に応じて: 影響クレートのみの `cargo test -p <crate>`

## 補足

- 開発時ビルド成果物は workspace ルートの `target/` に出る
- 配置規約は `driver-ui/<type>/registration-ui.exe` と `driver-ui/<type>/driver-<type>.exe`
- 返却 JSON の正本は `packages/protocol-rs`
