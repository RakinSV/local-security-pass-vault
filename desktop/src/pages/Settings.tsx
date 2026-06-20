import { useState, useEffect, useRef } from "react";
import { PasswordField } from "../components/PasswordField";
import {
  changeMasterPassword,
  getBrowserIntegrations,
  saveBrowserIntegrations,
  getNativeHostPath,
  parseImportCsv,
  importItemsFromCsv,
  getProfiles,
  setProfileName,
  openGithub,
  getAutostart,
  setAutostart as setAutostartCmd,
  bulkRetagItems,
  getAutoLockSettings,
  setAutoLockSettings,
} from "../api/vault";
import type { BrowserConfig, ImportRow, ProfileInfo } from "../types/vault";

interface Props {
  onBack: () => void;
  onImported?: () => void;
}

type Tab = "general" | "security" | "browser" | "import" | "about";

export function Settings({ onBack, onImported }: Props) {
  const [tab, setTab] = useState<Tab>("general");

  const tabs: { id: Tab; label: string }[] = [
    { id: "general",  label: "General"  },
    { id: "security", label: "Security" },
    { id: "browser",  label: "Browser"  },
    { id: "import",   label: "Import"   },
    { id: "about",    label: "About"    },
  ];

  return (
    <div className="flex flex-col h-screen">
      <div className="flex items-center gap-3 px-4 py-3 border-b border-[var(--border)]">
        <button
          onClick={onBack}
          className="text-[var(--muted)] hover:text-[var(--text)] transition-colors text-sm"
        >
          ←
        </button>
        <div className="flex-1 font-medium">Settings</div>
      </div>

      {/* Tab bar */}
      <div className="flex border-b border-[var(--border)] px-4">
        {tabs.map(t => (
          <button
            key={t.id}
            onClick={() => setTab(t.id)}
            className={`px-5 py-2.5 text-sm font-medium border-b-2 transition-colors -mb-px ${
              tab === t.id
                ? "border-[var(--accent)] text-[var(--accent)]"
                : "border-transparent text-[var(--muted)] hover:text-[var(--text)]"
            }`}
          >
            {t.label}
          </button>
        ))}
      </div>

      <div className="flex-1 overflow-y-auto">
        {tab === "general"  && <GeneralTab />}
        {tab === "security" && <SecurityTab />}
        {tab === "browser"  && <BrowserTab />}
        {tab === "import"   && <ImportTab onImported={onImported} />}
        {tab === "about"    && <AboutTab />}
      </div>
    </div>
  );
}

// ── General Tab ───────────────────────────────────────────────────────────────

function GeneralTab() {
  const [autostart, setAutostart] = useState(false);
  const [loading, setLoading]     = useState(true);
  const [status, setStatus]       = useState<{ type: "success" | "error"; msg: string } | null>(null);

  useEffect(() => {
    getAutostart().then(setAutostart).finally(() => setLoading(false));
  }, []);

  async function toggleAutostart() {
    const next = !autostart;
    try {
      await setAutostartCmd(next);
      setAutostart(next);
      setStatus({
        type: "success",
        msg: next ? "LSPV will start with your system." : "Autostart disabled.",
      });
    } catch (err) {
      setStatus({ type: "error", msg: String(err) });
    }
  }

  if (loading) return <div className="p-6 text-[var(--muted)] text-sm">Loading…</div>;

  return (
    <div className="p-6 max-w-md mx-auto w-full flex flex-col gap-4">
      {/* Autostart toggle */}
      <div className="rounded-xl border border-[var(--border)] bg-[var(--surface)] p-4
                      flex items-center justify-between gap-4">
        <div>
          <div className="font-medium text-sm">Start with system</div>
          <div className="text-[var(--muted)] text-xs mt-0.5">
            Launch automatically when you log in.
          </div>
        </div>
        <button
          onClick={toggleAutostart}
          className={`relative w-11 h-6 rounded-full transition-colors flex-shrink-0 ${
            autostart ? "bg-[var(--accent)]" : "bg-[var(--border)]"
          }`}
          title={autostart ? "Disable autostart" : "Enable autostart"}
        >
          <span
            className={`absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white shadow-sm
                        transition-transform ${autostart ? "translate-x-5" : "translate-x-0"}`}
          />
        </button>
      </div>

      {/* Tray info (read-only) */}
      <div className="rounded-xl border border-[var(--border)] bg-[var(--surface)] p-4">
        <div className="font-medium text-sm mb-1">Minimize to tray</div>
        <div className="text-[var(--muted)] text-xs leading-relaxed">
          Closing the window hides LSPV to the system tray. Left-click the tray icon to
          toggle visibility. Right-click for Show, Lock, or Quit.
        </div>
      </div>

      {status && <Alert type={status.type}>{status.msg}</Alert>}
    </div>
  );
}

