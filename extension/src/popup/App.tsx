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

export default function App() {
  const [status,   setStatus]   = useState<VaultStatus | null>(null);
  const [items,    setItems]    = useState<ItemSummary[]>([]);
  const [query,    setQuery]    = useState("");
  const [pageUrl,  setPageUrl]  = useState("");
  const [loading,  setLoading]  = useState(true);
  const [error,    setError]    = useState("");
  const [filling,  setFilling]  = useState<string | null>(null);
  const [profileLabel, setProfileLabel] = useState<string | null>(null);

  useEffect(() => {
    init();
  }, []);

  async function init() {
    try {
      // Get signed-in Google account email for this Chrome profile (empty = local profile)
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
      // ignore search errors silently
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

  // Client-side domain filter — applied on top of server-side text search
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

      <footer>{status.itemCount} items total</footer>
    </div>
  );
}
