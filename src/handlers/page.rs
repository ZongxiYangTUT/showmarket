use axum::response::Html as AxumHtml;
use yew::prelude::*;

const INLINE_WS_JS: &str = r#"
      const priceEl = document.getElementById('price');
      const statusDotEl = document.getElementById('status-dot');
      const statusTextEl = document.getElementById('status-text');
      const timeEl = document.getElementById('time');

      function formatTs(tsMs) {
        if (!tsMs) return '';
        const d = new Date(tsMs);
        return d.toLocaleString();
      }

      function connect() {
        const wsUrl =
          (location.protocol === 'https:' ? 'wss://' : 'ws://') +
          location.host +
          '/ws/prices';
        const ws = new WebSocket(wsUrl);

        ws.onopen = () => {
          statusDotEl.classList.add('connected');
          statusTextEl.textContent = '已连接，等待价格更新...';
        };

        ws.onmessage = (event) => {
          try {
            const data = JSON.parse(event.data);
            if (typeof data.price === 'number') {
              priceEl.textContent = data.price.toFixed(2);
            } else if (typeof data.price === 'string') {
              const n = Number(data.price);
              if (!Number.isNaN(n)) {
                priceEl.textContent = n.toFixed(2);
              }
            }
            timeEl.textContent = data.ts_ms
              ? '更新时间：' + formatTs(data.ts_ms)
              : '';
            statusTextEl.textContent = '实时价格推送中';
          } catch (e) {
            console.error('invalid message', e);
          }
        };

        ws.onclose = () => {
          statusDotEl.classList.remove('connected');
          statusTextEl.textContent = '连接已断开，正在重连...';
          setTimeout(connect, 2000);
        };

        ws.onerror = () => {
          statusDotEl.classList.remove('connected');
          statusTextEl.textContent = '连接出错，将尝试重连...';
        };
      }

      connect();
"#;

#[function_component(App)]
fn app() -> Html {
    html! {
        <html lang="en">
          <head>
            <meta charset="utf-8" />
            <title>{ "BTCUSDT Realtime Price" }</title>
            <link rel="stylesheet" href="/static/style.css" />
          </head>
          <body class="page">
            <div class="card">
              <div class="label">{ "Realtime Price" }</div>
              <div class="symbol">{ "BTC / USDT" }</div>
              <div id="price" class="price">{ "--.--" }</div>
              <div class="status">
                <span id="status-dot" class="dot"></span>
                <span id="status-text">{ "正在连接 WebSocket..." }</span>
              </div>
              <div id="time" class="time"></div>
            </div>
            <script>{ INLINE_WS_JS }</script>
          </body>
        </html>
    }
}

pub async fn index() -> AxumHtml<String> {
    let rendered = yew::ServerRenderer::<App>::new().render().await;
    AxumHtml(rendered)
}
