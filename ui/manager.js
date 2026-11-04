const win = window.__TAURI__.window.getCurrentWindow();
const $ = (id) => document.getElementById(id);

$("logo").innerHTML = svgIcon("lock");
$("btn-min").innerHTML = svgIcon("min");
$("btn-max").innerHTML = svgIcon("max");
$("btn-close").innerHTML = svgIcon("close");
$("btn-add").innerHTML = `${svgIcon("plus")}<span>新增</span>`;
$("filter-icon").innerHTML = svgIcon("search");
$("btn-gen").innerHTML = svgIcon("dice");

$("btn-min").onclick = () => win.minimize();
$("btn-max").onclick = () => win.toggleMaximize();
$("btn-close").onclick = () => {
  closeHotkeyDialog();
  win.hide();
};

let entries = [];
let selected = null;
let editing = null; // null = 新增；字符串 = 编辑中的原关键字
let deleting = null;

// ---------------- 列表 ----------------
async function reload() {
  try {
    entries = await invoke("list_titles");
  } catch (e) {
    toast(String(e), "err");
  }
  render();
}

function render() {
  const q = $("filter").value.trim().toLowerCase();
  const shown = entries.filter((t) => !q || t.toLowerCase().includes(q));
  $("subtitle").textContent = q
    ? `共 ${entries.length} 个条目 · 匹配 ${shown.length} 个`
    : `共 ${entries.length} 个条目`;
  $("count").textContent = "双击复制 · 关闭窗口后在托盘继续运行";

  const tbody = $("tbody");
  tbody.replaceChildren();
  if (!shown.length) {
    const box = document.createElement("div");
    box.className = "table-empty";
    box.innerHTML = entries.length
      ? `${svgIcon("search")}<div class="big">没有匹配的条目</div><div>换个关键字试试</div>`
      : `${svgIcon("lock")}<div class="big">还没有密码条目</div><div>点击右上角「新增」添加第一个</div>`;
    tbody.appendChild(box);
    return;
  }

  for (const t of shown) {
    const tr = document.createElement("div");
    tr.className = "tr" + (t === selected ? " sel" : "");
    tr.dataset.kw = t;
    tr.appendChild(avatarEl(t));
    tr.appendChild(cell(t, "title"));
    tr.appendChild(cell("••••••••", "secret"));

    const actions = document.createElement("div");
    actions.className = "actions";
    actions.append(
      iconButton("copy", "复制", () => copy(t)),
      iconButton("edit", "编辑", () => openEditor(t)),
      iconButton("trash", "删除", () => askDelete(t), "del"),
    );
    tr.appendChild(actions);

    tr.addEventListener("click", () => setSelected(t));
    tr.addEventListener("dblclick", (ev) => {
      if (!ev.target.closest(".actions")) copy(t);
    });
    tbody.appendChild(tr);
  }
}

function cell(text, cls) {
  const el = document.createElement("div");
  el.className = `cell ${cls}`;
  el.textContent = text;
  if (cls === "title") el.title = text;
  return el;
}

function iconButton(icon, title, onClick, extra = "") {
  const b = document.createElement("button");
  b.className = `icon-btn ${extra}`;
  b.title = title;
  b.innerHTML = svgIcon(icon);
  b.addEventListener("click", (ev) => {
    ev.stopPropagation();
    onClick();
  });
  return b;
}

function setSelected(kw) {
  selected = kw;
  document.querySelectorAll(".tr").forEach((tr) => tr.classList.toggle("sel", tr.dataset.kw === kw));
}

function visibleKeywords() {
  return [...document.querySelectorAll(".tr")].map((tr) => tr.dataset.kw);
}

async function copy(kw) {
  try {
    const secs = await invoke("copy_entry", { title: kw });
    toast(`已复制「${kw}」，${secs} 秒后自动清空剪贴板`, "ok");
  } catch (e) {
    toast(String(e), "err");
  }
}

// ---------------- 新增 / 编辑 ----------------
async function openEditor(kw = null) {
  editing = kw;
  let content = "";
  if (kw) {
    try {
      content = await invoke("get_content", { title: kw });
    } catch (e) {
      return toast(String(e), "err");
    }
  }
  $("edit-title").textContent = kw ? "编辑条目" : "新增条目";
  $("f-title").value = kw || "";
  $("f-content").value = content;
  $("f-error").textContent = "";
  setContentVisible(!kw);
  $("edit-overlay").classList.add("show");
  setTimeout(() => (kw ? $("f-content") : $("f-title")).focus(), 30);
}

function closeEditor() {
  $("edit-overlay").classList.remove("show");
  $("f-content").value = "";
}

function setContentVisible(v) {
  $("f-content").classList.toggle("masked", !v);
  $("btn-eye").innerHTML = svgIcon(v ? "eyeOff" : "eye");
}

