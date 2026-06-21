import { useState, useEffect } from "react";
import { domainsMatch } from "../shared/domain";
import type {
  ItemSummary,
  VaultStatus,
  ExtensionMessage,
  ExtensionResponse,
  Credentials,
} from "../shared/types";

function sendMsg<T>(msg: ExtensionMessage): Promise<T> {
  return chrome.runtime.sendMessage(msg).then((r: ExtensionResponse) => {
    if (!r.ok) throw new Error(r.error);
    return r.data as T;
  });
}

const ICONS: Record<string, string> = {
  login: "🔑",
  card: "💳",
  note: "📄",
  identity: "👤",
  ssh_key: "🖥",
};

// Secure password generator — uses crypto.getRandomValues(), never Math.random()
function generateSecurePassword(length = 20): string {
  const upper  = "ABCDEFGHJKLMNPQRSTUVWXYZ";   // no I, O
  const lower  = "abcdefghjkmnpqrstuvwxyz";     // no i, l, o
  const digits = "23456789";                    // no 0, 1
  const syms   = "!@#$%^&*-_+=?";
  const all    = upper + lower + digits + syms;

  const buf = new Uint32Array(length + 8);
  crypto.getRandomValues(buf);

  // Guarantee at least one char from each category
  const required = [
    upper[buf[0]  % upper.length],
    lower[buf[1]  % lower.length],
    digits[buf[2] % digits.length],
    syms[buf[3]   % syms.length],
  ];
  const rest = Array.from({ length: length - 4 }, (_, i) => all[buf[i + 4] % all.length]);
  const raw  = [...required, ...rest];

  // Shuffle with Fisher-Yates using crypto random
  const shuffle = new Uint32Array(raw.length);
  crypto.getRandomValues(shuffle);
  for (let i = raw.length - 1; i > 0; i--) {
    const j = shuffle[i] % (i + 1);
    [raw[i], raw[j]] = [raw[j], raw[i]];
  }
  return raw.join("");
}

