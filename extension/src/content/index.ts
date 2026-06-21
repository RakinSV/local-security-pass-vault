// Content script — isolated world, IIFE, no ES module runtime.
// Two responsibilities:
//   1. Auto-prompt: detect login forms and show a floating "Fill?" card.
//   2. Fill: receive FILL messages and autofill username/password fields.

// ── Shared helpers (bundled by Vite into this IIFE) ──────────────────────────

const nativeInputValueSetter = Object.getOwnPropertyDescriptor(
  HTMLInputElement.prototype,
  "value"
)?.set;

function isVisible(el: HTMLElement): boolean {
  return (
    el.offsetParent !== null &&
    getComputedStyle(el).visibility !== "hidden" &&
    getComputedStyle(el).opacity !== "0" &&
    el.getBoundingClientRect().width > 0 &&
    el.getBoundingClientRect().height > 0
  );
}

function fillInput(input: HTMLInputElement, value: string): void {
  nativeInputValueSetter?.call(input, value);
  input.setAttribute("autocomplete", "new-password");
  input.dispatchEvent(new Event("input",  { bubbles: true }));
  input.dispatchEvent(new Event("change", { bubbles: true }));
}

function fillPage(username: string, password: string): void {
  const inputs = Array.from(document.querySelectorAll<HTMLInputElement>("input"));
  let filledUser = false;
  let filledPass = false;

  for (const input of inputs) {
    if (!isVisible(input)) continue;
    if (!filledUser && (input.type === "text" || input.type === "email")) {
      fillInput(input, username);
      filledUser = true;
    } else if (!filledPass && input.type === "password") {
      fillInput(input, password);
      filledPass = true;
    }
    if (filledUser && filledPass) break;
  }
}

// ── Auto-prompt ───────────────────────────────────────────────────────────────

const HOST_ID = "__vaultpass_prompt__";
let promptDismissed = false;

interface PromptItem {
  id: string;
  title: string;
  username?: string;
}

function showPrompt(items: PromptItem[], profileLabel: string | null): void {
  if (document.getElementById(HOST_ID)) return;

  const host = document.createElement("div");
  host.id = HOST_ID;
  // Zero-size host; the actual card is position:fixed inside the shadow DOM
  Object.assign(host.style, {
    all: "initial",
    position: "fixed",
    top: "0",
    left: "0",
    width: "0",
    height: "0",
    zIndex: "2147483647",
    pointerEvents: "none",
  } as Partial<CSSStyleDeclaration>);

  const shadow = host.attachShadow({ mode: "open" });

  const style = document.createElement("style");
  style.textContent = `
    *{box-sizing:border-box;margin:0;padding:0}
    .wrap{
      position:fixed;top:12px;right:12px;width:272px;
      background:#0f1117;border:1px solid #2d2e45;border-radius:14px;
      padding:11px 12px;font:13px/1.45 -apple-system,system-ui,sans-serif;
      color:#c9d1d9;box-shadow:0 8px 36px rgba(0,0,0,.75);
      pointer-events:all;z-index:2147483647;
    }
    .head{display:flex;align-items:center;gap:6px;margin-bottom:9px}
    .logo{font-size:11px;font-weight:700;color:#6366f1;flex:1;letter-spacing:.03em;text-transform:uppercase}
    .profile{font-size:11px;color:#6b7280;max-width:120px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
    .close{background:none;border:none;cursor:pointer;color:#6b7280;font-size:20px;line-height:1;padding:0 1px;flex-shrink:0}
    .close:hover{color:#ef4444}
    .item{
      display:flex;align-items:center;gap:8px;
      padding:7px 9px;border-radius:9px;
      background:#1a1b26;border:1px solid transparent;
      cursor:pointer;width:100%;text-align:left;
      margin-top:5px;
    }
    .item:hover{border-color:#6366f1;background:#1e1f2e}
    .info{flex:1;min-width:0}
    .name{font-size:12px;font-weight:500;color:#e6edf3;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
    .user{font-size:11px;color:#6b7280;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
    .fill{font-size:11px;font-weight:600;color:#6366f1;background:none;border:none;cursor:pointer;
          padding:3px 8px;border-radius:5px;flex-shrink:0}
    .fill:hover{background:#6366f1;color:#fff}
  `;

  const wrap = document.createElement("div");
  wrap.className = "wrap";

  // Header
  const head = document.createElement("div");
  head.className = "head";

  const logo = document.createElement("span");
  logo.className = "logo";
  logo.textContent = "🔐 VaultPass";
  head.appendChild(logo);

  if (profileLabel) {
    const prof = document.createElement("span");
    prof.className = "profile";
    prof.title = profileLabel;
    prof.textContent = profileLabel;
    head.appendChild(prof);
  }

  const closeBtn = document.createElement("button");
  closeBtn.className = "close";
  closeBtn.textContent = "×";
  closeBtn.onclick = () => { promptDismissed = true; host.remove(); };
  head.appendChild(closeBtn);

  wrap.appendChild(head);

  // Item buttons
  for (const item of items.slice(0, 5)) {
    const row = document.createElement("div");
    row.className = "item";

    const info = document.createElement("div");
    info.className = "info";

    const name = document.createElement("div");
    name.className = "name";
    name.textContent = item.title;
    info.appendChild(name);

    if (item.username) {
      const user = document.createElement("div");
      user.className = "user";
      user.textContent = item.username;
      info.appendChild(user);
    }

    row.appendChild(info);

    const fillBtn = document.createElement("button");
    fillBtn.className = "fill";
    fillBtn.textContent = "Fill";
    fillBtn.onclick = (e) => {
      e.stopPropagation();
      chrome.runtime.sendMessage({ type: "FILL_FROM_PROMPT", itemId: item.id });
      host.remove();
    };
    row.appendChild(fillBtn);
    row.onclick = () => fillBtn.click();

    wrap.appendChild(row);
  }

  shadow.appendChild(style);
  shadow.appendChild(wrap);
  document.documentElement.appendChild(host);

  // Auto-dismiss after 20 s
  setTimeout(() => { if (!promptDismissed) host.remove(); }, 20_000);
}

