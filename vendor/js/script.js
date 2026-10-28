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

  socket.onclose = () => {
    // Close the browser window.
    window.open('', '_self', '');
    window.close();
  }
}

document.addEventListener('DOMContentLoaded', () => {
  setup()
  setupWebSockets()
});
