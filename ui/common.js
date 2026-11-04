// 两个窗口共用的工具函数
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const AVATAR_COLORS = [
  "#5b7cfa", "#7c5cf5", "#d65db1", "#ef6f6c", "#f59e0b",
  "#22a06b", "#14b8a6", "#0ea5e9", "#6366f1", "#e11d48",
];

function avatarColor(text) {
  let h = 0;
  for (const ch of text) h = (h * 31 + ch.codePointAt(0)) >>> 0;
  return AVATAR_COLORS[h % AVATAR_COLORS.length];
}

function avatarEl(title) {
  const el = document.createElement("div");
  el.className = "avatar";
  el.style.background = avatarColor(title);
  el.textContent = [...title.trim()][0] || "?";
  return el;
}

/**
 * 模糊匹配标题：前缀 > 包含 > 子序列（连续命中加分）。
 * 返回 { score, hits }，hits 为命中字符的下标，用于高亮。
 */
function fuzzyMatch(query, title) {
  const q = [...query.toLowerCase()];
  const t = [...title.toLowerCase()];
  const qs = q.join(""), ts = t.join("");
  let score = 0;
  let hits = [];
  const pos = ts.indexOf(qs);
  if (pos >= 0) {
    score += pos === 0 ? 100 : 50;
    const start = [...ts.slice(0, pos)].length;
    hits = q.map((_, i) => start + i);
  }
  let qi = 0, streak = 0, best = 0;
  const seq = [];
  for (let i = 0; i < t.length && qi < q.length; i++) {
    if (t[i] === q[qi]) {
      seq.push(i);
      qi++;
      best = Math.max(best, ++streak);
    } else {
      streak = 0;
    }
  }
  if (qi === q.length) {
    score += 10 + best;
    if (!hits.length) hits = seq;
  }
  return { score, hits };
}

function highlight(text, hits) {
  const frag = document.createDocumentFragment();
  const set = new Set(hits);
  let buf = "", inHit = false;
  const flush = () => {
    if (!buf) return;
    if (inHit) {
      const m = document.createElement("mark");
      m.textContent = buf;
      frag.appendChild(m);
    } else {
      frag.appendChild(document.createTextNode(buf));
    }
    buf = "";
  };
  [...text].forEach((ch, i) => {
    const h = set.has(i);
    if (h !== inHit) { flush(); inHit = h; }
    buf += ch;
  });
  flush();
  return frag;
}

function svgIcon(name) {
  const paths = {
    search: '<circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/>',
    copy: '<rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15V6a2 2 0 0 1 2-2h9"/>',
    edit: '<path d="M4 20h4L19 9l-4-4L4 16z"/><path d="m13.5 6.5 4 4"/>',
    trash: '<path d="M4 7h16"/><path d="M9 7V4h6v3"/><path d="M6 7l1 13h10l1-13"/>',
    plus: '<path d="M12 5v14M5 12h14"/>',
    eye: '<path d="M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z"/><circle cx="12" cy="12" r="3"/>',
    eyeOff: '<path d="M3 3l18 18"/><path d="M10.6 5.1A10 10 0 0 1 12 5c6.5 0 10 7 10 7a17 17 0 0 1-3.2 4.2M6.6 6.6A17 17 0 0 0 2 12s3.5 7 10 7a9.7 9.7 0 0 0 5.4-1.6"/><path d="M9.9 9.9a3 3 0 0 0 4.2 4.2"/>',
    dice: '<rect x="4" y="4" width="16" height="16" rx="3"/><circle cx="9" cy="9" r="1.2" fill="currentColor"/><circle cx="15" cy="15" r="1.2" fill="currentColor"/><circle cx="15" cy="9" r="1.2" fill="currentColor"/><circle cx="9" cy="15" r="1.2" fill="currentColor"/>',
    check: '<path d="m5 12 5 5 9-10"/>',
    lock: '<rect x="5" y="11" width="14" height="10" rx="2"/><path d="M8 11V8a4 4 0 0 1 8 0v3"/>',
    user: '<circle cx="12" cy="8" r="4"/><path d="M4 21a8 8 0 0 1 16 0"/>',
    alert: '<path d="M12 3 2 21h20z"/><path d="M12 10v5M12 18v.01"/>',
    min: '<path d="M5 12h14"/>',
    max: '<rect x="5" y="5" width="14" height="14" rx="1.5"/>',
    close: '<path d="m6 6 12 12M18 6 6 18"/>',
    gear: '<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z"/>',
  };
  return `<svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">${paths[name] || ""}</svg>`;
}

// 禁用 WebView 默认右键菜单和刷新快捷键，让它更像原生应用
document.addEventListener("contextmenu", (e) => {
  if (!(e.target instanceof HTMLInputElement)) e.preventDefault();
});
document.addEventListener("keydown", (e) => {
  if (e.key === "F5" || (e.ctrlKey && e.key.toLowerCase() === "r")) e.preventDefault();
});