function detectAndPrompt(): void {
  if (promptDismissed) return;
  if (document.getElementById(HOST_ID)) return;

  // Only trigger when at least one visible password field is present
  const pwInputs = Array.from(document.querySelectorAll<HTMLInputElement>("input[type=password]"));
  if (!pwInputs.some(isVisible)) return;

  chrome.runtime.sendMessage(
    { type: "DETECT_FORM", url: window.location.href },
    (resp: { ok: boolean; data?: { items: PromptItem[]; profileLabel: string | null } }) => {
      if (chrome.runtime.lastError) return;
      if (!resp?.ok || !resp.data?.items.length) return;
      showPrompt(resp.data.items, resp.data.profileLabel ?? null);
    }
  );
}

// Run after DOM idle (manifest: run_at = document_idle)
detectAndPrompt();

// Re-check after a short delay for SPAs that insert forms asynchronously
setTimeout(detectAndPrompt, 1_500);

// ── Fill messages from popup and background ───────────────────────────────────

function fillPasswordOnly(password: string): void {
  const inputs = Array.from(document.querySelectorAll<HTMLInputElement>("input[type=password]"));
  for (const input of inputs) {
    if (!isVisible(input)) continue;
    fillInput(input, password);
    break;
  }
}

function showTotpHint(): void {
  const id = "__vaultpass_totp_hint__";
  if (document.getElementById(id)) return;
  const el = document.createElement("div");
  el.id = id;
  Object.assign(el.style, {
    position: "fixed",
    bottom: "20px",
    right: "12px",
    zIndex: "2147483647",
    background: "#1a1d27",
    border: "1px solid #6366f1",
    borderRadius: "10px",
    padding: "10px 14px",
    font: "13px system-ui, sans-serif",
    color: "#e2e8f0",
    boxShadow: "0 4px 24px rgba(0,0,0,.7)",
    maxWidth: "260px",
    lineHeight: "1.45",
  } as Partial<CSSStyleDeclaration>);
  el.textContent = "🔐 Vault 2FA required — open the extension popup to fill";
  document.documentElement.appendChild(el);
  setTimeout(() => el.remove(), 5_000);
}

chrome.runtime.onMessage.addListener(
  (msg: { type: string; username?: string; password?: string }, sender) => {
    // Reject messages from any source other than this extension.
    if (sender.id !== chrome.runtime.id) return;
    if (msg.type === "FILL" && msg.username !== undefined && msg.password !== undefined) {
      fillPage(msg.username, msg.password);
    } else if (msg.type === "FILL_PASSWORD_ONLY" && msg.password !== undefined) {
      fillPasswordOnly(msg.password);
    } else if (msg.type === "TOTP_HINT") {
      showTotpHint();
    }
  }
);
