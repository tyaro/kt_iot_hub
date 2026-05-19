# ドキュメント索引（INDEX）

このファイルは `kt_iot_hub` のドキュメント入口です。

## ルート `docs/`（横断・正本）

- 全体設計: [`design.md`](./design.md)
- アーキテクチャ: [`architecture.md`](./architecture.md)
- 設計判断: [`decisions.md`](./decisions.md)
- リファクタ計画: [`refactor-plan.md`](./refactor-plan.md)
- 設定仕様: [`config-spec.md`](./config-spec.md)
- UI 登録仕様: [`ui-registration.md`](./ui-registration.md)
- ドライバ manifest 設計: [`driver-manifest-discovery-design.md`](./driver-manifest-discovery-design.md)
- ドライバ拡張性設計: [`driver-property-extensibility-design.md`](./driver-property-extensibility-design.md)

## 領域別ドキュメント（実装近接）

- Core: [`../core/docs/README.md`](../core/docs/README.md)
  - MQTT モニタ仕様: [`../core/docs/mqtt-monitor.md`](../core/docs/mqtt-monitor.md)
- Drivers: [`../drivers/docs/README.md`](../drivers/docs/README.md)
  - ドライバ開発ガイド: [`../drivers/docs/driver-development.md`](../drivers/docs/driver-development.md)
  - 実装フロー: [`../drivers/docs/driver-implementation-flow.md`](../drivers/docs/driver-implementation-flow.md)
- Operations: [`../ops/docs/README.md`](../ops/docs/README.md)
  - デプロイ手順: [`../ops/docs/deployment.md`](../ops/docs/deployment.md)
- JoyWatcher 固有: [`../drivers/joywatcher/docs/README.md`](../drivers/joywatcher/docs/README.md)
  - 調査: [`../drivers/joywatcher/docs/investigation.md`](../drivers/joywatcher/docs/investigation.md)
  - 引き継ぎ: [`../drivers/joywatcher/docs/session-handoff.md`](../drivers/joywatcher/docs/session-handoff.md)
  - x86 ブリッジ設計: [`../drivers/joywatcher/docs/bridge-x86-design.md`](../drivers/joywatcher/docs/bridge-x86-design.md)

## 運用ルール（要約）

- 横断仕様・不変条件は `docs/` に置く
- 実装手順・運用手順・調査メモは各領域 `*/docs/` に置く
- 迷ったらこの `INDEX.md` から辿る
