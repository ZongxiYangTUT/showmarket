use axum::response::Html as AxumHtml;
use yew::prelude::*;

const INLINE_WS_JS: &str = r#"
      const priceEl = document.getElementById('price');
      const statusDotEl = document.getElementById('status-dot');
      const statusTextEl = document.getElementById('status-text');
      const timeEl = document.getElementById('time');

      // 按 symbol 缓存最近一次价格和时间，方便切换指数时立即展示
      const lastPriceBySymbol = {};
      const lastTimeBySymbol = {};

      function formatTs(tsMs) {
        if (!tsMs) return '';
        const d = new Date(tsMs);
        return d.toLocaleString();
      }

      // 供 K 线脚本在切换 currentSymbol 时调用
      window.__setRealtimePriceFromCache = function (symbol) {
        const p = lastPriceBySymbol[symbol];
        const ts = lastTimeBySymbol[symbol];
        if (typeof p === 'number') {
          priceEl.textContent = p.toFixed(2);
          timeEl.textContent = ts ? '更新时间：' + formatTs(ts) : '';
          statusTextEl.textContent = '实时价格推送中';
        } else {
          priceEl.textContent = '--.--';
          timeEl.textContent = '';
          statusTextEl.textContent = '等待该指数价格推送...';
        }
      };

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
            if (!data || typeof data.price === 'undefined') {
              return;
            }

            const sym = data.symbol || window.currentSymbol || '000001.SH';

            let priceNum;
            if (typeof data.price === 'number') {
              priceNum = data.price;
            } else if (typeof data.price === 'string') {
              const n = Number(data.price);
              if (Number.isNaN(n)) return;
              priceNum = n;
            } else {
              return;
            }

            lastPriceBySymbol[sym] = priceNum;
            lastTimeBySymbol[sym] = data.ts_ms || null;

            if (sym === window.currentSymbol || (!window.currentSymbol && sym === '000001.SH')) {
              priceEl.textContent = priceNum.toFixed(2);
              timeEl.textContent = data.ts_ms
                ? '更新时间：' + formatTs(data.ts_ms)
                : '';
              statusTextEl.textContent = '实时价格推送中';
            }
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

