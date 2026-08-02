# Brief: market-time-clockboard-design

## Problem

把产品首页做成直接可扫读的全球交易时间表：一条 00–24 UTC 轴逐交易场所列出完整服务端 timeline、当地交易窗口、当前状态与日历例外。原有证据、修订、历史时刻和 unknown 细节移到独立 `/audit` 时间自检页；浏览器不得推导交易时段。

## Framework

native-html-css-js served by Axum

## Reference Sites

- User-supplied interaction reference: <https://www.jin10.com/activities/global_trading_hours/index.html>.
  Adopt only its high-density UTC-ruler / current-cursor / market-row hierarchy. Do not copy
  branding, copy, palette, venue data, or schedule content.

## Template Evidence

- None provided.

## Acceptance

- Resolve material product ambiguity through grill-with-docs.
- Assess scope before synthesis.
- Generate a project-specific DESIGN.md with explicit source decisions.
- Continue through implementation and normal design-pipeline QA.
- Make the complete daily timetable the product's visual centre; preserve the server/core as the
  only source of market state, phase intervals, trading windows, exceptions, and unknown.