// ── Security Tab ──────────────────────────────────────────────────────────────

const LOCK_TIMEOUT_OPTIONS: { label: string; secs: number }[] = [
  { label: "Never",   secs: 0    },
  { label: "1 min",   secs: 60   },
  { label: "5 min",   secs: 300  },
  { label: "15 min",  secs: 900  },
  { label: "30 min",  secs: 1800 },
  { label: "1 hour",  secs: 3600 },
];

function SecurityTab() {
  const [oldPw,     setOldPw]     = useState("");
  const [newPw,     setNewPw]     = useState("");
  const [confirmPw, setConfirmPw] = useState("");
  const [pwError,   setPwError]   = useState("");
  const [pwSuccess, setPwSuccess] = useState(false);
  const [pwLoading, setPwLoading] = useState(false);

  const [lockSecs,       setLockSecs]       = useState(300);
  const [lockOnMinimize, setLockOnMinimize] = useState(false);
  const [alLoading,      setAlLoading]      = useState(true);
  const [alStatus,       setAlStatus]       = useState<{ type: "success" | "error"; msg: string } | null>(null);

  useEffect(() => {
    getAutoLockSettings()
      .then(s => { setLockSecs(s.secs); setLockOnMinimize(s.lockOnMinimize); })
      .catch(() => {})
      .finally(() => setAlLoading(false));
  }, []);

  async function handleChange(e: React.FormEvent) {
    e.preventDefault();
    setPwError(""); setPwSuccess(false);
    if (newPw.length < 8)    { setPwError("New password must be at least 8 characters."); return; }
    if (newPw !== confirmPw) { setPwError("Passwords do not match."); return; }
    setPwLoading(true);
    try {
      await changeMasterPassword(oldPw, newPw);
      setPwSuccess(true);
      setOldPw(""); setNewPw(""); setConfirmPw("");
    } catch (err) {
      const msg = String(err).toLowerCase();
      setPwError(msg.includes("decryption") ? "Current password is incorrect." : String(err));
    } finally {
      setPwLoading(false);
    }
  }

  async function saveAutoLock(secs: number, lom: boolean) {
    setAlStatus(null);
    try {
      await setAutoLockSettings(secs, lom);
      setAlStatus({ type: "success", msg: secs === 0 ? "Auto-lock disabled." : `Auto-lock set to ${LOCK_TIMEOUT_OPTIONS.find(o => o.secs === secs)?.label ?? secs + "s"}.` });
    } catch (err) {
      setAlStatus({ type: "error", msg: String(err) });
    }
  }

  return (
    <div className="p-6 max-w-md mx-auto w-full flex flex-col gap-6">

      {/* ── Auto-lock ── */}
      <div>
        <h3 className="font-semibold mb-3">Auto-lock</h3>

        {/* Timeout selector */}
        <div className="rounded-xl border border-[var(--border)] bg-[var(--surface)] p-4 flex flex-col gap-3">
          <div>
            <div className="font-medium text-sm">Lock vault after idle</div>
            <div className="text-[var(--muted)] text-xs mt-0.5">
              Vault locks automatically when there is no activity.
            </div>
          </div>
          <div className="grid grid-cols-3 gap-1.5">
            {LOCK_TIMEOUT_OPTIONS.map(opt => (
              <button
                key={opt.secs}
                disabled={alLoading}
                onClick={async () => { setLockSecs(opt.secs); await saveAutoLock(opt.secs, lockOnMinimize); }}
                className={`py-2 rounded-lg text-sm font-medium border transition-colors ${
                  lockSecs === opt.secs
                    ? "bg-[var(--accent)] text-white border-[var(--accent)]"
                    : "border-[var(--border)] text-[var(--muted)] hover:text-[var(--text)] hover:border-[var(--accent)]"
                }`}
              >
                {opt.label}
              </button>
            ))}
          </div>

          {/* Lock on minimize */}
          <div className="flex items-center justify-between gap-4 pt-1 border-t border-[var(--border)]">
            <div>
              <div className="font-medium text-sm">Lock when hidden</div>
              <div className="text-[var(--muted)] text-xs mt-0.5">
                Lock vault whenever the window is closed to tray.
              </div>
            </div>
            <button
              disabled={alLoading}
              onClick={async () => { const next = !lockOnMinimize; setLockOnMinimize(next); await saveAutoLock(lockSecs, next); }}
              className={`relative w-11 h-6 rounded-full transition-colors flex-shrink-0 ${
                lockOnMinimize ? "bg-[var(--accent)]" : "bg-[var(--border)]"
              }`}
            >
              <span className={`absolute top-0.5 left-0.5 w-5 h-5 rounded-full bg-white shadow-sm
                                transition-transform ${lockOnMinimize ? "translate-x-5" : "translate-x-0"}`} />
            </button>
          </div>
        </div>
        {alStatus && <div className="mt-2"><Alert type={alStatus.type}>{alStatus.msg}</Alert></div>}
      </div>

      {/* ── Change master password ── */}
      <div>
        <h3 className="font-semibold mb-3">Change master password</h3>
        <form onSubmit={handleChange} className="flex flex-col gap-4">
          <PasswordField label="Current password"     value={oldPw}     onChange={setOldPw}     autoFocus />
          <PasswordField label="New password"         value={newPw}     onChange={setNewPw}     placeholder="At least 8 characters" />
          <PasswordField label="Confirm new password" value={confirmPw} onChange={setConfirmPw} />
          {pwError   && <Alert type="error">{pwError}</Alert>}
          {pwSuccess && <Alert type="success">Password changed successfully.</Alert>}
          <button
            type="submit"
            disabled={pwLoading || !oldPw || !newPw || !confirmPw}
            className="w-full bg-[var(--accent)] hover:bg-[var(--accent-hover)] disabled:opacity-40
                       text-white font-medium py-3 rounded-xl transition-colors"
          >
            {pwLoading ? "Changing…" : "Change password"}
          </button>
        </form>
      </div>

    </div>
  );
}

