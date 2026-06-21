import { useState, useEffect } from "react";
import { PasswordField } from "../components/PasswordField";
import { openVault, vaultRequires2fa } from "../api/vault";
import { touchVault } from "../lib/vaultRegistry";
import type { VaultEntry } from "../lib/vaultRegistry";

interface Props {
  entry: VaultEntry;
  onUnlocked: () => void;
  onBack: () => void;
}

export function Unlock({ entry, onUnlocked, onBack }: Props) {
  const [password, setPassword]     = useState("");
  const [totpCode, setTotpCode]     = useState("");
  const [needs2fa, setNeeds2fa]     = useState(false);
  const [error, setError]           = useState("");
  const [totpError, setTotpError]   = useState("");
  const [loading, setLoading]       = useState(false);

  useEffect(() => {
    vaultRequires2fa(entry.path)
      .then(setNeeds2fa)
      .catch(() => setNeeds2fa(false));
  }, [entry.path]);

  async function handleUnlock(e: React.FormEvent) {
    e.preventDefault();
    setError("");
    setTotpError("");
    setLoading(true);
    try {
      await openVault(entry.path, password, needs2fa ? totpCode : undefined);
      touchVault(entry.id);
      onUnlocked();
    } catch (err) {
      const msg = String(err).toLowerCase();
      if (msg.includes("two-factor code incorrect")) {
        setTotpError("Invalid authenticator code. Please try again.");
      } else if (msg.includes("two-factor required")) {
        setNeeds2fa(true);
        setTotpError("This vault requires a 2FA code.");
      } else if (msg.includes("decryption")) {
        setError("Wrong password.");
      } else if (msg.includes("not found")) {
        setError("Vault not found at this path. The folder may have been moved or deleted.");
      } else {
        setError(String(err));
      }
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="flex flex-col items-center justify-center h-screen px-6 bg-[var(--bg)]">
      <div className="w-full max-w-sm">
        <button onClick={onBack}
          className="text-[var(--muted)] hover:text-[var(--text)] text-sm mb-6 transition-colors">
          ← All vaults
        </button>

        <div className="text-3xl mb-3">🔒</div>
        <h2 className="text-xl font-bold mb-0.5">{entry.name}</h2>
        <p className="text-[var(--muted)] text-xs font-mono mb-1 truncate">{entry.path}</p>
        <p className="text-[var(--muted)] text-sm mb-6">
          Enter your master password to unlock.
        </p>

        <form onSubmit={handleUnlock} className="flex flex-col gap-4">
          <PasswordField
            label="Master password"
            value={password}
            onChange={setPassword}
            placeholder="Enter your master password"
            autoFocus
          />

          {needs2fa && (
            <div className="flex flex-col gap-1">
              <label className="text-sm font-medium text-[var(--text)]">
                Authenticator code
              </label>
              <input
                type="text"
                inputMode="numeric"
                pattern="[0-9]*"
                maxLength={6}
                value={totpCode}
                onChange={e => setTotpCode(e.target.value.replace(/\D/g, ""))}
                placeholder="6-digit code"
                className="w-full px-3 py-2 rounded-lg border border-[var(--border)]
                           bg-[var(--input-bg)] text-[var(--text)] text-center text-xl
                           tracking-[0.4em] focus:outline-none focus:ring-2 focus:ring-[var(--accent)]"
                autoComplete="one-time-code"
              />
              {totpError && (
                <p className="text-[var(--danger)] text-xs">{totpError}</p>
              )}
            </div>
          )}

          {error && (
            <div className="text-[var(--danger)] text-sm bg-red-950/30 border border-red-900/40 rounded-lg px-3 py-2">
              {error}
            </div>
          )}

          <button type="submit"
            disabled={loading || !password || (needs2fa && totpCode.length < 6)}
            className="w-full bg-[var(--accent)] hover:bg-[var(--accent-hover)] disabled:opacity-40
                       text-white font-medium py-3 rounded-xl transition-colors mt-2">
            {loading ? "Unlocking…" : "Unlock"}
          </button>
        </form>
      </div>
    </div>
  );
}
