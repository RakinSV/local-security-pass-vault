import { useState, useEffect } from "react";
import { PasswordField } from "../components/PasswordField";
import { createVault, suggestVaultDir, pickFolder } from "../api/vault";
import { registerVault } from "../lib/vaultRegistry";

interface Props {
  onCreated: () => void;
  onBack: () => void;
}

export function Setup({ onCreated, onBack }: Props) {
  const [vaultName, setVaultName] = useState("Personal");
  const [vaultPath, setVaultPath] = useState("");
  const [password, setPassword]   = useState("");
  const [confirm, setConfirm]     = useState("");
  const [hint, setHint]           = useState("");
  const [error, setError]         = useState("");
  const [loading, setLoading]     = useState(false);

  useEffect(() => {
    if (!vaultName.trim()) { setVaultPath(""); return; }
    suggestVaultDir(vaultName.trim()).then(setVaultPath).catch(() => {});
  }, [vaultName]);

  async function handleBrowse() {
    const picked = await pickFolder();
    if (picked) setVaultPath(picked);
  }

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault();
    setError("");
    if (!vaultName.trim()) { setError("Enter a vault name."); return; }
    if (!vaultPath.trim()) { setError("Choose a folder path."); return; }
    if (password.length < 8) { setError("Password must be at least 8 characters."); return; }
    if (password !== confirm) { setError("Passwords do not match."); return; }

    setLoading(true);
    try {
      await createVault(vaultPath.trim(), password, hint || undefined);
      registerVault(vaultName.trim(), vaultPath.trim());
      onCreated();
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="flex flex-col items-center justify-center h-screen px-6 bg-[var(--bg)]">
      <div className="w-full max-w-sm">
        <button onClick={onBack}
          className="text-[var(--muted)] hover:text-[var(--text)] text-sm mb-6 transition-colors">
          ← Back
        </button>

        <h2 className="text-xl font-bold mb-1">Create new vault</h2>
        <p className="text-[var(--muted)] text-sm mb-6">
          Choose a name, location, and strong master password.
        </p>

        <form onSubmit={handleCreate} className="flex flex-col gap-4">
          <div className="flex flex-col gap-1">
            <label className="text-xs font-medium text-[var(--muted)] uppercase tracking-wide">
              Vault name
            </label>
            <input
              type="text"
              value={vaultName}
              onChange={e => setVaultName(e.target.value)}
              placeholder="Personal, Work, Family…"
              autoFocus
              className="w-full bg-[var(--bg)] border border-[var(--border)] rounded-lg px-3 py-2
                         text-[var(--text)] placeholder-[var(--muted)] text-sm
                         focus:outline-none focus:border-[var(--accent)] transition-colors"
            />
          </div>

          <div className="flex flex-col gap-1">
            <label className="text-xs font-medium text-[var(--muted)] uppercase tracking-wide">
              Storage location
            </label>
            <div className="flex gap-2">
              <input
                type="text"
                value={vaultPath}
                onChange={e => setVaultPath(e.target.value)}
                placeholder="Vault folder path…"
                className="flex-1 bg-[var(--bg)] border border-[var(--border)] rounded-lg px-3 py-2
                           text-[var(--text)] placeholder-[var(--muted)] text-sm font-mono
                           focus:outline-none focus:border-[var(--accent)] transition-colors"
              />
              <button type="button" onClick={handleBrowse}
                className="px-3 py-2 bg-[var(--surface)] border border-[var(--border)] rounded-lg text-sm
                           hover:border-[var(--accent)] hover:text-[var(--accent)] transition-colors flex-shrink-0">
                Browse…
              </button>
            </div>
          </div>

          <PasswordField label="Master password"    value={password} onChange={setPassword} placeholder="At least 8 characters" />
          <PasswordField label="Confirm password"   value={confirm}  onChange={setConfirm}  placeholder="Repeat password" />

          <div className="flex flex-col gap-1">
            <label className="text-xs font-medium text-[var(--muted)] uppercase tracking-wide">
              Password hint (optional)
            </label>
            <input
              type="text"
              value={hint}
              onChange={e => setHint(e.target.value)}
              placeholder="A hint to help you remember"
              maxLength={120}
              className="w-full bg-[var(--bg)] border border-[var(--border)] rounded-lg px-3 py-2
                         text-[var(--text)] placeholder-[var(--muted)] text-sm
                         focus:outline-none focus:border-[var(--accent)] transition-colors"
            />
          </div>

          {error && (
            <div className="text-[var(--danger)] text-sm bg-red-950/30 border border-red-900/40 rounded-lg px-3 py-2">
              {error}
            </div>
          )}

          <button type="submit"
            disabled={loading || !vaultName.trim() || !vaultPath.trim() || !password || !confirm}
            className="w-full bg-[var(--accent)] hover:bg-[var(--accent-hover)] disabled:opacity-40
                       text-white font-medium py-3 rounded-xl transition-colors mt-2">
            {loading ? "Creating…" : "Create vault"}
          </button>
        </form>
      </div>
    </div>
  );
}