// ── Browser Tab ───────────────────────────────────────────────────────────────

function BrowserTab() {
  const [cfg,     setCfg]     = useState<BrowserConfig>({ chromeIds: [], firefoxIds: [] });
  const [hostPath, setHostPath] = useState<string | null>(null);
  const [profiles, setProfiles] = useState<ProfileInfo[]>([]);
  const [chromeInput,  setChromeInput]  = useState("");
  const [firefoxInput, setFirefoxInput] = useState("");
  const [status, setStatus] = useState<{ type: "success" | "error"; msg: string } | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving,  setSaving]  = useState(false);

  useEffect(() => {
    Promise.all([getBrowserIntegrations(), getNativeHostPath(), getProfiles()])
      .then(([c, p, profs]) => { setCfg(c); setHostPath(p); setProfiles(profs); })
      .catch(console.error)
      .finally(() => setLoading(false));
  }, []);

  function addId(
    field: "chromeIds" | "firefoxIds",
    input: string,
    setInput: (v: string) => void,
  ) {
    const id = input.trim();
    if (!id || cfg[field].includes(id)) return;
    setCfg(prev => ({ ...prev, [field]: [...prev[field], id] }));
    setInput("");
    setStatus(null);
  }

  function removeId(field: "chromeIds" | "firefoxIds", id: string) {
    setCfg(prev => ({ ...prev, [field]: prev[field].filter(x => x !== id) }));
    setStatus(null);
  }

  async function handleApply() {
    setSaving(true); setStatus(null);
    try {
      const path = await saveBrowserIntegrations(cfg.chromeIds, cfg.firefoxIds);
      setHostPath(path);
      setStatus({ type: "success", msg: `Registered. Native host:\n${path}` });
    } catch (err) {
      setStatus({ type: "error", msg: String(err) });
    } finally {
      setSaving(false);
    }
  }

  if (loading) return <div className="p-6 text-[var(--muted)] text-sm">Loading…</div>;

  return (
    <div className="p-6 max-w-lg mx-auto w-full flex flex-col gap-6">
      {/* Native host status */}
      <div className="rounded-xl border border-[var(--border)] bg-[var(--surface)] p-4 text-sm">
        <div className="font-medium mb-1.5">Native host binary</div>
        {hostPath ? (
          <div className="text-[var(--success)] font-mono text-xs break-all">{hostPath}</div>
        ) : (
          <div>
            <div className="text-[var(--danger)] mb-1">Not found — build first:</div>
            <pre className="text-xs bg-black/30 rounded-lg px-3 py-2 text-[var(--muted)] overflow-x-auto">
              cargo build -p vaultpass-native-host --release
            </pre>
          </div>
        )}
      </div>

      {/* Chrome / Edge */}
      <IdSection
        label="Chrome / Edge extension IDs"
        hint='chrome://extensions → enable Developer mode → copy the ID shown under LSPV.'
        ids={cfg.chromeIds}
        input={chromeInput}
        onInputChange={setChromeInput}
        onAdd={() => addId("chromeIds", chromeInput, setChromeInput)}
        onRemove={id => removeId("chromeIds", id)}
      />

      {/* Firefox */}
      <IdSection
        label="Firefox extension ID"
        hint='The extension always uses a fixed ID: lspv@lspv.app — just click Add.'
        defaultValue="lspv@lspv.app"
        ids={cfg.firefoxIds}
        input={firefoxInput}
        onInputChange={setFirefoxInput}
        onAdd={() => addId("firefoxIds", firefoxInput, setFirefoxInput)}
        onRemove={id => removeId("firefoxIds", id)}
      />

      {/* Connected profiles */}
      <ProfilesSection
        profiles={profiles}
        onRename={(id, name) => {
          setProfiles(prev =>
            prev.map(p => p.id === id ? { ...p, name: name || null } : p)
          );
          setProfileName(id, name || null).catch(console.error);
        }}
      />

      {status && <Alert type={status.type}>{status.msg}</Alert>}

      <button
        onClick={handleApply}
        disabled={saving || (cfg.chromeIds.length === 0 && cfg.firefoxIds.length === 0)}
        className="w-full bg-[var(--accent)] hover:bg-[var(--accent-hover)] disabled:opacity-40
                   text-white font-medium py-3 rounded-xl transition-colors"
      >
        {saving ? "Applying…" : "Apply & Register"}
      </button>
    </div>
  );
}

