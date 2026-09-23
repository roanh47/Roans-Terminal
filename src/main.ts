import { invoke } from "@tauri-apps/api/core";
import { Session } from "./terminal";
import type { Host, HostInput } from "./types";

interface Tab {
  host: Host;
  session: Session;
  el: HTMLElement;
}

const $ = <T extends HTMLElement>(s: string) => document.querySelector(s) as T;

let hosts: Host[] = [];
const tabs: Tab[] = [];
let active = -1;

function termColors() {
  const cs = getComputedStyle(document.documentElement);
  return { bg: cs.getPropertyValue("--term-bg").trim(), fg: cs.getPropertyValue("--term-fg").trim() };
}

/* ---------- Theme ---------- */
function applyTheme(t: "dark" | "light") {
  document.documentElement.dataset.theme = t;
  localStorage.setItem("theme", t);
  const { bg, fg } = termColors();
  tabs.forEach((t) => t.session.setTheme(bg, fg));
}
function toggleTheme() {
  const cur = document.documentElement.dataset.theme === "light" ? "dark" : "light";
  applyTheme(cur);
}

/* ---------- Hosts ---------- */
async function loadHosts() {
  hosts = await invoke<Host[]>("list_hosts");
  renderHosts();
}
function renderHosts() {
  const ul = $("#host-list");
  ul.innerHTML = "";
  hosts.forEach((h) => {
    const li = document.createElement("li");
    const dot = document.createElement("span");
    dot.className = "dot";
    dot.style.background = h.proto === "serial" ? "var(--text-muted)" : "var(--accent)";
    const name = document.createElement("span");
    name.className = "hname";
    name.textContent = h.name;
    const type = document.createElement("span");
    type.className = "htype";
    type.textContent = h.proto.toUpperCase();
    const actions = document.createElement("span");
    actions.className = "actions";
    const edit = document.createElement("button");
    edit.className = "icon-btn";
    edit.textContent = "✎";
    edit.title = "Edit";
    edit.onclick = (e) => { e.stopPropagation(); openHostDialog(h); };
    const del = document.createElement("button");
    del.className = "icon-btn";
    del.textContent = "✕";
    del.title = "Delete";
    del.onclick = (e) => { e.stopPropagation(); void deleteHost(h); };
    actions.append(edit, del);
    li.append(dot, name, type, actions);
    li.onclick = () => openTab(h);
    ul.appendChild(li);
  });
}
async function deleteHost(h: Host) {
  if (!confirm(`Delete host "${h.name}"?`)) return;
  await invoke("delete_host", { id: h.id });
  await loadHosts();
}

/* ---------- Tabs ---------- */
function openTab(host: Host) {
  const existing = tabs.findIndex((t) => t.host.id === host.id);
  if (existing >= 0) { setActive(existing); return; }

  const el = document.createElement("div");
  el.className = "terminal";
  $("#term-host").appendChild(el);
  $("#empty").style.display = "none";

  const sessionId = crypto.randomUUID();
  const { bg, fg } = termColors();
  const session = new Session(sessionId, el, bg, fg);
  const tab: Tab = { host, session, el };
  tabs.push(tab);

  session.onExit = (s, msg) => {
    const i = tabs.findIndex((t) => t.session === s);
    if (i >= 0) {
      s.term.write(`\r\n\x1b[31m[disconnected${msg ? ": " + msg : ""}]\x1b[0m\r\n`);
    }
  };

  setActive(tabs.length - 1);
  invoke("connect", { sessionId, hostId: host.id })
    .then(() => session.syncResize())
    .catch((e) => {
      session.term.write(`\r\n\x1b[31m[connect failed: ${e}]\x1b[0m\r\n`);
    });
}

function setActive(i: number) {
  active = i;
  tabs.forEach((t, j) => {
    t.el.classList.toggle("active", j === i);
    const tabEl = document.querySelector(`[data-tab="${j}"]`);
    if (tabEl) tabEl.classList.toggle("active", j === i);
  });
  if (i >= 0) setTimeout(() => tabs[i].session.refit(), 10);
  renderTabbar();
}