const INLINE_KLINE_JS: &str = r#"
      let currentSymbol = '000001.SH';
      window.currentSymbol = currentSymbol;
      let currentInterval = '1m';
      let lastKlines = [];
      let hoverIndex = null;
      let crossX = null;
      let crossY = null;
      let viewStart = 0;
      let viewCount = 120;
      let isDragging = false;
      let dragStartX = 0;
      let dragStartViewStart = 0;
      let refreshTimer = null;

      function computeMa(data, period) {
        const out = new Array(data.length).fill(null);
        let sum = 0;
        for (let i = 0; i < data.length; i++) {
          sum += data[i].close;
          if (i >= period) {
            sum -= data[i - period].close;
          }
          if (i >= period - 1) {
            out[i] = sum / period;
          }
        }
        return out;
      }

      function initKline() {
        const symbolTabs = document.querySelectorAll('[data-symbol-tab]');
        symbolTabs.forEach((tab) => {
          tab.addEventListener('click', () => {
            const sym = tab.getAttribute('data-symbol-tab');
            if (!sym || sym === currentSymbol) return;
            currentSymbol = sym;
            window.currentSymbol = currentSymbol;
            updateActiveTabs();
            loadKlines(true);
          });
        });

        const intervalTabs = document.querySelectorAll('[data-interval-tab]');
        intervalTabs.forEach((tab) => {
          tab.addEventListener('click', () => {
            const itv = tab.getAttribute('data-interval-tab');
            if (!itv || itv === currentInterval) return;
            currentInterval = itv;
            updateActiveTabs();
            loadKlines(true);
          });
        });

        const canvas = document.getElementById('kline-canvas');
        if (canvas) {
          canvas.addEventListener('mousemove', (e) => {
            if (!lastKlines.length) return;
            const rect = canvas.getBoundingClientRect();
            const x = e.clientX - rect.left;
            const y = e.clientY - rect.top;
            crossX = x;
            crossY = y;
            const paddingLeft = 60;
            const paddingRight = 60;
            const plotW = rect.width - paddingLeft - paddingRight;
            if (plotW <= 0) return;

            if (isDragging) {
              const total = lastKlines.length;
              const deltaX = e.clientX - dragStartX;
              // 再进一步降低拖动速度（0.25 系数，越拖越细）
              const barsDelta = Math.round((deltaX / plotW) * viewCount * 0.25);
              const maxStart = Math.max(0, total - viewCount);
              viewStart = dragStartViewStart - barsDelta;
              if (viewStart < 0) viewStart = 0;
              if (viewStart > maxStart) viewStart = maxStart;
              drawKlines(canvas, lastKlines);
              return;
            }

            const total = lastKlines.length;
            const maxStart = Math.max(0, total - viewCount);
            if (viewStart > maxStart) viewStart = maxStart;
            const start = viewStart;
            const visibleCount = Math.min(viewCount, total - start);
            if (visibleCount <= 0) return;

            const step = plotW / visibleCount;
            const idxInView = Math.floor((x - paddingLeft) / step);
            const idx = start + idxInView;
            if (idxInView < 0 || idxInView >= visibleCount) {
              hoverIndex = null;
            } else {
              hoverIndex = idx;
            }
            drawKlines(canvas, lastKlines);
          });

          canvas.addEventListener('mouseleave', () => {
            hoverIndex = null;
            crossX = null;
            crossY = null;
            isDragging = false;
            drawKlines(canvas, lastKlines);
          });

          canvas.addEventListener('mousedown', (e) => {
            if (!lastKlines.length) return;
            isDragging = true;
            dragStartX = e.clientX;
            dragStartViewStart = viewStart;
          });

          window.addEventListener('mouseup', () => {
            isDragging = false;
          });

          canvas.addEventListener('wheel', (e) => {
            if (!lastKlines.length) return;
            e.preventDefault();
            const total = lastKlines.length;
            const rect = canvas.getBoundingClientRect();
            const paddingLeft = 60;
            const paddingRight = 60;
            const plotW = rect.width - paddingLeft - paddingRight;
            if (plotW <= 0) return;

            const minView = 20;
            const maxView = total;
            let factor = e.deltaY > 0 ? 1.2 : 0.8;
            let newViewCount = Math.round(viewCount * factor);
            if (newViewCount < minView) newViewCount = minView;
            if (newViewCount > maxView) newViewCount = maxView;

            const x = e.clientX - rect.left;
            const centerRatio = Math.max(
              0,
              Math.min(1, (x - paddingLeft) / plotW)
            );
            const centerIndex = viewStart + centerRatio * viewCount;

            viewCount = newViewCount;
            viewStart = Math.round(centerIndex - viewCount / 2);
            const maxStart = Math.max(0, total - viewCount);
            if (viewStart < 0) viewStart = 0;
            if (viewStart > maxStart) viewStart = maxStart;

            drawKlines(canvas, lastKlines);
          });
        }

        updateActiveTabs();
        loadKlines();
      }

      function updateActiveTabs() {
        const symbolTabs = document.querySelectorAll('[data-symbol-tab]');
        symbolTabs.forEach((tab) => {
          const sym = tab.getAttribute('data-symbol-tab');
          if (sym === currentSymbol) {
            tab.classList.add('active');
          } else {
            tab.classList.remove('active');
          }
        });

        const intervalTabs = document.querySelectorAll('[data-interval-tab]');
        intervalTabs.forEach((tab) => {
          const itv = tab.getAttribute('data-interval-tab');
          if (itv === currentInterval) {
            tab.classList.add('active');
          } else {
            tab.classList.remove('active');
          }
        });

        const symbolLabel = document.getElementById('chart-symbol');
        if (symbolLabel) symbolLabel.textContent = currentSymbol;
        const symbolLabelTop = document.getElementById('chart-symbol-label');
        if (symbolLabelTop) {
          const active = document.querySelector('[data-symbol-tab].active');
          if (active) {
            symbolLabelTop.textContent = active.textContent || currentSymbol;
          } else {
            symbolLabelTop.textContent = currentSymbol;
          }
        }

        if (window.__setRealtimePriceFromCache) {
          window.__setRealtimePriceFromCache(currentSymbol);
        }
        const intervalLabel = document.getElementById('chart-interval');
        if (intervalLabel) intervalLabel.textContent = currentInterval;
      }

      async function loadKlines(resetView = true) {
        const canvas = document.getElementById('kline-canvas');
        if (!canvas) return;
        const ctx = canvas.getContext('2d');
        ctx.clearRect(0, 0, canvas.width, canvas.height);

        if (refreshTimer !== null) {
          clearTimeout(refreshTimer);
          refreshTimer = null;
        }

        try {
          const res = await fetch(
            `/api/klines/${currentSymbol}?interval=${encodeURIComponent(
              currentInterval
            )}`
          );
          if (!res.ok) {
            throw new Error('HTTP ' + res.status);
          }
          const data = await res.json();
          lastKlines = Array.isArray(data) ? data : [];
          hoverIndex = null;
          const total = lastKlines.length;
          if (resetView && total > 0) {
            const defaultCount = Math.min(150, total);
            viewCount = defaultCount;
            viewStart = Math.max(0, total - viewCount);
          }
          drawKlines(canvas, lastKlines);
        } catch (e) {
          console.error('failed to load klines', e);
          ctx.fillStyle = '#9ca3af';
          ctx.font = '12px system-ui, sans-serif';
          ctx.fillText('无法加载K线数据', 20, 30);
        }
      }

      function drawKlines(canvas, data) {
        if (!Array.isArray(data) || data.length === 0) {
          return;
        }
        const rect = canvas.getBoundingClientRect();
        const dpr = window.devicePixelRatio || 1;
        canvas.width = rect.width * dpr;
        canvas.height = rect.height * dpr;
        const ctx = canvas.getContext('2d');
        ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
        const w = rect.width;
        const h = rect.height;

        const paddingLeft = 60;
        const paddingRight = 60;
        const paddingTop = 20;
        const paddingBottom = 36;
        const plotW = w - paddingLeft - paddingRight;
        const plotH = h - paddingTop - paddingBottom;
        const pricePlotH = plotH * 0.7;
        const volumePlotH = plotH * 0.25;
        const priceBottom = paddingTop + pricePlotH;
        const volumeTop = priceBottom + 8;
        const volumeBottom = h - paddingBottom;

        const total = data.length;
        if (!viewCount || viewCount > total) {
          viewCount = Math.min(150, total);
        }
        if (viewCount < 1) viewCount = 1;
        const maxStart = Math.max(0, total - viewCount);
        if (viewStart < 0) viewStart = 0;
        if (viewStart > maxStart) viewStart = maxStart;
        const start = viewStart;
        const end = Math.min(total, start + viewCount);
        const visibleCount = end - start;
        if (visibleCount <= 0) return;

        const highs = data.slice(start, end).map((d) => d.high);
        const lows = data.slice(start, end).map((d) => d.low);
        let max = Math.max.apply(null, highs);
        let min = Math.min.apply(null, lows);
        const rangeBase = max - min || 1;

        const ma7 = computeMa(data, 7);
        const ma25 = computeMa(data, 25);
        const ma99 = computeMa(data, 99);
        const maAll = []
          .concat(ma7, ma25, ma99)
          .filter((v) => typeof v === 'number' && !Number.isNaN(v));
        if (maAll.length) {
          const maMax = Math.max.apply(null, maAll);
          const maMin = Math.min.apply(null, maAll);
          max = Math.max(max, maMax);
          min = Math.min(min, maMin);
        }
        const range = max - min || rangeBase;

        ctx.fillStyle = '#020617';
        ctx.fillRect(0, 0, w, h);

        ctx.strokeStyle = '#1f2937';
        ctx.beginPath();
        ctx.moveTo(paddingLeft, paddingTop);
        ctx.lineTo(paddingLeft, h - paddingBottom);
        ctx.lineTo(w - paddingRight, h - paddingBottom);
        ctx.stroke();

        const n = visibleCount;
        const step = plotW / n;
        const candleW = Math.max(2, step * 0.6);

        // candles
        for (let i = 0; i < n; i++) {
          const d = data[start + i];
          const xCenter = paddingLeft + step * i + step / 2;

          const yHigh = paddingTop + ((max - d.high) / range) * pricePlotH;
          const yLow = paddingTop + ((max - d.low) / range) * pricePlotH;
          const yOpen = paddingTop + ((max - d.open) / range) * pricePlotH;
          const yClose = paddingTop + ((max - d.close) / range) * pricePlotH;

          const isUp = d.close >= d.open;
          const color = isUp ? '#22c55e' : '#ef4444';

          ctx.strokeStyle = color;
          ctx.beginPath();
          ctx.moveTo(xCenter, yHigh);
          ctx.lineTo(xCenter, yLow);
          ctx.stroke();

          const rectTop = isUp ? yClose : yOpen;
          const rectBottom = isUp ? yOpen : yClose;
          const rectHeight = Math.max(1, rectBottom - rectTop);

          ctx.fillStyle = color;
          ctx.fillRect(xCenter - candleW / 2, rectTop, candleW, rectHeight);
        }

        // volume bars
        const volumes = data.slice(start, end).map((d) => d.volume);
        const maxVol = Math.max.apply(null, volumes);
        const volRange = maxVol || 1;
        for (let i = 0; i < n; i++) {
          const d = data[start + i];
          const xCenter = paddingLeft + step * i + step / 2;
          const v = d.volume;
          const barBottom = volumeBottom;
          const barTop =
            volumeBottom -
            (v / volRange) * Math.max(10, volumePlotH);
          const isUp = d.close >= d.open;
          const colorVol = isUp
            ? 'rgba(34,197,94,0.6)'
            : 'rgba(239,68,68,0.6)';
          ctx.fillStyle = colorVol;
          ctx.fillRect(
            xCenter - candleW / 2,
            barTop,
            candleW,
            Math.max(1, barBottom - barTop)
          );
        }

        // moving average lines
        function drawMaLine(values, color) {
          ctx.strokeStyle = color;
          ctx.lineWidth = 1;
          ctx.beginPath();
          let started = false;
          for (let i = 0; i < n; i++) {
            const v = values[start + i];
            if (v == null || Number.isNaN(v)) continue;
            const xCenter = paddingLeft + step * i + step / 2;
            const y =
              paddingTop + ((max - v) / range) * pricePlotH;
            if (!started) {
              ctx.moveTo(xCenter, y);
              started = true;
            } else {
              ctx.lineTo(xCenter, y);
            }
          }
          if (started) ctx.stroke();
        }

        drawMaLine(ma7, '#facc15');
        drawMaLine(ma25, '#a855f7');
        drawMaLine(ma99, '#22d3ee');

        // hover crosshair + tooltip / price box
        let crossPrice = null;
        let crossXDraw = null;
        let crossYDraw = null;

        // 如果悬停在某一根 K 线上，显示该 K 的 OHLC 明细（十字位置交给鼠标控制）
        if (hoverIndex != null && hoverIndex >= start && hoverIndex < end) {
          const idx = hoverIndex;
          const i = idx - start;
          const d = data[idx];
          const xCenter = paddingLeft + step * i + step / 2;
          const yClose =
            paddingTop + ((max - d.close) / range) * pricePlotH;

          const boxWidth = 160;
          const boxHeight = 70;
          let boxX = xCenter + 10;
          if (boxX + boxWidth > w - 10) {
            boxX = xCenter - boxWidth - 10;
          }
          const boxY = Math.max(
            paddingTop + 4,
            Math.min(
              yClose - boxHeight / 2,
              h - paddingBottom - boxHeight - 4
            )
          );

          ctx.fillStyle = 'rgba(15, 23, 42, 0.95)';
          ctx.strokeStyle = 'rgba(148, 163, 184, 0.6)';
          ctx.lineWidth = 1;
          ctx.beginPath();
          ctx.roundRect(boxX, boxY, boxWidth, boxHeight, 6);
          ctx.fill();
          ctx.stroke();

          ctx.fillStyle = '#e5e7eb';
          ctx.font = '10px system-ui, sans-serif';
          ctx.textAlign = 'left';
          ctx.textBaseline = 'top';
          ctx.fillText('O: ' + d.open.toFixed(2), boxX + 8, boxY + 4);
          ctx.fillText('H: ' + d.high.toFixed(2), boxX + 8, boxY + 16);
          ctx.fillText('L: ' + d.low.toFixed(2), boxX + 8, boxY + 28);
          ctx.fillText('C: ' + d.close.toFixed(2), boxX + 8, boxY + 40);
          ctx.fillText('V: ' + d.volume.toFixed(2), boxX + 8, boxY + 52);
        }

        // 若有鼠标位置，允许在任意位置悬浮十字，并计算对应价格
        if (crossX != null && crossY != null) {
          const clampedY = Math.max(
            paddingTop,
            Math.min(priceBottom, crossY)
          );
          const rel = (clampedY - paddingTop) / pricePlotH;
          const priceFromY = max - rel * (max - min || 1);
          crossPrice = priceFromY;
          crossYDraw = clampedY;

          // 竖直线尽量吸附到当前 hover 的 K 线中心，否则跟随鼠标 X
          let xDraw = crossX;
          if (hoverIndex != null && hoverIndex >= start && hoverIndex < end) {
            const i = hoverIndex - start;
            xDraw = paddingLeft + step * i + step / 2;
          }
          crossXDraw = xDraw;
        }

        // 画十字和右侧价格/百分比浮动框
        if (crossPrice != null && crossXDraw != null && crossYDraw != null) {
          ctx.strokeStyle = 'rgba(148, 163, 184, 0.8)';
          ctx.setLineDash([4, 4]);
          ctx.beginPath();
          ctx.moveTo(crossXDraw, paddingTop);
          ctx.lineTo(crossXDraw, h - paddingBottom);
          ctx.stroke();

          ctx.beginPath();
          ctx.moveTo(paddingLeft, crossYDraw);
          ctx.lineTo(w - paddingRight, crossYDraw);
          ctx.stroke();
          ctx.setLineDash([]);

          const refPrice = data[end - 1]?.close || crossPrice;
          const boxWidth = 110;
          const boxHeight = 34;
          // 完全放在绘图区内部，避免被右侧边界遮挡
          let boxX = w - paddingRight - boxWidth + 8;
          let boxY = crossYDraw - boxHeight / 2;
          if (boxY < paddingTop + 4) {
            boxY = paddingTop + 4;
          }
          if (boxY + boxHeight > h - paddingBottom - 4) {
            boxY = h - paddingBottom - boxHeight - 4;
          }

          ctx.fillStyle = 'rgba(15, 23, 42, 0.95)';
          ctx.strokeStyle = 'rgba(148, 163, 184, 0.6)';
          ctx.lineWidth = 1;
          ctx.beginPath();
          ctx.roundRect(boxX, boxY, boxWidth, boxHeight, 6);
          ctx.fill();
          ctx.stroke();

          ctx.textAlign = 'right';
          ctx.textBaseline = 'top';
          ctx.font = '11px system-ui, sans-serif';
          ctx.fillStyle = '#e5e7eb';
          const priceText = crossPrice.toFixed(2);
          ctx.fillText(priceText, boxX + boxWidth - 6, boxY + 3);

          if (refPrice > 0) {
            const pct = ((crossPrice - refPrice) / refPrice) * 100;
            const pctText =
              (pct >= 0 ? '+' : '') + pct.toFixed(2) + '%';
            ctx.textBaseline = 'bottom';
            ctx.fillStyle = pct >= 0 ? '#4ade80' : '#f97373';
            ctx.fillText(
              pctText,
              boxX + boxWidth - 6,
              boxY + boxHeight - 4
            );
          }
        }

          ctx.fillStyle = '#9ca3af';
          ctx.font = '10px system-ui, sans-serif';
          ctx.textAlign = 'right';
          ctx.textBaseline = 'middle';
          const priceTicks = 4;
          for (let i = 0; i < priceTicks; i++) {
            const t = i / (priceTicks - 1);
            const price = max - t * (max - min);
            const y =
              paddingTop + ((max - price) / (max - min || 1)) * pricePlotH;
            ctx.fillText(
              price.toFixed(2),
              w - 6,
              y
            );
          }

          // time axis
          ctx.textAlign = 'center';
          ctx.textBaseline = 'top';
          ctx.fillStyle = '#6b7280';
          // 先给一个上限，然后再用像素间距做第二层过滤，避免重叠
          const maxLabels = Math.max(2, Math.floor(plotW / 80));
          const labelCount = Math.min(maxLabels, n);
          const firstTime = data[start].open_time;
          const lastTime = data[end - 1].open_time;
          const spanMs = Math.max(0, lastTime - firstTime);

          const oneHour = 60 * 60 * 1000;
          const oneDay = 24 * oneHour;
          // 根据时间跨度和标签内容估算最小像素间距
          let minSpacing;
          if (spanMs <= 6 * oneHour) {
            // 短周期，标签较短，如 12:30
            minSpacing = 60;
          } else if (spanMs <= 7 * oneDay) {
            // 中周期，标签较长，如 02-10 12:30
            minSpacing = 110;
          } else {
            // 长周期，仅日期，如 02-10
            minSpacing = 80;
          }

          let lastLabelX = -Infinity;
          for (let i = 0; i < labelCount; i++) {
            const ratio = labelCount === 1 ? 0 : i / (labelCount - 1);
            const idx = start + Math.round((n - 1) * ratio);
            const d = data[idx];
            const date = new Date(d.open_time);
            let label;
            if (spanMs <= 6 * oneHour) {
              // 短周期：只显示时:分
              const hh = String(date.getHours()).padStart(2, '0');
              const mm = String(date.getMinutes()).padStart(2, '0');
              label = hh + ':' + mm;
            } else if (spanMs <= 7 * oneDay) {
              // 中周期：显示 月-日 和 时:分（仍然相对紧凑）
              const month = String(date.getMonth() + 1).padStart(2, '0');
              const day = String(date.getDate()).padStart(2, '0');
              const hh = String(date.getHours()).padStart(2, '0');
              const mm = String(date.getMinutes()).padStart(2, '0');
              label = month + '-' + day + ' ' + hh + ':' + mm;
            } else {
              // 长周期：只显示月-日，避免过长
              const month = String(date.getMonth() + 1).padStart(2, '0');
              const day = String(date.getDate()).padStart(2, '0');
              label = month + '-' + day;
            }
            const x = paddingLeft + step * i + step / 2;
            const y = h - paddingBottom + 4;
            if (x - lastLabelX < minSpacing) {
              continue;
            }
            ctx.fillText(label, x, y);
            lastLabelX = x;
          }
      }

      window.addEventListener('load', initKline);