function generatePassword(len = 20) {
  const sets = ["ABCDEFGHJKLMNPQRSTUVWXYZ", "abcdefghijkmnopqrstuvwxyz", "23456789", "!@#$%^&*-_=+?"];
  const all = sets.join("");
  const rand = (n) => {
    const buf = new Uint32Array(1);
    crypto.getRandomValues(buf);
    return buf[0] % n;
  };
  const chars = sets.map((s) => s[rand(s.length)]);
  while (chars.length < len) chars.push(all[rand(all.length)]);
  for (let i = chars.length - 1; i > 0; i--) {
    const j = rand(i + 1);
    [chars[i], chars[j]] = [chars[j], chars[i]];
  }
  return chars.join("");
}

$("btn-eye").onclick = () => setContentVisible($("f-content").classList.contains("masked"));
$("btn-gen").onclick = () => {
  $("f-content").value = generatePassword();
  setContentVisible(true);
};
$("btn-cancel").onclick = closeEditor;
$("btn-add").onclick = () => openEditor();

$("f-title").addEventListener("keydown", (e) => {
  if (e.key === "Enter") { e.preventDefault(); $("f-content").focus(); }
});
$("f-content").addEventListener("keydown", (e) => {
  if (e.key === "Enter" && e.ctrlKey) { e.preventDefault(); $("edit-form").requestSubmit(); }
});

$("edit-form").addEventListener("submit", async (ev) => {
  ev.preventDefault();
  const entry = {
    title: $("f-title").value.trim(),
    content: $("f-content").value,
  };
  try {
    await invoke("save_entry", { original: editing, entry });
    selected = entry.title;
    closeEditor();
    toast(`已保存「${entry.title}」`, "ok");
  } catch (e) {
    $("f-error").textContent = String(e);
  }
});

// ---------------- 删除 ----------------
function askDelete(kw) {
  deleting = kw;
  $("del-text").textContent = `确定删除「${kw}」吗？此操作无法撤销。`;
  $("del-overlay").classList.add("show");
  setTimeout(() => $("del-ok").focus(), 30);
}

function closeDelete() {
  $("del-overlay").classList.remove("show");
  deleting = null;
}

$("del-cancel").onclick = closeDelete;
$("del-ok").onclick = async () => {
  const kw = deleting;
  closeDelete();
  try {
    await invoke("delete_entry", { title: kw });
    if (selected === kw) selected = null;
    toast(`已删除「${kw}」`, "ok");
  } catch (e) {
    toast(String(e), "err");
  }
};

// ---------------- 设置与状态 ----------------
async function loadInfo() {
  const info = await invoke("app_info");
  const chip = $("hotkey-chip");
  chip.classList.toggle("bad", !info.hotkey);
  chip.innerHTML = "";
  chip.append(info.hotkey_label ? `${info.hotkey_label} 唤出搜索` : "全局热键未注册，点击设置");
  const hint = document.createElement("span");
  hint.className = "edit-hint";
  hint.textContent = " · 修改";
  if (info.hotkey_label) chip.appendChild(hint);
  $("autostart").checked = info.autostart;

  const banners = $("banners");
  banners.replaceChildren();
  if (info.load_error) {
    banners.appendChild(banner("err", `${info.load_error}（数据目录：${info.data_dir}）。为保护原文件，当前已禁止修改。`));
  }
  if (info.hotkey_notice) {
    banners.appendChild(banner("warn", `${info.hotkey_notice}。可点击左下角重新设置。`));
  }
  if (!info.hotkey) {
    banners.appendChild(banner("warn", "常用组合键都被其他程序占用，无法唤出搜索框。请点击左下角设置一个未被占用的快捷键。"));
  }
}

function banner(kind, text) {
  const el = document.createElement("div");
  el.className = `banner ${kind}`;
  el.innerHTML = svgIcon("alert");
  const span = document.createElement("span");
  span.textContent = text;
  el.appendChild(span);
  return el;
}

$("autostart").addEventListener("change", async (ev) => {
  const want = ev.target.checked;
  try {
    ev.target.checked = await invoke("set_autostart", { enabled: want });
    toast(want ? "已开启开机自启" : "已关闭开机自启", "ok");
  } catch (e) {
    ev.target.checked = !want;
    toast(String(e), "err");
  }
});

let toastTimer;
function toast(text, kind = "") {
  const t = $("toast");
  t.textContent = text;
  t.className = `toast show ${kind}`;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => t.classList.remove("show"), 2600);
}

// ---------------- 快捷键设置 ----------------
// 录制由后端键盘钩子完成（Win+Space、Alt+Space 等系统组合不会先被系统吃掉），
// 通过 hotkey-capture 事件推送按键
let hkAccel = null;
let hkOpen = false;

function keyLabel(token) {
  if (token === "Control") return "Ctrl";
  if (token === "Super") return "Win";
  if (/^Key[A-Z]$/.test(token)) return token.slice(3);
  if (/^Digit\d$/.test(token)) return token.slice(5);
  const names = {
    Backquote: "`", Minus: "-", Equal: "=", BracketLeft: "[", BracketRight: "]", Backslash: "\\",
    Semicolon: ";", Quote: "'", Comma: ",", Period: ".", Slash: "/",
    ArrowUp: "↑", ArrowDown: "↓", ArrowLeft: "←", ArrowRight: "→",
  };
  return names[token] || token;
}

const accelLabels = (acc) => (acc ? acc.split("+").map(keyLabel) : []);

