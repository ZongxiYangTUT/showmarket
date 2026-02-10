# showmarket

一个基于 **Axum + Tokio** 的 Rust Web 服务示例项目，提供：

- **REST**：`GET /health`
- **REST**：`GET /price/btc`
- **WebSocket**：`GET /ws/prices`
- **后台任务**：定时从 Binance 拉取 **BTCUSDT** 最新价格，并通过广播推送给所有 WebSocket 客户端

## 功能说明

- **健康检查**：`GET /health`
  - 返回：`{"status":"ok"}`
- **获取 BTC 最新价格**：`GET /price/btc`
  - 当后台尚未拉到价格时，返回 `503 Service Unavailable`
  - 拉到价格后，返回 JSON（示例）：
    - `{"symbol":"BTCUSDT","price":12345.67,"ts_ms":1700000000000}`
- **实时价格推送**：`GET /ws/prices`
  - 连接建立后：
    - 若已有最新价格，会先推送 1 条最新价格
    - 后续持续推送后台任务拉到的价格更新
  - 注意：使用 `broadcast` 进行推送，若客户端消费太慢可能会丢失部分更新（对行情推送通常是可接受的）

## 项目结构（现代 module 布局，无 `mod.rs`）

```
src/
  main.rs            # 启动 HTTP 服务 + 后台抓价任务
  lib.rs             # app(state) 路由组装，便于测试
  state.rs           # AppState：latest + broadcast sender
  handlers.rs        # handlers 模块入口（无 mod.rs）
  handlers/
    health.rs
    price.rs
    ws.rs
  services.rs        # services 模块入口（无 mod.rs）
  services/
    binance.rs        # Binance HTTP 拉价
  models.rs          # models 模块入口（无 mod.rs）
  models/
    price.rs
tests/
  auxm_api.rs         # 集成测试（health / price）
```

## 运行

```bash
cargo run
```

默认监听：`127.0.0.1:3000`

示例请求：

```bash
curl -s http://127.0.0.1:3000/health
curl -s http://127.0.0.1:3000/price/btc
```

WebSocket（示例，使用 websocat）：

```bash
websocat ws://127.0.0.1:3000/ws/prices
```

## 测试

```bash
cargo test
```

当前集成测试覆盖：

- `/health` 返回 200
- `/price/btc` 在未准备好时返回 503
- `/price/btc` 在写入最新价格后返回 200