"#;
#[function_component(App)]
fn app() -> Html {
    html! {
        <html lang="en">
          <head>
            <meta charset="utf-8" />
            <title>{ "A股指数实时行情" }</title>
            <link rel="stylesheet" href="/static/style.css" />
          </head>
          <body class="page">
            <div class="layout">
              <div class="card">
                <div>
                  <div class="label">{ "Realtime Index" }</div>
                  <div class="symbol" id="chart-symbol-label">{ "上证指数 000001.SH" }</div>
                </div>
                <div class="price" id="price">{ "--.--" }</div>
                <div class="status-row">
                  <div class="status">
                    <span id="status-dot" class="dot"></span>
                    <span id="status-text">{ "正在连接 WebSocket..." }</span>
                  </div>
                  <div id="time" class="time"></div>
                </div>
              </div>

              <div class="panel">
                <div class="panel-header">
                  <div class="panel-title">
                    <span class="panel-title-main" id="chart-symbol">{ "000001.SH" }</span>
                    <span class="panel-title-sub" id="chart-interval">{ "1m" }</span>
                  </div>
                  <div class="legend">
                    <div class="legend-item">
                      <span class="legend-dot ma7"></span>
                      <span>{ "MA7" }</span>
                    </div>
                    <div class="legend-item">
                      <span class="legend-dot ma25"></span>
                      <span>{ "MA25" }</span>
                    </div>
                    <div class="legend-item">
                      <span class="legend-dot ma99"></span>
                      <span>{ "MA99" }</span>
                    </div>
                  </div>
                </div>
                <div class="symbol-tabs">
                  <button class="tab active" data-symbol-tab="000001.SH">{ "上证指数 000001.SH" }</button>
                  <button class="tab" data-symbol-tab="399001.SZ">{ "深证成指 399001.SZ" }</button>
                  <button class="tab" data-symbol-tab="399006.SZ">{ "创业板指 399006.SZ" }</button>
                </div>
                <div class="interval-tabs">
                  <span class="interval-label">{ "周期" }</span>
                  <button class="interval-tab active" data-interval-tab="1m">{ "1m" }</button>
                  <button class="interval-tab" data-interval-tab="5m">{ "5m" }</button>
                  <button class="interval-tab" data-interval-tab="15m">{ "15m" }</button>
                  <button class="interval-tab" data-interval-tab="1h">{ "1h" }</button>
                  <button class="interval-tab" data-interval-tab="4h">{ "4h" }</button>
                  <button class="interval-tab" data-interval-tab="1d">{ "1d" }</button>
                  <button class="interval-tab" data-interval-tab="1w">{ "1w" }</button>
                </div>
                <div class="chart-container">
                  <canvas
                    id="kline-canvas"
                    class="chart-canvas"
                    width="960"
                    height="360"
                  >
                  </canvas>
                </div>
              </div>
            </div>
            <script>{ INLINE_WS_JS }</script>
            <script>{ INLINE_KLINE_JS }</script>
          </body>
        </html>
    }
}

pub async fn index() -> AxumHtml<String> {
    let rendered = yew::ServerRenderer::<App>::new().render().await;
    AxumHtml(rendered)
}
