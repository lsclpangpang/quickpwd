const $q = document.getElementById("q");
const $list = document.getElementById("list");
const $count = document.getElementById("count");
const $status = document.getElementById("status");
const $statusText = document.getElementById("status-text");
const $done = document.getElementById("done");

document.getElementById("search-icon").innerHTML = svgIcon("search");
document.getElementById("logo").innerHTML = svgIcon("lock");
document.getElementById("done-badge").innerHTML = svgIcon("check");

let entries = [];
let matches = [];
let sel = 0;
let busy = false;

async function reload() {
  try {
    entries = await invoke("list_titles");
  } catch (e) {
    entries = [];
    setStatus(String(e), true);
  }
  render();
}

function setStatus(text, isErr = false) {
  $statusText.textContent = text;
  $status.classList.toggle("err", isErr);
}

function render() {
  const query = $q.value.trim();
  if (query) {
    matches = entries
      .map((t) => ({ t, ...fuzzyMatch(query, t) }))
      .filter((m) => m.score > 0)
      .sort((a, b) => b.score - a.score || a.t.localeCompare(b.t));
  } else {
    matches = entries.map((t) => ({ t, hits: [] }));
  }
  sel = Math.min(sel, Math.max(0, matches.length - 1));
  $count.textContent = entries.length ? `${matches.length} / ${entries.length}` : "";
  $list.replaceChildren();

  if (!matches.length) {
    $list.appendChild(emptyState(query));
    return;
  }

  const header = document.createElement("div");
  header.className = "section";
  header.textContent = query ? "匹配结果" : "全部条目";
  $list.appendChild(header);

  matches.forEach((m, i) => {
    const row = document.createElement("div");
    row.className = "row" + (i === sel ? " sel" : "");
    row.appendChild(avatarEl(m.t));

    const title = document.createElement("div");
    title.className = "title";
    title.appendChild(highlight(m.t, m.hits));
    row.appendChild(title);

    const action = document.createElement("div");
    action.className = "action";
    action.innerHTML = "复制 <kbd>↵</kbd>";
    row.appendChild(action);

    row.addEventListener("mousemove", () => {
      if (sel !== i) select(i, false);
    });
    row.addEventListener("click", () => {
      select(i, false);
      copySelected();
    });
    $list.appendChild(row);
  });
}

function emptyState(query) {
  const box = document.createElement("div");
  box.className = "empty";
  if (!entries.length) {
    box.innerHTML = `${svgIcon("lock")}<div class="big">还没有条目</div><div>先在管理窗口里添加几个</div>`;
    const btn = document.createElement("button");
    btn.textContent = "打开管理窗口";
    btn.onclick = () => invoke("open_manager");
    box.appendChild(btn);
  } else {
    box.innerHTML = `${svgIcon("search")}<div class="big">没有匹配的条目</div>`;
    const sub = document.createElement("div");
    sub.textContent = `找不到与「${query}」相关的结果`;
    box.appendChild(sub);
  }
  return box;
}

function select(i, scroll = true) {
  if (!matches.length) return;
  sel = Math.max(0, Math.min(i, matches.length - 1));
  const rows = $list.querySelectorAll(".row");
  rows.forEach((r, idx) => r.classList.toggle("sel", idx === sel));
  if (scroll) rows[sel]?.scrollIntoView({ block: "nearest" });
}

async function copySelected() {
  if (busy) return;
  const cmd = $q.value.trim();
  if (cmd === ":m") return invoke("open_manager");
  if (cmd === ":q") return invoke("quit_app");
  const m = matches[sel];
  if (!m) return;
  busy = true;
  try {
    const secs = await invoke("copy_entry", { title: m.t });
    showDone("已复制", `${m.t} · ${secs} 秒后自动清空剪贴板`);
    // 隐藏后再清理，避免下次唤出时闪现上一次的确认层
    setTimeout(async () => {
      await invoke("hide_launcher");
      $done.classList.remove("show");
      $q.value = "";
      render();
    }, 550);
  } catch (e) {
    setStatus(String(e), true);
  } finally {
    setTimeout(() => (busy = false), 600);
  }
}

function showDone(title, sub) {
  document.getElementById("done-title").textContent = title;
  document.getElementById("done-sub").textContent = sub;
  $done.classList.add("show");
}

function reset() {
  $done.classList.remove("show");
  setStatus("QuickPwd");
  $q.value = "";
  sel = 0;
  $list.scrollTop = 0;
  reload();
  $q.focus();
}

$q.addEventListener("input", () => {
  sel = 0;
  render();
  $list.scrollTop = 0;
});

document.addEventListener("keydown", (e) => {
  switch (e.key) {
    case "ArrowDown":
      e.preventDefault();
      select(sel + 1);
      break;
    case "ArrowUp":
      e.preventDefault();
      select(sel - 1);
      break;
    case "PageDown":
      e.preventDefault();
      select(sel + 6);
      break;
    case "PageUp":
      e.preventDefault();
      select(sel - 6);
      break;
    case "Enter":
      e.preventDefault();
      copySelected();
      break;
    case "Escape":
      e.preventDefault();
      if ($q.value) {
        $q.value = "";
        render();
      } else {
        invoke("hide_launcher");
      }
      break;
    default:
      if (document.activeElement !== $q && e.key.length === 1 && !e.ctrlKey && !e.altKey) $q.focus();
  }
});

invoke("app_info").then((info) => document.body.className = info.appearance).catch(() => {});
listen("launcher-show", reset);
listen("store-changed", reload);
window.addEventListener("focus", () => $q.focus());
reset();