function showCapture(parts, ok) {
  const box = $("hk-capture");
  box.replaceChildren();
  box.classList.toggle("ok", ok);
  if (!parts.length) {
    box.textContent = "请按下组合键…";
    return;
  }
  parts.forEach((p, i) => {
    if (i) {
      const plus = document.createElement("span");
      plus.className = "plus";
      plus.textContent = "+";
      box.appendChild(plus);
    }
    const k = document.createElement("kbd");
    k.textContent = p;
    box.appendChild(k);
  });
}

function setCandidate(acc) {
  hkAccel = acc;
  $("hk-save").disabled = !acc;
  $("hk-error").textContent = "";
  $("hk-note").textContent = acc && acc.split("+").includes("Super") && acc.endsWith("Space")
    ? "Win + Space 原本是系统切换输入法的快捷键，设置后将由 QuickPwd 接管（仍可用 Ctrl + Shift 或点击任务栏切换输入法）"
    : "";
  showCapture(accelLabels(acc), true);
}

function clearCandidate(parts, error = "") {
  hkAccel = null;
  $("hk-save").disabled = true;
  $("hk-error").textContent = error;
  $("hk-note").textContent = "";
  showCapture(parts, false);
}

async function openHotkeyDialog() {
  clearCandidate([]);
  try {
    await invoke("start_hotkey_recording");
  } catch (e) {
    return toast(String(e), "err");
  }
  hkOpen = true;
  $("hk-overlay").classList.add("show");
}

function closeHotkeyDialog(saved = false) {
  if (!hkOpen) return;
  hkOpen = false;
  $("hk-overlay").classList.remove("show");
  if (!saved) invoke("stop_hotkey_recording");
}

listen("hotkey-capture", ({ payload }) => {
  if (!hkOpen) return;
  const { kind, accelerator } = payload;
  if (kind === "cancel") return closeHotkeyDialog();
  if (kind === "confirm") {
    if (hkAccel) saveHotkey();
    return;
  }
  if (kind === "mods") {
    // 按下修饰键：开始一个新组合
    clearCandidate(accelLabels(accelerator));
    return;
  }
  if (kind === "invalid") return clearCandidate(accelLabels(accelerator), "需要同时按住 Ctrl / Alt / Shift / Win 中的至少一个");
  if (kind === "reserved") return clearCandidate(accelLabels(accelerator), "这是系统窗口切换 / 关闭的组合，不能使用");
  setCandidate(accelerator);
});

async function saveHotkey() {
  if (!hkAccel) return;
  try {
    const res = await invoke("set_hotkey", { accelerator: hkAccel });
    closeHotkeyDialog(true);
    await loadInfo();
    toast(`快捷键已改为 ${res.label}`, "ok");
  } catch (err) {
    $("hk-error").textContent = String(err);
    $("hk-capture").classList.remove("ok");
  }
}

$("hotkey-chip").onclick = openHotkeyDialog;
// 录制中切走窗口时恢复原快捷键
window.addEventListener("blur", () => closeHotkeyDialog());
$("hk-cancel").onclick = () => closeHotkeyDialog();
document.querySelectorAll(".hk-presets button").forEach((b) => {
  b.onclick = () => setCandidate(b.dataset.acc);
});
$("hk-save").onclick = saveHotkey;

// ---------------- 键盘 ----------------
document.addEventListener("keydown", (e) => {
  if (hkOpen) {
    e.preventDefault();
    return;
  }
  const editOpen = $("edit-overlay").classList.contains("show");
  const delOpen = $("del-overlay").classList.contains("show");
  if (e.key === "Escape") {
    e.preventDefault();
    if (editOpen) closeEditor();
    else if (delOpen) closeDelete();
    else if ($("filter").value) { $("filter").value = ""; render(); }
    else win.hide();
    return;
  }
  if (editOpen || delOpen) return;
  if (e.ctrlKey && e.key.toLowerCase() === "f") {
    e.preventDefault();
    $("filter").focus();
    return;
  }
  if (e.ctrlKey && e.key.toLowerCase() === "n") {
    e.preventDefault();
    openEditor();
    return;
  }
  const inFilter = document.activeElement === $("filter");
  const kws = visibleKeywords();
  if (e.key === "ArrowDown" || e.key === "ArrowUp") {
    e.preventDefault();
    if (!kws.length) return;
    const i = kws.indexOf(selected);
    const next = e.key === "ArrowDown" ? Math.min(kws.length - 1, i + 1) : Math.max(0, i - 1);
    setSelected(kws[next]);
    document.querySelector(".tr.sel")?.scrollIntoView({ block: "nearest" });
  } else if (e.key === "Enter" && selected && kws.includes(selected)) {
    e.preventDefault();
    copy(selected);
  } else if (e.key === "Delete" && selected && !inFilter) {
    askDelete(selected);
  } else if (e.key === "F2" && selected) {
    openEditor(selected);
  }
});

$("filter").addEventListener("input", render);
listen("store-changed", reload);
window.addEventListener("focus", () => loadInfo().catch(() => {}));
reload();
loadInfo().catch((e) => toast(String(e), "err"));
