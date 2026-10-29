function setupDiagrams() {
  if (typeof mermaid === 'undefined') {
    return
  }

  mermaid.initialize({ startOnLoad: false })
  const e = document.querySelectorAll("code.language-mermaid");
  let n = 0;
  for (const t of e) {
    const id = `mermaid${n}`;
    n++;
    const d = t.textContent;
    // Mermaid 10+ renders asynchronously and resolves to the SVG markup.
    mermaid.render(id, d)
      .then(({ svg }) => { t.innerHTML = svg })
      .catch((err) => { console.error(`mermaid: ${err}`) })
  }
}

function setupMath() {
  if (typeof katex === 'undefined') {
    return
  }

  for (const e of document.querySelectorAll(".language-math")) {
    katex.render(e.textContent, e)
  }
}

function setupHighlight() {
  if (typeof hljs === 'undefined') {
    return
  }

  hljs.highlightAll();
}

function setup() {
  setupMath()
  setupDiagrams()
  setupHighlight()
}

function setupWebSockets() {
  if (typeof ReconnectingWebSocket === 'undefined') {
    return
  }

  var scheme = window.location.protocol === 'https:' ? 'wss://' : 'ws://';
  var webSocketUrl = scheme + window.location.host;

  var socket = new ReconnectingWebSocket(webSocketUrl);
  socket.maxReconnectInterval = 5000;

  socket.onmessage = event => {
    document.getElementById('root').innerHTML = event.data;
    setup()
  }

  // ReconnectingWebSocket fires onclose on every disconnect and then retries,
  // so show a marker while the connection is down instead of closing the tab.
  socket.onopen = () => {
    var marker = document.getElementById('marky-reconnecting');
    if (marker) {
      marker.remove();
    }
  }

  socket.onclose = () => {
    if (document.getElementById('marky-reconnecting')) {
      return
    }

    var marker = document.createElement('div');
    marker.id = 'marky-reconnecting';
    marker.textContent = 'reconnecting\u2026';
    marker.style.cssText = 'position:fixed;top:0.5em;right:0.5em;padding:0.2em 0.6em;'
      + 'font:12px sans-serif;background:#333;color:#fff;opacity:0.8;border-radius:4px;z-index:9999';
    document.body.appendChild(marker);
  }
}

document.addEventListener('DOMContentLoaded', () => {
  setup()
  setupWebSockets()
});
