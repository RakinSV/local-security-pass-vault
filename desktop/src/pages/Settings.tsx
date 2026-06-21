import { useState, useEffect, useRef } from "react";
import { ExtensionInstaller } from "../components/ExtensionInstaller";
import { PasswordField } from "../components/PasswordField";
import {
  changeMasterPassword,
  getBrowserIntegrations,
  saveBrowserIntegrations,
  getNativeHostPath,
  parseImportCsv,
  importItemsFromCsv,
  importBitwardenJson,
  getProfiles,
  setProfileName,
  openGithub,
  getAutostart,
  setAutostart as setAutostartCmd,
  bulkRetagItems,
  getAutoLockSettings,
  setAutoLockSettings,
  generateSeedPhrase,
  validateSeedPhrase,
  exportBackup,
  restoreBackup,
  listAutoBackups,
  pickBackupFile,
  pickBackupSavePath,
  keychainVaultStatus,
  keychainDeleteKey,
  pickFolder,
  vaultStatus,
  listDeletedItems,
  restoreItem,
  purgeItem,
  purgeAllTrash,
  getHealthReport,
  pickCsvSavePath,
  exportItemsCsv,
  vaultHas2fa,
  setupVault2fa,
  confirmVault2fa,
  disableVault2fa,
} from "../api/vault";
import type { VaultTwoFaSetup } from "../api/vault";
import type { AutoBackupEntry, HealthEntry, KeychainVaultStatus } from "../api/vault";
import type { BrowserConfig, ImportRow, ItemSummary, ProfileInfo } from "../types/vault";
import { getThemeMode, getAccentColor, saveTheme, type ThemeMode } from "../theme";

interface Props {
  onBack: () => void;
  onImported?: () => void;
}

type Tab = "general" | "security" | "backup" | "browser" | "data" | "about";

export function Settings({ onBack, onImported }: Props) {
  const [tab, setTab] = useState<Tab>("general");

  const tabs: { id: Tab; label: string }[] = [
    { id: "general",  label: "General"  },
    { id: "security", label: "Security" },
    { id: "backup",   label: "Backup"   },
    { id: "browser",  label: "Browser"  },
    { id: "data",     label: "Data"     },
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
        {tab === "backup"   && <BackupTab />}
        {tab === "browser"  && <BrowserTab />}
        {tab === "data"     && <DataTab onImported={onImported} />}
        {tab === "about"    && <AboutTab />}
      </div>
    </div>
  );
}

// ── General Tab ───────────────────────────────────────────────────────────────

const ACCENT_PRESETS = [
  { label: "Indigo",  color: "#6366f1" },
  { label: "Blue",    color: "#3b82f6" },
  { label: "Violet",  color: "#8b5cf6" },
  { label: "Rose",    color: "#f43f5e" },
  { label: "Emerald", color: "#10b981" },
  { label: "Amber",   color: "#f59e0b" },
  { label: "Cyan",    color: "#06b6d4" },
  { label: "Fuchsia", color: "#d946ef" },
];