function renderTabbar() {
  const bar = $("#tabbar");
  bar.innerHTML = "";
  tabs.forEach((t, i) => {
    const tab = document.createElement("div");
    tab.className = "tab" + (i === active ? " active" : "");
    tab.dataset.tab = String(i);
    const name = document.createElement("span");
    name.textContent = t.host.name;
    const close = document.createElement("span");
    close.className = "close";
    close.textContent = "✕";
    close.onclick = (e) => { e.stopPropagation(); closeTab(i); };
    tab.append(name, close);
    tab.onclick = () => setActive(i);
    bar.appendChild(tab);
  });
}

function closeTab(i: number) {
  const tab = tabs[i];
  if (!tab) return;
  void invoke("disconnect", { sessionId: tab.session.id });
  tab.session.dispose();
  tab.el.remove();
  tabs.splice(i, 1);
  if (tabs.length === 0) {
    active = -1;
    $("#empty").style.display = "flex";
  } else {
    setActive(Math.max(0, Math.min(i, tabs.length - 1)));
  }
  renderTabbar();
}

/* ---------- Host dialog ---------- */
function openHostDialog(h?: Host) {
  const dlg = $("#host-dialog") as HTMLDialogElement;
  $("#host-dialog-title").textContent = h ? "Edit host" : "New host";
  ($("#f-id") as HTMLInputElement).value = h?.id ?? "";
  ($("#f-name") as HTMLInputElement).value = h?.name ?? "";
  ($("#f-proto") as HTMLSelectElement).value = h?.proto ?? "ssh";
  ($("#f-host") as HTMLInputElement).value = h?.host ?? "";
  ($("#f-port") as HTMLInputElement).value = String(h?.port ?? 22);
  ($("#f-user") as HTMLInputElement).value = h?.username ?? "";
  ($("#f-auth") as HTMLSelectElement).value = h?.auth ?? "password";
  ($("#f-password") as HTMLInputElement).value = "";
  ($("#f-key") as HTMLInputElement).value = h?.keyPath ?? "";
  ($("#f-passphrase") as HTMLInputElement).value = "";
  ($("#f-serial-port") as HTMLInputElement).value = h?.serialPort ?? "";
  ($("#f-baud") as HTMLInputElement).value = String(h?.baud ?? 115200);
  syncFields();
  dlg.showModal();
}
function syncFields() {
  const proto = ($("#f-proto") as HTMLSelectElement).value;
  const auth = ($("#f-auth") as HTMLSelectElement).value;
  $("#ssh-fields").hidden = proto !== "ssh";
  $("#serial-fields").hidden = proto !== "serial";
  $("#f-password-wrap").hidden = auth !== "password";
  $("#f-key-wrap").hidden = auth !== "key";
  $("#f-passphrase-wrap").hidden = auth !== "key";
}

async function saveHost(e: Event) {
  e.preventDefault();
  const proto = ($("#f-proto") as HTMLSelectElement).value as HostInput["proto"];
  const input: HostInput = {
    id: ($("#f-id") as HTMLInputElement).value || undefined,
    name: ($("#f-name") as HTMLInputElement).value,
    proto,
  };
  if (proto === "ssh") {
    input.host = ($("#f-host") as HTMLInputElement).value;
    input.port = parseInt(($("#f-port") as HTMLInputElement).value) || 22;
    input.username = ($("#f-user") as HTMLInputElement).value;
    input.auth = ($("#f-auth") as HTMLSelectElement).value as HostInput["auth"];
    if (input.auth === "password") input.password = ($("#f-password") as HTMLInputElement).value;
    else {
      input.keyPath = ($("#f-key") as HTMLInputElement).value;
      input.passphrase = ($("#f-passphrase") as HTMLInputElement).value;
    }
  } else {
    input.serialPort = ($("#f-serial-port") as HTMLInputElement).value;
    input.baud = parseInt(($("#f-baud") as HTMLInputElement).value) || 115200;
  }
  await invoke("save_host", { host: input });
  ($("#host-dialog") as HTMLDialogElement).close();
  await loadHosts();
}

/* ---------- Wire up ---------- */
$("#btn-theme").onclick = toggleTheme;
$("#btn-new-host").onclick = () => openHostDialog();
$("#btn-new-host-2").onclick = () => openHostDialog();
$("#btn-cancel").onclick = () => ($("#host-dialog") as HTMLDialogElement).close();
$("#host-form").addEventListener("submit", (e) => void saveHost(e));
$("#f-proto").addEventListener("change", syncFields);
$("#f-auth").addEventListener("change", syncFields);

applyTheme((localStorage.getItem("theme") as "dark" | "light") || "dark");
void loadHosts();