export default function App() {
  const [status,   setStatus]   = useState<VaultStatus | null>(null);
  const [items,    setItems]    = useState<ItemSummary[]>([]);
  const [query,    setQuery]    = useState("");
  const [pageUrl,  setPageUrl]  = useState("");
  const [loading,  setLoading]  = useState(true);
  const [error,    setError]    = useState("");
  const [filling,  setFilling]  = useState<string | null>(null);
  const [profileLabel, setProfileLabel] = useState<string | null>(null);
  const [genPw,    setGenPw]    = useState("");
  const [genCopied, setGenCopied] = useState(false);

  useEffect(() => {
    init();
  }, []);

  async function init() {
    try {
      chrome.identity.getProfileUserInfo({ accountStatus: "ANY" }, (info) => {
        setProfileLabel(info.email || null);
      });

      const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
      const url = tab?.url ?? "";
      setPageUrl(url);

      const s = await sendMsg<VaultStatus>({ type: "GET_STATUS" });
      setStatus(s);

      if (s && !s.isLocked) {
        const results = await sendMsg<ItemSummary[]>({
          type: "SEARCH",
          query: "",
          pageUrl: url,
        });
        setItems(results ?? []);
      }
    } catch (e) {
      setError(
        e instanceof Error && e.message.includes("Native host")
          ? "LSPV desktop is not running."
          : String(e)
      );
    } finally {
      setLoading(false);
    }
  }

  async function search(q: string) {
    setQuery(q);
    try {
      const results = await sendMsg<ItemSummary[]>({
        type: "SEARCH",
        query: q,
        pageUrl,
      });
      setItems(results ?? []);
    } catch {
      // ignore silently
    }
  }

  async function fill(item: ItemSummary) {
    setFilling(item.id);
    try {
      const creds = await sendMsg<Credentials>({
        type: "GET_CREDENTIALS",
        itemId: item.id,
      });
      const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
      if (tab?.id) {
        await chrome.tabs.sendMessage(tab.id, {
          type: "FILL",
          username: creds.username,
          password: creds.password,
        });
      }
      window.close();
    } catch (e) {
      setError(String(e));
    } finally {
      setFilling(null);
    }
  }

  async function lock() {
    await sendMsg({ type: "LOCK" });
    setStatus((s) => s && { ...s, isLocked: true });
    setItems([]);
  }

  function generateAndShow() {
    const pw = generateSecurePassword(20);
    setGenPw(pw);
    setGenCopied(false);
  }

  async function fillGenerated() {
    if (!genPw) return;
    try {
      const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
      if (tab?.id) {
        await chrome.tabs.sendMessage(tab.id, {
          type: "FILL_PASSWORD_ONLY",
          password: genPw,
        });
      }
      window.close();
    } catch (e) {
      setError(String(e));
    }
  }

  async function copyGenerated() {
    if (!genPw) return;
    await navigator.clipboard.writeText(genPw);
    setGenCopied(true);
    setTimeout(() => setGenCopied(false), 2000);
  }

  // Client-side domain filter
  const visible = pageUrl
    ? items.filter(
        (it) =>
          it.itemType !== "login" ||
          !it.url ||
          query.length > 0 ||
          domainsMatch(it.url, pageUrl)
      )
    : items;

  if (loading) {
    return (
      <div className="center">
        <div className="spinner" />
        <p>Connecting…</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="center error">
        <div className="icon">⚠️</div>
        <p>{error}</p>
        <button onClick={init}>Retry</button>
      </div>
    );
  }

  if (!status || status.isLocked) {
    return (
      <div className="center locked">
        <div className="icon">🔒</div>
        <p className="locked-title">Vault is locked</p>
        <p className="hint">Open LSPV desktop to unlock.</p>
      </div>
    );
  }

  return (
    <div className="app">
      <header>
        <span className="logo">🔐 LSPV</span>
        {profileLabel && (
          <span className="profile-label" title={profileLabel}>
            {profileLabel}
          </span>
        )}
        <button className="lock-btn" onClick={lock} title="Lock vault">
          🔒
        </button>
      </header>

      <div className="search-wrap">
        <input
          type="search"
          className="search"
          placeholder="Search items…"
          value={query}
          onChange={(e) => search(e.target.value)}
          autoFocus
        />
      </div>

      <ul className="items">
        {visible.length === 0 && (
          <li className="empty">
            {query ? "No results." : "No items for this page."}
          </li>
        )}
        {visible.map((item) => (
          <li key={item.id} className="item">
            <span className="item-icon">{ICONS[item.itemType] ?? "📁"}</span>
            <div className="item-info">
              <span className="item-title">{item.title}</span>
              {item.username && (
                <span className="item-sub">{item.username}</span>
              )}
            </div>
            {item.itemType === "login" && (
              <button
                className="fill-btn"
                onClick={() => fill(item)}
                disabled={filling === item.id}
              >
                {filling === item.id ? "…" : "Fill"}
              </button>
            )}
          </li>
        ))}
      </ul>

      {/* ── Password generator ── */}
      <div className="gen-section">
        <button className="gen-trigger" onClick={generateAndShow}>
          ⚡ Generate password
        </button>
        {genPw && (
          <div className="gen-result">
            <span className="gen-pw" title={genPw}>{genPw}</span>
            <div className="gen-actions">
              <button className="gen-btn" onClick={fillGenerated} title="Fill into password field">
                Fill
              </button>
              <button className="gen-btn gen-copy" onClick={copyGenerated}>
                {genCopied ? "✓" : "Copy"}
              </button>
              <button className="gen-btn gen-regen" onClick={generateAndShow} title="Regenerate">
                ↺
              </button>
            </div>
          </div>
        )}
      </div>

      <footer>{status.itemCount} items total</footer>
    </div>
  );
}