function GeneralTab() {
  const [autostart, setAutostart] = useState(false);
  const [loading, setLoading]     = useState(true);
  const [status, setStatus]       = useState<{ type: "success" | "error"; msg: string } | null>(null);

  const [themeMode, setThemeMode] = useState<ThemeMode>(getThemeMode());
  const [accent, setAccent]       = useState(getAccentColor());

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

  function applyTheme(mode: ThemeMode, color: string) {
    setThemeMode(mode);
    setAccent(color);
    saveTheme(mode, color);
  }

  if (loading) return <div className="p-6 text-[var(--muted)] text-sm">Loading…</div>;

  return (
    <div className="p-6 max-w-md mx-auto w-full flex flex-col gap-4">
      {/* Theme */}
      <div className="rounded-xl border border-[var(--border)] bg-[var(--surface)] p-4 flex flex-col gap-3">
        <div className="font-medium text-sm">Appearance</div>
        <div className="flex gap-2">
          {(["dark", "light", "system"] as ThemeMode[]).map(m => (
            <button
              key={m}
              onClick={() => applyTheme(m, accent)}
              className={`flex-1 py-1.5 rounded-lg text-xs font-medium border transition-colors capitalize
                ${ themeMode === m
                  ? "border-[var(--accent)] bg-[var(--accent)]/10 text-[var(--accent)]"
                  : "border-[var(--border)] text-[var(--muted)] hover:text-[var(--text)]"
                }`}
            >
              {m === "dark" ? "🌙 Dark" : m === "light" ? "☀️ Light" : "💻 System"}
            </button>
          ))}
        </div>

        <div>
          <div className="text-xs text-[var(--muted)] mb-2">Accent color</div>
          <div className="flex flex-wrap gap-2">
            {ACCENT_PRESETS.map(p => (
              <button
                key={p.color}
                title={p.label}
                onClick={() => applyTheme(themeMode, p.color)}
                className={`w-7 h-7 rounded-full transition-all border-2 ${
                  accent === p.color ? "border-white scale-110" : "border-transparent hover:scale-105"
                }`}
                style={{ backgroundColor: p.color }}
              />
            ))}
            <label className="w-7 h-7 rounded-full border-2 border-dashed border-[var(--border)]
                              hover:border-[var(--accent)] cursor-pointer flex items-center justify-center"
                   title="Custom color">
              <input
                type="color"
                value={accent}
                onChange={e => applyTheme(themeMode, e.target.value)}
                className="opacity-0 absolute w-0 h-0"
              />
              <span className="text-[var(--muted)] text-xs">+</span>
            </label>
          </div>
        </div>
      </div>

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

// ── Two-Factor Authentication section (shown when vault is open) ──────────────

type TwoFaStep = "idle" | "setup" | "disable";

function TwoFaSection({ vaultOpen }: { vaultOpen: boolean }) {
  const [has2fa,   setHas2fa]   = useState<boolean | null>(null);
  const [step,     setStep]     = useState<TwoFaStep>("idle");
  const [setup,    setSetup]    = useState<VaultTwoFaSetup | null>(null);
  const [code,     setCode]     = useState("");
  const [error,    setError]    = useState("");
  const [success,  setSuccess]  = useState("");
  const [loading,  setLoading]  = useState(false);

  useEffect(() => {
    if (!vaultOpen) return;
    vaultHas2fa().then(setHas2fa).catch(() => setHas2fa(false));
  }, [vaultOpen]);

  if (!vaultOpen || has2fa === null) return null;

  async function startSetup() {
    setError(""); setSuccess(""); setLoading(true);
    try {
      const s = await setupVault2fa();
      setSetup(s);
      setStep("setup");
    } catch (err) { setError(String(err)); }
    finally { setLoading(false); }
  }

  async function confirmEnable() {
    if (!setup) return;
    setError(""); setLoading(true);
    try {
      await confirmVault2fa(setup.secret, code);
      setHas2fa(true);
      setStep("idle");
      setSetup(null);
      setCode("");
      setSuccess("Two-factor authentication enabled.");
    } catch (err) {
      const msg = String(err).toLowerCase();
      setError(msg.includes("two-factor") ? "Invalid code — please check your authenticator app." : String(err));
    } finally { setLoading(false); }
  }

  async function confirmDisable() {
    setError(""); setLoading(true);
    try {
      await disableVault2fa(code);
      setHas2fa(false);
      setStep("idle");
      setCode("");
      setSuccess("Two-factor authentication disabled.");
    } catch (err) {
      const msg = String(err).toLowerCase();
      setError(msg.includes("two-factor") ? "Invalid code — please check your authenticator app." : String(err));
    } finally { setLoading(false); }
  }

  return (
    <div>
      <h3 className="font-semibold mb-3">Two-Factor Authentication</h3>
      <div className="rounded-xl border border-[var(--border)] bg-[var(--surface)] p-4 flex flex-col gap-4">

        {/* Status row */}
        <div className="flex items-center justify-between gap-4">
          <div>
            <div className="font-medium text-sm">
              {has2fa ? "2FA is enabled" : "2FA is disabled"}
            </div>
            <div className="text-[var(--muted)] text-xs mt-0.5">
              {has2fa
                ? "A TOTP code from your authenticator app is required to unlock this vault."
                : "Add an extra layer of protection with an authenticator app (TOTP)."}
            </div>
          </div>
          {step === "idle" && (
            <button
              onClick={has2fa ? () => { setStep("disable"); setError(""); setCode(""); } : startSetup}
              disabled={loading}
              className={`text-sm px-3 py-2 rounded-lg border flex-shrink-0 transition-colors disabled:opacity-40 ${
                has2fa
                  ? "border-[var(--danger)] text-[var(--danger)] hover:bg-red-950/20"
                  : "border-[var(--accent)] text-[var(--accent)] hover:bg-[var(--accent)]/10"
              }`}
            >
              {loading ? "…" : has2fa ? "Disable" : "Enable"}
            </button>
          )}
        </div>

        {/* Setup: show QR + secret */}
        {step === "setup" && setup && (
          <div className="flex flex-col gap-3 border-t border-[var(--border)] pt-4">
            <p className="text-sm text-[var(--muted)]">
              Scan the QR code with your authenticator app (Google Authenticator, Aegis, Authy, etc.),
              then enter the 6-digit code to confirm.
            </p>

            {/* QR code */}
            {setup.qrSvg && (
              <div className="flex flex-col items-center gap-1.5">
                <div className="p-2 bg-white rounded-xl inline-block shadow-sm">
                  <img
                    src={`data:image/svg+xml,${encodeURIComponent(setup.qrSvg)}`}
                    alt="Scan this QR code with your authenticator app"
                    className="w-44 h-44 block"
                    draggable={false}
                  />
                </div>
                <span className="text-xs text-[var(--muted)]">Scan with your authenticator app</span>
              </div>
            )}

            {/* Secret key for manual entry */}
            <div>
              <div className="text-xs text-[var(--muted)] mb-1">Or enter the key manually</div>
              <div className="flex items-center gap-2">
                <code className="flex-1 bg-[var(--bg)] border border-[var(--border)] rounded-lg px-3 py-2
                                 text-xs font-mono break-all select-all">
                  {setup.secret}
                </code>
                <button
                  onClick={() => navigator.clipboard.writeText(setup.secret)}
                  className="text-xs px-2 py-2 rounded-lg border border-[var(--border)]
                             hover:bg-[var(--accent)]/10 transition-colors flex-shrink-0"
                  title="Copy secret key"
                >
                  Copy
                </button>
              </div>
            </div>

            {/* Warning */}
            <div className="rounded-lg border border-amber-800/40 bg-amber-950/25 px-3 py-2.5 text-xs text-amber-300 leading-relaxed">
              <span className="font-semibold">Save this key now.</span>{" "}
              If you lose access to your authenticator app and don't have a copy of this key,
              you will be permanently locked out of your vault.
              Your TOTP secret is included in <code className="font-mono">.vbk</code> backup files — keep your backups safe.
            </div>

            <div>
              <label className="text-xs text-[var(--muted)] mb-1 block">
                Enter the 6-digit code from your authenticator app
              </label>
              <input
                type="text"
                inputMode="numeric"
                maxLength={6}
                value={code}
                onChange={e => setCode(e.target.value.replace(/\D/g, ""))}
                placeholder="000000"
                className="w-full px-3 py-2 rounded-lg border border-[var(--border)]
                           bg-[var(--input-bg)] text-[var(--text)] text-center text-xl
                           tracking-[0.4em] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
                autoComplete="one-time-code"
              />
            </div>
            {error && <Alert type="error">{error}</Alert>}
            <div className="flex gap-2">
              <button
                onClick={() => { setStep("idle"); setSetup(null); setCode(""); setError(""); }}
                className="flex-1 py-2 rounded-lg border border-[var(--border)] text-sm hover:bg-[var(--accent)]/5 transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={confirmEnable}
                disabled={loading || code.length < 6}
                className="flex-1 py-2 rounded-lg bg-[var(--accent)] text-white text-sm
                           disabled:opacity-40 hover:bg-[var(--accent-hover)] transition-colors"
              >
                {loading ? "Verifying…" : "Confirm Enable"}
              </button>
            </div>
          </div>
        )}

        {/* Disable: verify current code */}
        {step === "disable" && (
          <div className="flex flex-col gap-3 border-t border-[var(--border)] pt-4">
            <p className="text-sm text-[var(--muted)]">
              Enter the current 6-digit code from your authenticator app to disable 2FA.
            </p>
            <input
              type="text"
              inputMode="numeric"
              maxLength={6}
              value={code}
              onChange={e => setCode(e.target.value.replace(/\D/g, ""))}
              placeholder="000000"
              className="w-full px-3 py-2 rounded-lg border border-[var(--border)]
                         bg-[var(--input-bg)] text-[var(--text)] text-center text-xl
                         tracking-[0.4em] focus:outline-none focus:ring-2 focus:ring-[var(--danger)]"
              autoComplete="one-time-code"
            />
            {error && <Alert type="error">{error}</Alert>}
            <div className="flex gap-2">
              <button
                onClick={() => { setStep("idle"); setCode(""); setError(""); }}
                className="flex-1 py-2 rounded-lg border border-[var(--border)] text-sm hover:bg-[var(--accent)]/5 transition-colors"
              >
                Cancel
              </button>
              <button
                onClick={confirmDisable}
                disabled={loading || code.length < 6}
                className="flex-1 py-2 rounded-lg bg-[var(--danger)] text-white text-sm
                           disabled:opacity-40 transition-colors"
              >
                {loading ? "Disabling…" : "Disable 2FA"}
              </button>
            </div>
          </div>
        )}

        {success && step === "idle" && <Alert type="success">{success}</Alert>}
      </div>
    </div>
  );
}

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

  const [kcStatus,    setKcStatus]    = useState<KeychainVaultStatus | null>(null);
  const [removingKey, setRemovingKey] = useState(false);

  useEffect(() => {
    getAutoLockSettings()
      .then(s => { setLockSecs(s.secs); setLockOnMinimize(s.lockOnMinimize); })
      .catch(() => {})
      .finally(() => setAlLoading(false));
    keychainVaultStatus().then(setKcStatus).catch(() => {});
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

  async function handleRemoveCachedKey() {
    if (!kcStatus?.vaultUuid) return;
    setRemovingKey(true);
    try {
      await keychainDeleteKey(kcStatus.vaultUuid);
      setKcStatus(prev => prev ? { ...prev, hasCachedKey: false } : prev);
    } catch { /* ignore */ } finally {
      setRemovingKey(false);
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

      {/* ── OS Keychain quick unlock ── */}
      {kcStatus?.vaultOpen && (
        <div>
          <h3 className="font-semibold mb-3">OS Keychain (Quick Unlock)</h3>
          <div className="rounded-xl border border-[var(--border)] bg-[var(--surface)] p-4 flex items-center justify-between gap-4">
            <div>
              <div className="font-medium text-sm">
                {kcStatus.hasCachedKey ? "Key cached — quick unlock active" : "Key not cached"}
              </div>
              <div className="text-[var(--muted)] text-xs mt-0.5">
                {kcStatus.hasCachedKey
                  ? "Your vault key is stored in the OS keychain (DPAPI / libsecret / Keychain). Quick unlock is active."
                  : "The vault key will be cached after the next unlock."}
              </div>
            </div>
            {kcStatus.hasCachedKey && (
              <button
                onClick={handleRemoveCachedKey}
                disabled={removingKey}
                className="text-sm px-3 py-2 rounded-lg border border-[var(--danger)] text-[var(--danger)]
                           hover:bg-red-950/20 disabled:opacity-40 transition-colors flex-shrink-0"
              >
                {removingKey ? "Removing…" : "Remove"}
              </button>
            )}
          </div>
        </div>
      )}

      {/* ── Two-Factor Authentication ── */}
      <TwoFaSection vaultOpen={kcStatus?.vaultOpen ?? false} />

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

// ── Backup Tab ────────────────────────────────────────────────────────────────

type ExportStep = "idle" | "showPhrase" | "verifying";

function BackupTab() {
  const [vaultOpen,   setVaultOpen]   = useState(false);

  // Export state machine
  const [exportStep,    setExportStep]    = useState<ExportStep>("idle");
  const [phrase,        setPhrase]        = useState("");
  const [confirmed,     setConfirmed]     = useState(false);
  const [verifyIdxs,    setVerifyIdxs]    = useState<number[]>([]);
  const [verifyInputs,  setVerifyInputs]  = useState(["", "", ""]);
  const [generating,    setGenerating]    = useState(false);
  const [exporting,     setExporting]     = useState(false);
  const [exportStatus,  setExportStatus]  = useState<{ type: "success" | "error"; msg: string } | null>(null);

  // Auto-backups
  const [autoBackups, setAutoBackups] = useState<AutoBackupEntry[]>([]);

  // Restore flow
  const [restorePhrase,   setRestorePhrase]   = useState("");
  const [restoreFile,     setRestoreFile]     = useState<string | null>(null);
  const [restoreDestDir,  setRestoreDestDir]  = useState<string | null>(null);
  const [restoring,       setRestoring]       = useState(false);
  const [restoreStatus,   setRestoreStatus]   = useState<{ type: "success" | "error"; msg: string } | null>(null);

  const words = phrase.split(" ");
  const verifyOk = verifyIdxs.length === 3 &&
    verifyIdxs.every((idx, i) => verifyInputs[i].trim().toLowerCase() === words[idx]);

  useEffect(() => {
    vaultStatus().then(s => setVaultOpen(!s.isLocked)).catch(() => {});
    listAutoBackups().then(setAutoBackups).catch(() => {});
  }, []);

  function refreshBackups() {
    listAutoBackups().then(setAutoBackups).catch(() => {});
  }

  async function handleGenerate() {
    setGenerating(true);
    setExportStatus(null);
    try {
      const p = await generateSeedPhrase();
      setPhrase(p);
      setConfirmed(false);
      setExportStep("showPhrase");
    } catch (err) {
      setExportStatus({ type: "error", msg: String(err) });
    } finally {
      setGenerating(false);
    }
  }

  function handleStartVerify() {
    const positions: number[] = [];
    while (positions.length < 3) {
      const n = Math.floor(Math.random() * 24);
      if (!positions.includes(n)) positions.push(n);
    }
    positions.sort((a, b) => a - b);
    setVerifyIdxs(positions);
    setVerifyInputs(["", "", ""]);
    setExportStep("verifying");
  }

  async function handleExport() {
    setExporting(true);
    setExportStatus(null);
    try {
      const savePath = await pickBackupSavePath();
      if (!savePath) { setExporting(false); return; }
      await exportBackup(savePath, phrase);
      setPhrase("");
      setExportStep("idle");
      setExportStatus({ type: "success", msg: "Backup saved. A copy was also added to the auto-backups folder." });
      refreshBackups();
    } catch (err) {
      setExportStatus({ type: "error", msg: String(err) });
    } finally {
      setExporting(false);
    }
  }

  async function handleRestore() {
    if (!restoreFile || !restoreDestDir) return;
    const ws = restorePhrase.trim().split(/\s+/);
    if (ws.length !== 24) {
      setRestoreStatus({ type: "error", msg: "Please enter all 24 seed phrase words." });
      return;
    }
    const isValid = await validateSeedPhrase(restorePhrase.trim()).catch(() => false);
    if (!isValid) {
      setRestoreStatus({ type: "error", msg: "Invalid seed phrase — check that all words are valid BIP-39 words." });
      return;
    }
    setRestoring(true);
    setRestoreStatus(null);
    try {
      await restoreBackup(restoreFile, restoreDestDir, restorePhrase.trim());
      setRestoreStatus({ type: "success", msg: "Vault restored. Close settings and open it from the vault picker." });
      setRestoreFile(null);
      setRestoreDestDir(null);
      setRestorePhrase("");
    } catch (err) {
      const msg = String(err).toLowerCase();
      setRestoreStatus({
        type: "error",
        msg: msg.includes("decrypt") || msg.includes("checksum")
          ? "Incorrect seed phrase or corrupted backup file."
          : String(err),
      });
    } finally {
      setRestoring(false);
    }
  }

  function formatDate(ts: number): string {
    const d = new Date(ts * 1000);
    const now = new Date();
    const yesterday = new Date(now);
    yesterday.setDate(now.getDate() - 1);
    const time = d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
    if (d.toDateString() === now.toDateString())       return `Today ${time}`;
    if (d.toDateString() === yesterday.toDateString()) return `Yesterday ${time}`;
    return d.toLocaleDateString([], { month: "short", day: "numeric" }) + " " + time;
  }

  function formatSize(bytes: number): string {
    if (bytes < 1024)          return bytes + " B";
    if (bytes < 1024 * 1024)   return (bytes / 1024).toFixed(1) + " KB";
    return (bytes / (1024 * 1024)).toFixed(1) + " MB";
  }

  return (
    <div className="p-6 max-w-lg mx-auto w-full flex flex-col gap-6">

      {/* ── Export ── */}
      <div>
        <h3 className="font-semibold mb-3">Export encrypted backup</h3>

        {!vaultOpen ? (
          <div className="rounded-xl border border-[var(--border)] bg-[var(--surface)] p-4 text-sm text-[var(--muted)]">
            Unlock your vault first to create a backup.
          </div>

        ) : exportStep === "idle" ? (
          <div className="rounded-xl border border-[var(--border)] bg-[var(--surface)] p-4 flex flex-col gap-3">
            <p className="text-sm text-[var(--muted)] leading-relaxed">
              Export a <strong className="text-[var(--text)]">.vbk</strong> backup encrypted
              with a 24-word seed phrase. Even if the file leaks, it cannot be opened without
              the phrase.
            </p>
            <button
              onClick={handleGenerate}
              disabled={generating}
              className="w-full bg-[var(--accent)] hover:bg-[var(--accent-hover)] disabled:opacity-40
                         text-white font-medium py-3 rounded-xl transition-colors"
            >
              {generating ? "Generating…" : "Generate backup phrase"}
            </button>
          </div>

        ) : exportStep === "showPhrase" ? (
          <div className="rounded-xl border border-[var(--border)] bg-[var(--surface)] p-4 flex flex-col gap-4">
            <div>
              <div className="font-medium text-sm mb-1">Your 24-word seed phrase</div>
              <div className="text-xs text-[var(--muted)]">
                Write these words on paper and store them safely. VaultPass never saves
                this phrase to disk.
              </div>
            </div>
            {/* 4 × 6 word grid */}
            <div className="grid grid-cols-4 gap-1.5 bg-black/10 rounded-xl p-3 select-all">
              {words.map((w, i) => (
                <div key={i} className="flex items-center gap-1 text-xs">
                  <span className="text-[var(--muted)] w-5 text-right flex-shrink-0 font-mono tabular-nums">
                    {i + 1}.
                  </span>
                  <span className="text-[var(--text)] font-mono font-medium">{w}</span>
                </div>
              ))}
            </div>
            <label className="flex items-center gap-3 cursor-pointer">
              <input
                type="checkbox"
                checked={confirmed}
                onChange={e => setConfirmed(e.target.checked)}
                className="w-4 h-4 accent-[var(--accent)]"
              />
              <span className="text-sm text-[var(--muted)]">
                I have written these words on paper and stored them safely.
              </span>
            </label>
            <div className="flex gap-2">
              <button
                onClick={handleStartVerify}
                disabled={!confirmed}
                className="flex-1 py-2.5 rounded-xl text-sm font-medium border transition-colors
                           border-[var(--accent)] text-[var(--accent)] hover:bg-[var(--accent)]/10
                           disabled:opacity-40 disabled:pointer-events-none"
              >
                Verify words
              </button>
              <button
                onClick={handleExport}
                disabled={!confirmed || exporting}
                className="flex-1 bg-[var(--accent)] hover:bg-[var(--accent-hover)] disabled:opacity-40
                           text-white font-medium py-2.5 rounded-xl text-sm transition-colors"
              >
                {exporting ? "Saving…" : "Skip & export"}
              </button>
            </div>
            <button
              onClick={() => { setPhrase(""); setExportStep("idle"); }}
              className="text-xs text-[var(--muted)] hover:text-[var(--text)] self-center transition-colors"
            >
              Cancel
            </button>
          </div>

        ) : (
          /* verifying step */
          <div className="rounded-xl border border-[var(--border)] bg-[var(--surface)] p-4 flex flex-col gap-4">
            <div>
              <div className="font-medium text-sm mb-1">Verify your seed phrase</div>
              <div className="text-xs text-[var(--muted)]">
                Enter words #{verifyIdxs.map(i => i + 1).join(", ")} from your written list.
              </div>
            </div>
            <div className="flex flex-col gap-2">
              {verifyIdxs.map((wordIdx, i) => (
                <div key={wordIdx} className="flex items-center gap-3">
                  <span className="text-xs text-[var(--muted)] w-14 flex-shrink-0 font-mono">
                    Word #{wordIdx + 1}
                  </span>
                  <input
                    type="text"
                    value={verifyInputs[i]}
                    onChange={e => {
                      const next = [...verifyInputs];
                      next[i] = e.target.value;
                      setVerifyInputs(next);
                    }}
                    placeholder="…"
                    className={`flex-1 bg-[var(--bg)] border rounded-lg px-3 py-2 text-sm
                                font-mono text-[var(--text)] placeholder-[var(--muted)]
                                focus:outline-none transition-colors ${
                      verifyInputs[i].trim()
                        ? verifyInputs[i].trim().toLowerCase() === words[wordIdx]
                          ? "border-[var(--success)]"
                          : "border-[var(--danger)]"
                        : "border-[var(--border)] focus:border-[var(--accent)]"
                    }`}
                  />
                </div>
              ))}
            </div>
            <button
              onClick={handleExport}
              disabled={!verifyOk || exporting}
              className="w-full bg-[var(--accent)] hover:bg-[var(--accent-hover)] disabled:opacity-40
                         text-white font-medium py-3 rounded-xl transition-colors"
            >
              {exporting ? "Saving…" : "Verify & export"}
            </button>
            <button
              onClick={() => setExportStep("showPhrase")}
              className="text-xs text-[var(--muted)] hover:text-[var(--text)] self-center transition-colors"
            >
              ← Back
            </button>
          </div>
        )}

        {exportStatus && (
          <div className="mt-2">
            <Alert type={exportStatus.type}>{exportStatus.msg}</Alert>
          </div>
        )}
      </div>

      {/* ── Auto-saved copies ── */}
      {autoBackups.length > 0 && (
        <div>
          <h3 className="font-semibold mb-3">Auto-saved copies</h3>
          <div className="rounded-xl border border-[var(--border)] overflow-hidden divide-y divide-[var(--border)]">
            {autoBackups.map((b, i) => {
              const filename = b.path.replace(/\\/g, "/").split("/").pop() ?? b.path;
              return (
                <div key={i} className="flex items-center gap-3 px-4 py-2.5 bg-[var(--surface)]">
                  <div className="flex-1 min-w-0">
                    <div className="text-xs font-mono text-[var(--text)] truncate">{filename}</div>
                    <div className="text-xs text-[var(--muted)] mt-0.5">{formatSize(b.sizeBytes)}</div>
                  </div>
                  <div className="text-xs text-[var(--muted)] flex-shrink-0">{formatDate(b.createdAt)}</div>
                </div>
              );
            })}
          </div>
          <div className="text-xs text-[var(--muted)] mt-1.5">
            Last 7 copies are kept automatically.
          </div>
        </div>
      )}

      {/* ── Restore ── */}
      <div className="border-t border-[var(--border)] pt-4">
        <h3 className="font-semibold mb-3">Restore from backup</h3>
        <div className="flex flex-col gap-3">
          <p className="text-xs text-[var(--muted)] leading-relaxed">
            The restored vault will be created in the chosen folder.
            Open it from the vault picker afterward.
          </p>
          <div className="flex flex-col gap-1.5">
            <label className="text-xs font-medium text-[var(--muted)] uppercase tracking-wide">
              24-word seed phrase
            </label>
            <textarea
              value={restorePhrase}
              onChange={e => setRestorePhrase(e.target.value)}
              rows={3}
              placeholder="word1 word2 word3 … word24"
              className="w-full bg-[var(--bg)] border border-[var(--border)] rounded-xl px-3 py-2
                         text-sm font-mono text-[var(--text)] placeholder-[var(--muted)]
                         focus:outline-none focus:border-[var(--accent)] transition-colors resize-none"
            />
          </div>
          <div className="flex gap-2">
            <button
              onClick={() => pickBackupFile().then(p => p && setRestoreFile(p)).catch(() => {})}
              className="flex-1 border border-[var(--border)] rounded-xl px-3 py-2 text-sm
                         text-[var(--muted)] hover:text-[var(--text)] hover:border-[var(--accent)]
                         transition-colors text-left truncate"
            >
              {restoreFile
                ? `📄 ${restoreFile.replace(/\\/g, "/").split("/").pop()}`
                : "📄 Choose .vbk file…"}
            </button>
            <button
              onClick={() => pickFolder().then(p => p && setRestoreDestDir(p)).catch(() => {})}
              className="flex-1 border border-[var(--border)] rounded-xl px-3 py-2 text-sm
                         text-[var(--muted)] hover:text-[var(--text)] hover:border-[var(--accent)]
                         transition-colors text-left truncate"
            >
              {restoreDestDir
                ? `📁 ${restoreDestDir.replace(/\\/g, "/").split("/").pop()}`
                : "📁 Choose destination…"}
            </button>
          </div>
          <button
            onClick={handleRestore}
            disabled={
              restoring ||
              !restoreFile ||
              !restoreDestDir ||
              restorePhrase.trim().split(/\s+/).length !== 24
            }
            className="w-full bg-amber-700/80 hover:bg-amber-700 disabled:opacity-40
                       text-white font-medium py-3 rounded-xl transition-colors"
          >
            {restoring ? "Restoring…" : "Restore vault"}
          </button>
          {restoreStatus && <Alert type={restoreStatus.type}>{restoreStatus.msg}</Alert>}
        </div>
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

      {/* ── Extension Installer ── */}
      <div>
        <h3 className="font-semibold mb-1 text-sm">Install browser extension</h3>
        <p className="text-xs text-[var(--muted)] mb-3 leading-relaxed">
          Installs the LSPV extension directly into your browsers without visiting the store.
          Choose which browsers and Firefox profiles to install to.
        </p>
        <ExtensionInstaller />
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

// ── Data Tab ──────────────────────────────────────────────────────────────────

function DataTab({ onImported }: { onImported?: () => void }) {
  const fileRef    = useRef<HTMLInputElement>(null);
  const bwFileRef  = useRef<HTMLInputElement>(null);
  const [trashItems, setTrashItems]   = useState<ItemSummary[]>([]);
  const [health,     setHealth]       = useState<HealthEntry[] | null>(null);
  const [status,     setStatus]       = useState<{ type: "success" | "error"; msg: string } | null>(null);
  const [healthLoading,  setHealthLoading]  = useState(false);
  const [exportLoading,  setExportLoading]  = useState(false);
  const [refreshKey, setRefreshKey]   = useState(0);

  // Import state
  const [rows,          setRows]          = useState<ImportRow[]>([]);
  const [parseError,    setParseError]    = useState("");
  const [importing,     setImporting]     = useState(false);
  const [importedCount, setImportedCount] = useState<number | null>(null);
  const [profiles,      setProfiles]      = useState<ProfileInfo[]>([]);
  const [sourceTag,     setSourceTag]     = useState<string>("");
  const [retag,         setRetag]         = useState<{ old: string; new: string } | null>(null);

  // Bitwarden JSON import state
  const [bwImporting,     setBwImporting]     = useState(false);
  const [bwImportedCount, setBwImportedCount] = useState<number | null>(null);
  const [bwError,         setBwError]         = useState("");
  const [bwTag,           setBwTag]           = useState("");

  useEffect(() => {
    listDeletedItems()
      .then(trash => setTrashItems(trash))
      .catch(err => setStatus({ type: "error", msg: String(err) }));
    getProfiles().then(setProfiles).catch(() => {});
  }, [refreshKey]);

  const refresh = () => setRefreshKey(k => k + 1);

  async function handleRestore(id: string) {
    try {
      await restoreItem(id);
      refresh();
      setStatus({ type: "success", msg: "Item restored." });
    } catch (err) { setStatus({ type: "error", msg: String(err) }); }
  }

  async function handlePurge(id: string, title: string) {
    if (!confirm(`Permanently delete "${title}"? This cannot be undone.`)) return;
    try {
      await purgeItem(id);
      refresh();
      setStatus({ type: "success", msg: "Item permanently deleted." });
    } catch (err) { setStatus({ type: "error", msg: String(err) }); }
  }

  async function handleEmptyTrash() {
    if (!confirm(`Permanently delete ALL ${trashItems.length} item(s) in trash? This cannot be undone.`)) return;
    try {
      const n = await purgeAllTrash();
      refresh();
      setStatus({ type: "success", msg: `Deleted ${n} item(s).` });
    } catch (err) { setStatus({ type: "error", msg: String(err) }); }
  }

  async function handleHealthReport() {
    setHealthLoading(true);
    setHealth(null);
    try {
      const entries = await getHealthReport();
      setHealth(entries);
    } catch (err) { setStatus({ type: "error", msg: String(err) }); }
    finally { setHealthLoading(false); }
  }

  async function handleExportCsv() {
    setExportLoading(true);
    try {
      const path = await pickCsvSavePath();
      if (!path) return;
      await exportItemsCsv(path);
      setStatus({ type: "success", msg: `Exported to ${path}` });
    } catch (err) { setStatus({ type: "error", msg: String(err) }); }
    finally { setExportLoading(false); }
  }

  async function handleFile(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file) return;
    setParseError(""); setRows([]); setImportedCount(null);
    try {
      const text = await file.text();
      const parsed = await parseImportCsv(text);
      setRows(parsed);
    } catch (err) { setParseError(String(err)); }
    e.target.value = "";
  }

  async function handleImport() {
    setImporting(true); setParseError("");
    try {
      const tag = sourceTag.trim() || null;
      const count = await importItemsFromCsv(rows, tag);
      setImportedCount(count);
      setRows([]);
      onImported?.();
    } catch (err) { setParseError(String(err)); }
    finally { setImporting(false); }
  }

  async function handleRetag() {
    if (!retag || !retag.old.trim()) return;
    try {
      const n = await bulkRetagItems(retag.old.trim(), retag.new.trim() || null);
      setRetag(null);
      onImported?.();
      setStatus({ type: "success", msg: `Updated ${n} item${n !== 1 ? "s" : ""}.` });
    } catch (err) { setStatus({ type: "error", msg: String(err) }); }
  }

  async function handleBwFile(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file) return;
    setBwError(""); setBwImportedCount(null);
    setBwImporting(true);
    try {
      const text = await file.text();
      const count = await importBitwardenJson(text, bwTag.trim() || undefined);
      setBwImportedCount(count);
      onImported?.();
    } catch (err) { setBwError(String(err)); }
    finally { setBwImporting(false); e.target.value = ""; }
  }

  const S = "flex flex-col gap-3 p-4 border border-[var(--border)] rounded-xl bg-[var(--surface)]";
  const H = "text-sm font-semibold text-[var(--text)]";

  return (
    <div className="p-4 flex flex-col gap-6 max-w-lg w-full">
      {status && (
        <div className={`text-sm px-3 py-2 rounded-lg border ${
          status.type === "success"
            ? "text-green-400 bg-green-950/30 border-green-900/40"
            : "text-[var(--danger)] bg-red-950/30 border-red-900/40"
        }`}>
          {status.msg}
          <button onClick={() => setStatus(null)} className="ml-2 text-[var(--muted)] hover:text-[var(--text)]">×</button>
        </div>
      )}

      {/* Bitwarden JSON import */}
      <div className={S}>
        <div className={H}>🦊 Import from Bitwarden</div>
        <p className="text-xs text-[var(--muted)] leading-relaxed">
          Accepts Bitwarden JSON exports ({" "}
          <span className="font-mono">Settings → Export vault → .json (unencrypted)</span>).
          Imports logins, secure notes, cards, and identities.
        </p>
        <input
          type="text"
          value={bwTag}
          onChange={e => setBwTag(e.target.value)}
          placeholder="Tag for these imports (optional)…"
          className="w-full bg-[var(--bg)] border border-[var(--border)] rounded-lg px-3 py-2
                     text-sm text-[var(--text)] placeholder-[var(--muted)]
                     focus:outline-none focus:border-[var(--accent)] transition-colors"
        />
        <input ref={bwFileRef} type="file" accept=".json,application/json" onChange={handleBwFile} className="hidden" />
        <button
          onClick={() => bwFileRef.current?.click()}
          disabled={bwImporting}
          className="w-full border-2 border-dashed border-[var(--border)] hover:border-[var(--accent)]
                     rounded-xl py-5 text-sm text-[var(--muted)] hover:text-[var(--text)]
                     disabled:opacity-50 transition-colors"
        >
          {bwImporting ? "Importing…" : "📂 Choose Bitwarden JSON file…"}
        </button>
        {bwError && <Alert type="error">{bwError}</Alert>}
        {bwImportedCount !== null && (
          <Alert type="success">✓ Imported {bwImportedCount} item{bwImportedCount !== 1 ? "s" : ""} from Bitwarden</Alert>
        )}
      </div>

      {/* Import CSV */}
      <div className={S}>
        <div className={H}>📥 Import passwords (CSV)</div>
        <p className="text-xs text-[var(--muted)] leading-relaxed">
          Accepts CSV exports from Chrome
          {" "}(<span className="font-mono">chrome://password-manager → Settings → Export</span>)
          {" "}and Firefox
          {" "}(<span className="font-mono">about:logins → ··· → Export Logins…</span>).
        </p>

        <input ref={fileRef} type="file" accept=".csv,text/csv" onChange={handleFile} className="hidden" />
        <button
          onClick={() => fileRef.current?.click()}
          className="w-full border-2 border-dashed border-[var(--border)] hover:border-[var(--accent)]
                     rounded-xl py-6 text-sm text-[var(--muted)] hover:text-[var(--text)] transition-colors"
        >
          📂 Choose CSV file…
        </button>

        {parseError && <Alert type="error">{parseError}</Alert>}

        {importedCount !== null && (
          <Alert type="success">✓ Imported {importedCount} item{importedCount !== 1 ? "s" : ""}</Alert>
        )}

        {rows.length > 0 && (
          <div className="flex flex-col gap-3">
            <div className="text-xs text-[var(--muted)]">{rows.length} items ready to import:</div>
            <div className="border border-[var(--border)] rounded-xl overflow-hidden max-h-48 overflow-y-auto">
              <table className="w-full text-xs">
                <thead className="sticky top-0 bg-[var(--surface)] border-b border-[var(--border)]">
                  <tr>
                    <th className="text-left px-3 py-2 text-[var(--muted)] font-medium">Title</th>
                    <th className="text-left px-3 py-2 text-[var(--muted)] font-medium">URL</th>
                    <th className="text-left px-3 py-2 text-[var(--muted)] font-medium">User</th>
                  </tr>
                </thead>
                <tbody>
                  {rows.map((r, i) => (
                    <tr key={i} className="border-b border-[var(--border)]/40 last:border-0">
                      <td className="px-3 py-1.5 text-[var(--text)] truncate max-w-[120px]">{r.title}</td>
                      <td className="px-3 py-1.5 text-[var(--muted)] truncate max-w-[120px]" title={r.url}>{r.url}</td>
                      <td className="px-3 py-1.5 text-[var(--muted)] truncate max-w-[80px]">{r.username}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>

            <div className="flex flex-col gap-1.5">
              <label className="text-xs font-medium text-[var(--muted)] uppercase tracking-wide">
                Tag these imports (optional)
              </label>
              {profiles.length > 0 && (
                <div className="flex flex-wrap gap-1.5">
                  {profiles.map(p => {
                    const label = p.name || p.email || p.id.slice(0, 8);
                    return (
                      <button
                        key={p.id}
                        type="button"
                        onClick={() => setSourceTag(t => t === label ? "" : label)}
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
                         text-white font-medium py-2.5 rounded-xl transition-colors"
            >
              {importing ? "Importing…" : `Import ${rows.length} item${rows.length !== 1 ? "s" : ""}${sourceTag.trim() ? ` → "${sourceTag.trim()}"` : ""}`}
            </button>
          </div>
        )}

        {/* Bulk retag */}
        <div className="border-t border-[var(--border)] pt-3 flex flex-col gap-2">
          <div className="text-xs font-medium text-[var(--muted)] uppercase tracking-wide">Bulk re-tag</div>
          {retag === null ? (
            <button
              type="button"
              onClick={() => setRetag({ old: "", new: "" })}
              className="text-xs text-[var(--accent)] hover:underline self-start"
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
                placeholder="New tag (leave empty to clear)"
                className="w-full bg-[var(--bg)] border border-[var(--border)] rounded-lg px-3 py-2
                           text-sm text-[var(--text)] placeholder-[var(--muted)]
                           focus:outline-none focus:border-[var(--accent)] transition-colors"
              />
              <div className="flex gap-2">
                <button type="button" onClick={() => setRetag(null)}
                  className="flex-1 border border-[var(--border)] text-[var(--muted)]
                             hover:text-[var(--text)] py-2 rounded-xl text-sm transition-colors"
                >Cancel</button>
                <button type="button" onClick={handleRetag} disabled={!retag.old.trim()}
                  className="flex-1 bg-[var(--accent)] hover:bg-[var(--accent-hover)] disabled:opacity-40
                             text-white font-medium py-2 rounded-xl text-sm transition-colors"
                >Apply</button>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Export CSV */}
      <div className={S}>
        <div className={H}>📤 Export passwords</div>
        <p className="text-xs text-[var(--muted)]">
          Export all Login entries as a CSV file compatible with Chrome, Firefox, and Bitwarden.
        </p>
        <button
          onClick={handleExportCsv}
          disabled={exportLoading}
          className="self-start px-4 py-2 border border-[var(--border)] text-sm text-[var(--text)]
                     hover:border-[var(--accent)] hover:text-[var(--accent)] rounded-lg transition-colors
                     disabled:opacity-50"
        >
          {exportLoading ? "Saving…" : "Export as CSV"}
        </button>
      </div>

      {/* Trash */}
      <div className={S}>
        <div className="flex items-center justify-between">
          <div className={H}>🗑 Trash ({trashItems.length})</div>
          {trashItems.length > 0 && (
            <button onClick={handleEmptyTrash}
              className="text-xs text-[var(--danger)] hover:text-red-400 transition-colors"
            >Empty trash</button>
          )}
        </div>
        <p className="text-xs text-[var(--muted)]">
          Items in trash are automatically purged after 30 days.
          Use the main vault list to bulk-select and move items here.
        </p>
        {trashItems.length === 0 ? (
          <div className="text-xs text-[var(--muted)]">Trash is empty.</div>
        ) : (
          <div className="flex flex-col divide-y divide-[var(--border)] border border-[var(--border)] rounded-lg overflow-hidden">
            {trashItems.map(item => (
              <div key={item.id} className="flex items-center gap-2 px-3 py-2 bg-[var(--bg)]">
                <div className="flex-1 min-w-0">
                  <div className="text-sm text-[var(--text)] truncate">{item.title}</div>
                  {item.subtitle && (
                    <div className="text-[10px] text-[var(--muted)] truncate">{item.subtitle}</div>
                  )}
                  <div className="text-[10px] text-[var(--muted)]">
                    {item.itemType} · deleted {new Date(item.updatedAt * 1000).toLocaleDateString()}
                  </div>
                </div>
                <button onClick={() => handleRestore(item.id)}
                  className="text-xs text-[var(--accent)] hover:text-[var(--accent-hover)] transition-colors flex-shrink-0"
                >Restore</button>
                <button onClick={() => handlePurge(item.id, item.title)}
                  className="text-xs text-[var(--muted)] hover:text-[var(--danger)] transition-colors flex-shrink-0"
                >Delete</button>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Password health */}
      <div className={S}>
        <div className={H}>🔍 Password Health</div>
        <p className="text-xs text-[var(--muted)]">
          Analyse login passwords for weaknesses, duplicates, and entries not updated in 6+ months.
        </p>
        <button
          onClick={handleHealthReport}
          disabled={healthLoading}
          className="w-full px-4 py-2.5 bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white text-sm
                     font-medium rounded-lg transition-colors disabled:opacity-50"
        >
          {healthLoading ? "Analysing…" : "Run health check"}
        </button>
        {health !== null && (
          health.length === 0 ? (
            <div className="text-sm text-green-400">✓ All passwords look healthy!</div>
          ) : (
            <div className="flex flex-col divide-y divide-[var(--border)] border border-[var(--border)] rounded-lg overflow-hidden mt-1">
              {health.map(e => (
                <div key={e.id} className="px-3 py-2 bg-[var(--bg)] flex flex-col gap-0.5">
                  <div className="text-sm text-[var(--text)] font-medium truncate">{e.title}</div>
                  {e.url && <div className="text-[10px] text-[var(--muted)] truncate">{e.url}</div>}
                  <div className="flex flex-wrap gap-1 mt-0.5">
                    {e.isWeak && (
                      <span className="text-[10px] px-1.5 py-0.5 rounded bg-red-900/40 text-red-400 border border-red-900/40">Weak</span>
                    )}
                    {e.isDuplicate && (
                      <span className="text-[10px] px-1.5 py-0.5 rounded bg-amber-900/40 text-amber-400 border border-amber-900/40">Duplicate</span>
                    )}
                    {e.isOld && (
                      <span className="text-[10px] px-1.5 py-0.5 rounded bg-[var(--border)] text-[var(--muted)] border border-[var(--border)]">Old (&gt;6mo)</span>
                    )}
                  </div>
                </div>
              ))}
            </div>
          )
        )}
      </div>
    </div>
  );
}

// ── About Tab ─────────────────────────────────────────────────────────────────

const BTC_ADDRESS = "bc1qwnkyez3nv86dry54dqfjjtav29qqq72h69pevw";
const CONTACT_EMAIL = "ssss2883866@gmail.com";

function AboutTab() {
  const [copied,    setCopied]    = useState(false);
  const [copiedBtc, setCopiedBtc] = useState(false);
  const [copiedMail, setCopiedMail] = useState(false);
  const GITHUB = "https://github.com/RakinSV/local-security-pass-vault";

  async function handleGithub() {
    try {
      await openGithub();
    } catch {
      await navigator.clipboard.writeText(GITHUB).catch(() => {});
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  }

  async function copyBtc() {
    await navigator.clipboard.writeText(BTC_ADDRESS).catch(() => {});
    setCopiedBtc(true);
    setTimeout(() => setCopiedBtc(false), 2000);
  }

  async function copyMail() {
    await navigator.clipboard.writeText(CONTACT_EMAIL).catch(() => {});
    setCopiedMail(true);
    setTimeout(() => setCopiedMail(false), 2000);
  }

  return (
    <div className="p-6 max-w-md mx-auto w-full flex flex-col gap-6">
      <div className="flex items-center gap-4">
        <div className="text-5xl">🔐</div>
        <div>
          <h2 className="font-bold text-base text-[var(--text)] leading-tight">
            Local Security Pass Vault
          </h2>
          <div className="text-xs text-[var(--muted)] mt-0.5">Version 0.2.3</div>
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

      {/* Source code */}
      <div className="flex flex-col gap-2">
        <div className="text-xs font-medium text-[var(--muted)] uppercase tracking-wider">Source code</div>
        <button
          onClick={handleGithub}
          className="flex items-center gap-3 rounded-xl border border-[var(--border)] bg-[var(--surface)]
                     hover:border-[var(--accent)] px-4 py-3 transition-colors text-left group w-full"
        >
          <span className="text-lg">⌥</span>
          <div className="flex-1 min-w-0">
            <div className="text-sm font-medium text-[var(--text)] group-hover:text-[var(--accent)] transition-colors">
              {copied ? "URL copied!" : "Open on GitHub"}
            </div>
            <div className="text-xs text-[var(--muted)] font-mono mt-0.5 truncate">{GITHUB}</div>
          </div>
          <span className="text-[var(--muted)] text-xs">↗</span>
        </button>
      </div>

      {/* Contact */}
      <div className="flex flex-col gap-2">
        <div className="text-xs font-medium text-[var(--muted)] uppercase tracking-wider">Contact</div>
        <button
          onClick={copyMail}
          className="flex items-center gap-3 rounded-xl border border-[var(--border)] bg-[var(--surface)]
                     hover:border-[var(--accent)] px-4 py-3 transition-colors text-left group w-full"
        >
          <span className="text-lg">✉️</span>
          <div className="flex-1 min-w-0">
            <div className="text-sm font-medium text-[var(--text)] group-hover:text-[var(--accent)] transition-colors">
              {copiedMail ? "Copied!" : "Send an email"}
            </div>
            <div className="text-xs text-[var(--muted)] font-mono mt-0.5 truncate">{CONTACT_EMAIL}</div>
          </div>
          <span className="text-[10px] text-[var(--muted)]">copy</span>
        </button>
      </div>

      {/* Bitcoin donation */}
      <div className="flex flex-col gap-2">
        <div className="text-xs font-medium text-[var(--muted)] uppercase tracking-wider">Support the project</div>
        <div className="rounded-xl border border-[var(--border)] bg-[var(--surface)] p-4 flex flex-col gap-3">
          <p className="text-xs text-[var(--muted)] leading-relaxed">
            If LSPV saves you time or keeps your data safe, consider a Bitcoin donation —
            it helps keep the project alive and open-source.
          </p>
          <button
            onClick={copyBtc}
            className="flex items-center gap-3 bg-[var(--bg)] border border-[var(--border)]
                       hover:border-amber-600/60 rounded-xl px-4 py-3 transition-colors
                       text-left group w-full"
          >
            <span className="text-xl flex-shrink-0">₿</span>
            <div className="flex-1 min-w-0">
              <div className="text-[10px] text-[var(--muted)] uppercase tracking-wide mb-0.5">Bitcoin (BTC)</div>
              <div className="text-xs font-mono text-[var(--text)] break-all leading-relaxed">
                {BTC_ADDRESS}
              </div>
            </div>
            <span className={`text-xs flex-shrink-0 transition-colors ml-2 ${
              copiedBtc ? "text-amber-400" : "text-[var(--muted)] group-hover:text-amber-400"
            }`}>
              {copiedBtc ? "✓ copied" : "copy"}
            </span>
          </button>
        </div>
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