interface IdSectionProps {
  label: string;
  hint: string;
  defaultValue?: string;
  ids: string[];
  input: string;
  onInputChange: (v: string) => void;
  onAdd: () => void;
  onRemove: (id: string) => void;
}

function IdSection({ label, hint, defaultValue, ids, input, onInputChange, onAdd, onRemove }: IdSectionProps) {
  function handleFocus() {
    if (!input && defaultValue) onInputChange(defaultValue);
  }

  return (
    <div className="flex flex-col gap-2">
      <div className="font-medium text-sm">{label}</div>
      <div className="text-xs text-[var(--muted)]">{hint}</div>
      <div className="flex gap-2">
        <input
          type="text"
          value={input}
          onChange={e => onInputChange(e.target.value)}
          onFocus={handleFocus}
          onKeyDown={e => e.key === "Enter" && onAdd()}
          placeholder={defaultValue ?? "Extension ID…"}
          className="flex-1 bg-[var(--surface)] border border-[var(--border)] rounded-lg px-3 py-2
                     text-sm font-mono text-[var(--text)] placeholder-[var(--muted)]
                     focus:outline-none focus:border-[var(--accent)] transition-colors"
        />
        <button
          onClick={onAdd}
          disabled={!input.trim()}
          className="px-4 py-2 bg-[var(--surface)] border border-[var(--border)] rounded-lg text-sm
                     hover:border-[var(--accent)] hover:text-[var(--accent)] disabled:opacity-40 transition-colors"
        >
          Add
        </button>
      </div>
      {ids.length > 0 && (
        <div className="flex flex-col gap-1">
          {ids.map(id => (
            <div key={id}
              className="flex items-center gap-2 bg-[var(--surface)] border border-[var(--border)]
                         rounded-lg px-3 py-2"
            >
              <span className="flex-1 font-mono text-xs text-[var(--text)] break-all">{id}</span>
              <button
                onClick={() => onRemove(id)}
                className="text-[var(--muted)] hover:text-[var(--danger)] text-sm flex-shrink-0 transition-colors"
                title="Remove"
              >
                ✕
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

// ── Import Tab ────────────────────────────────────────────────────────────────

function ImportTab({ onImported }: { onImported?: () => void }) {
  const fileRef = useRef<HTMLInputElement>(null);
  const [rows,         setRows]         = useState<ImportRow[]>([]);
  const [error,        setError]        = useState("");
  const [importing,    setImporting]    = useState(false);
  const [importedCount, setImportedCount] = useState<number | null>(null);
  const [profiles,     setProfiles]     = useState<ProfileInfo[]>([]);
  const [sourceTag,    setSourceTag]    = useState<string>("");
  const [retag,        setRetag]        = useState<{ old: string; new: string } | null>(null);

  useEffect(() => {
    getProfiles().then(setProfiles).catch(() => {});
  }, []);

  async function handleFile(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file) return;
    setError(""); setRows([]); setImportedCount(null);
    try {
      const text = await file.text();
      const parsed = await parseImportCsv(text);
      setRows(parsed);
    } catch (err) {
      setError(String(err));
    }
    e.target.value = "";
  }

  async function handleImport() {
    setImporting(true); setError("");
    try {
      const tag = sourceTag.trim() || null;
      const count = await importItemsFromCsv(rows, tag);
      setImportedCount(count);
      setRows([]);
      onImported?.();
    } catch (err) {
      setError(String(err));
    } finally {
      setImporting(false);
    }
  }

  async function handleRetag() {
    if (!retag || !retag.old.trim()) return;
    try {
      const n = await bulkRetagItems(retag.old.trim(), retag.new.trim() || null);
      setRetag(null);
      onImported?.();
      alert(`Updated ${n} item${n !== 1 ? "s" : ""}.`);
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <div className="p-6 max-w-lg mx-auto w-full flex flex-col gap-5">
      <div>
        <h3 className="font-semibold mb-1">Import from browser</h3>
        <p className="text-[var(--muted)] text-sm leading-relaxed">
          Accepts CSV exports from Chrome
          {" "}(<span className="font-mono text-xs">chrome://password-manager → Settings → Export</span>)
          {" "}and Firefox
          {" "}(<span className="font-mono text-xs">about:logins → ··· → Export Logins…</span>).
        </p>
      </div>

      <input ref={fileRef} type="file" accept=".csv,text/csv" onChange={handleFile} className="hidden" />

      <button
        onClick={() => fileRef.current?.click()}
        className="w-full border-2 border-dashed border-[var(--border)] hover:border-[var(--accent)]
                   rounded-xl py-8 text-sm text-[var(--muted)] hover:text-[var(--text)] transition-colors"
      >
        📂 Choose CSV file…
      </button>

      {error && <Alert type="error">{error}</Alert>}

      {importedCount !== null && (
        <Alert type="success">
          ✓ Imported {importedCount} item{importedCount !== 1 ? "s" : ""} — go back to see them in your vault.
        </Alert>
      )}

      {rows.length > 0 && (
        <>
          <div className="text-sm text-[var(--muted)]">{rows.length} items found:</div>
          <div className="border border-[var(--border)] rounded-xl overflow-hidden max-h-60 overflow-y-auto">
            <table className="w-full text-xs">
              <thead className="sticky top-0 bg-[var(--surface)] border-b border-[var(--border)]">
                <tr>
                  <th className="text-left px-3 py-2 text-[var(--muted)] font-medium">Title</th>
                  <th className="text-left px-3 py-2 text-[var(--muted)] font-medium">URL</th>
                  <th className="text-left px-3 py-2 text-[var(--muted)] font-medium">Username</th>
                </tr>
              </thead>
              <tbody>
                {rows.map((r, i) => (
                  <tr key={i} className="border-b border-[var(--border)]/40 last:border-0 hover:bg-[var(--surface)]/50">
                    <td className="px-3 py-1.5 text-[var(--text)]">{r.title}</td>
                    <td className="px-3 py-1.5 text-[var(--muted)] truncate max-w-[130px]" title={r.url}>{r.url}</td>
                    <td className="px-3 py-1.5 text-[var(--muted)]">{r.username}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          {/* Source tag for this import batch */}
          <div className="flex flex-col gap-1.5">
            <label className="text-xs font-medium text-[var(--muted)] uppercase tracking-wide">
              Tag these imports as (optional)
            </label>
            {profiles.length > 0 && (
              <div className="flex flex-wrap gap-1.5">
                {profiles.map(p => {
                  const label = p.name || p.email || p.id.slice(0, 8);
                  return (
                    <button
                      key={p.id}
                      type="button"
                      onClick={() => setSourceTag(tag => tag === label ? "" : label)}
                      className={`px-2.5 py-1 rounded-full text-xs border transition-colors
                        ${sourceTag === label
                          ? "bg-[var(--accent)]/20 border-[var(--accent)] text-[var(--accent)]"
                          : "border-[var(--border)] text-[var(--muted)] hover:text-[var(--text)]"
                        }`}
                    >
                      {label}
                    </button>
                  );
                })}
              </div>
            )}
            <input
              type="text"
              value={sourceTag}
              onChange={e => setSourceTag(e.target.value)}
              placeholder="or type a custom label…"
              className="w-full bg-[var(--bg)] border border-[var(--border)] rounded-lg px-3 py-2
                         text-sm text-[var(--text)] placeholder-[var(--muted)]
                         focus:outline-none focus:border-[var(--accent)] transition-colors"
            />
          </div>

          <button
            onClick={handleImport}
            disabled={importing}
            className="w-full bg-[var(--accent)] hover:bg-[var(--accent-hover)] disabled:opacity-40
                       text-white font-medium py-3 rounded-xl transition-colors"
          >
            {importing ? "Importing…" : `Import ${rows.length} item${rows.length !== 1 ? "s" : ""}${sourceTag.trim() ? ` → "${sourceTag.trim()}"` : ""}`}
          </button>
        </>
      )}

      {/* Bulk retag section */}
      <div className="border-t border-[var(--border)] pt-4 flex flex-col gap-3">
        <h4 className="font-medium text-sm">Bulk re-tag</h4>
        <p className="text-xs text-[var(--muted)]">
          Rename or remove a source tag from all items at once.
        </p>
        {retag === null ? (
          <button
            type="button"
            onClick={() => setRetag({ old: "", new: "" })}
            className="text-sm text-[var(--accent)] hover:underline self-start"
          >
            + Rename or clear a tag…
          </button>
        ) : (
          <div className="flex flex-col gap-2">
            <input
              type="text"
              value={retag.old}
              onChange={e => setRetag(r => r ? { ...r, old: e.target.value } : r)}
              placeholder="Existing tag to rename"
              className="w-full bg-[var(--bg)] border border-[var(--border)] rounded-lg px-3 py-2
                         text-sm text-[var(--text)] placeholder-[var(--muted)]
                         focus:outline-none focus:border-[var(--accent)] transition-colors"
            />
            <input
              type="text"
              value={retag.new}
              onChange={e => setRetag(r => r ? { ...r, new: e.target.value } : r)}
              placeholder="New tag name (leave empty to clear)"
              className="w-full bg-[var(--bg)] border border-[var(--border)] rounded-lg px-3 py-2
                         text-sm text-[var(--text)] placeholder-[var(--muted)]
                         focus:outline-none focus:border-[var(--accent)] transition-colors"
            />
            <div className="flex gap-2">
              <button
                type="button"
                onClick={() => setRetag(null)}
                className="flex-1 border border-[var(--border)] text-[var(--muted)]
                           hover:text-[var(--text)] py-2 rounded-xl text-sm transition-colors"
              >
                Cancel
              </button>
              <button
                type="button"
                onClick={handleRetag}
                disabled={!retag.old.trim()}
                className="flex-1 bg-[var(--accent)] hover:bg-[var(--accent-hover)] disabled:opacity-40
                           text-white font-medium py-2 rounded-xl text-sm transition-colors"
              >
                Apply
              </button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

// ── Profiles Section ──────────────────────────────────────────────────────────

function timeAgo(ms: number): string {
  const d = Date.now() - ms;
  if (d < 60_000)        return "just now";
  if (d < 3_600_000)    return `${Math.floor(d / 60_000)}m ago`;
  if (d < 86_400_000)   return `${Math.floor(d / 3_600_000)}h ago`;
  return `${Math.floor(d / 86_400_000)}d ago`;
}

function BrowserBadge({ type }: { type: string | null }) {
  const map: Record<string, { label: string; color: string }> = {
    chrome:  { label: "Chrome",  color: "bg-blue-900/40 text-blue-300 border-blue-800/50" },
    firefox: { label: "Firefox", color: "bg-orange-900/40 text-orange-300 border-orange-800/50" },
    edge:    { label: "Edge",    color: "bg-teal-900/40 text-teal-300 border-teal-800/50" },
  };
  const b = type ? (map[type] ?? { label: type, color: "bg-[var(--surface)] text-[var(--muted)] border-[var(--border)]" }) : null;
  if (!b) return null;
  return (
    <span className={`text-[10px] font-medium px-1.5 py-0.5 rounded border ${b.color} flex-shrink-0`}>
      {b.label}
    </span>
  );
}

interface ProfilesSectionProps {
  profiles: ProfileInfo[];
  onRename: (id: string, name: string) => void;
}

function ProfilesSection({ profiles, onRename }: ProfilesSectionProps) {
  const [editing, setEditing] = useState<string | null>(null);
  const [draft,   setDraft]   = useState("");

  function startEdit(p: ProfileInfo) {
    setEditing(p.id);
    setDraft(p.name ?? p.email ?? "");
  }

  function commitEdit(id: string) {
    onRename(id, draft.trim());
    setEditing(null);
  }

  return (
    <div className="flex flex-col gap-2">
      <div className="font-medium text-sm">Connected profiles</div>
      <div className="text-xs text-[var(--muted)]">
        Each Chrome / Firefox profile that connects to LSPV appears here.
        You can give them a friendly name.
      </div>

      {profiles.length === 0 ? (
        <div className="text-xs text-[var(--muted)] italic py-1">
          No profiles yet — connect a browser to see them here.
        </div>
      ) : (
        <div className="flex flex-col gap-2">
          {profiles.map(p => {
            const defaultLabel = p.email ?? "Local profile";
            const displayName  = p.name ?? defaultLabel;
            return (
              <div
                key={p.id}
                className="rounded-xl border border-[var(--border)] bg-[var(--surface)] p-3 flex items-start gap-3"
              >
                <div className="flex-shrink-0 text-lg">
                  {p.email ? "👤" : "🖥"}
                </div>
                <div className="flex-1 min-w-0">
                  {editing === p.id ? (
                    <div className="flex gap-2">
                      <input
                        value={draft}
                        onChange={e => setDraft(e.target.value)}
                        onKeyDown={e => {
                          if (e.key === "Enter") commitEdit(p.id);
                          if (e.key === "Escape") setEditing(null);
                        }}
                        autoFocus
                        className="flex-1 bg-[var(--bg)] border border-[var(--accent)] rounded-lg px-2 py-1
                                   text-sm text-[var(--text)] focus:outline-none"
                      />
                      <button
                        onClick={() => commitEdit(p.id)}
                        className="text-xs px-3 py-1 bg-[var(--accent)] text-white rounded-lg"
                      >
                        Save
                      </button>
                    </div>
                  ) : (
                    <div className="flex items-center gap-2">
                      <span className="text-sm font-medium text-[var(--text)] truncate">{displayName}</span>
                      <BrowserBadge type={p.browserType} />
                      <button
                        onClick={() => startEdit(p)}
                        className="text-[var(--muted)] hover:text-[var(--accent)] text-xs flex-shrink-0 transition-colors"
                        title="Rename"
                      >
                        ✏
                      </button>
                    </div>
                  )}
                  {p.email && p.name && (
                    <div className="text-xs text-[var(--muted)] mt-0.5">{p.email}</div>
                  )}
                  <div className="text-xs text-[var(--muted)] mt-0.5">
                    Last seen: {timeAgo(p.lastSeenMs)}
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}

// ── About Tab ─────────────────────────────────────────────────────────────────

function AboutTab() {
  const [copied, setCopied] = useState(false);
  const GITHUB = "https://github.com/RakinSV/local-security-pass-vault";

  async function handleGithub() {
    try {
      await openGithub();
    } catch {
      // Fallback: copy URL to clipboard
      await navigator.clipboard.writeText(GITHUB).catch(() => {});
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  }

  return (
    <div className="p-6 max-w-md mx-auto w-full flex flex-col gap-6">
      <div className="flex items-center gap-4">
        <div className="text-5xl">🔐</div>
        <div>
          <h2 className="font-bold text-base text-[var(--text)] leading-tight">
            Local Security Pass Vault
          </h2>
          <div className="text-xs text-[var(--muted)] mt-0.5">Version 0.1.0</div>
        </div>
      </div>

      <div className="rounded-xl border border-[var(--border)] bg-[var(--surface)] p-4 text-sm
                      text-[var(--muted)] leading-relaxed">
        Local password manager. No cloud, no telemetry, no network requests.
        All data is stored locally and encrypted with libsodium (XChaCha20-Poly1305 + Argon2id).
      </div>

      <div className="flex flex-col gap-2">
        <div className="text-xs font-medium text-[var(--muted)] uppercase tracking-wider">Stack</div>
        <div className="rounded-xl border border-[var(--border)] bg-[var(--surface)] divide-y divide-[var(--border)]">
          {[
            ["Crypto",   "libsodium · XChaCha20-Poly1305 · Argon2id"],
            ["Desktop",  "Rust · Tauri 2 · React · Tailwind"],
            ["Database", "SQLite · SQLCipher (AES-256)"],
            ["Backups",  "BIP-39 24-word · BLAKE3 · XChaCha20"],
            ["Keychain", "OS native (Windows / Linux / macOS)"],
          ].map(([k, v]) => (
            <div key={k} className="flex gap-3 px-4 py-2.5 text-sm">
              <span className="text-[var(--muted)] w-20 flex-shrink-0">{k}</span>
              <span className="text-[var(--text)] text-xs">{v}</span>
            </div>
          ))}
        </div>
      </div>

      <div className="flex flex-col gap-2">
        <div className="text-xs font-medium text-[var(--muted)] uppercase tracking-wider">Source code</div>
        <button
          onClick={handleGithub}
          className="flex items-center gap-3 rounded-xl border border-[var(--border)] bg-[var(--surface)]
                     hover:border-[var(--accent)] hover:bg-[var(--surface)] px-4 py-3 transition-colors
                     text-left group w-full"
        >
          <span className="text-lg">⌥</span>
          <div className="flex-1">
            <div className="text-sm font-medium text-[var(--text)] group-hover:text-[var(--accent)] transition-colors">
              {copied ? "URL copied!" : "Open on GitHub"}
            </div>
            <div className="text-xs text-[var(--muted)] font-mono mt-0.5 truncate">{GITHUB}</div>
          </div>
          <span className="text-[var(--muted)] text-xs">↗</span>
        </button>
      </div>

      <div className="text-xs text-[var(--muted)] text-center">
        MIT License · Made with Rust 🦀
      </div>
    </div>
  );
}

// ── Shared ────────────────────────────────────────────────────────────────────

function Alert({ type, children }: { type: "success" | "error"; children: React.ReactNode }) {
  return (
    <div className={`text-sm rounded-lg px-3 py-2 whitespace-pre-wrap ${
      type === "error"
        ? "text-[var(--danger)] bg-red-950/30 border border-red-900/40"
        : "text-[var(--success)] bg-green-950/30 border border-green-900/40"
    }`}>
      {children}
    </div>
  );
}
