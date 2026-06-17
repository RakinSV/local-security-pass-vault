import { useState } from "react";
import { PasswordField } from "../components/PasswordField";
import { createVault, getDefaultVaultDir } from "../api/vault";

interface Props {
  onCreated: () => void;
  onBack: () => void;
}

export function Setup({ onCreated, onBack }: Props) {
  const [password, setPassword] = useState("");
  const [confirm, setConfirm] = useState("");
  const [hint, setHint] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault();
    setError("");

    if (password.length < 8) {
      setError("Password must be at least 8 characters.");
      return;
    }
    if (password !== confirm) {
      setError("Passwords do not match.");
      return;
    }

    setLoading(true);
    try {
      const dir = await getDefaultVaultDir();
      await createVault(dir, password, hint || undefined);
      onCreated();
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="flex flex-col items-center justify-center h-screen px-6">
      <div className="w-full max-w-sm">
        <button
          onClick={onBack}
          className="text-[var(--muted)] hover:text-[var(--text)] text-sm mb-6 transition-colors"
        >
          ← Back
        </button>

        <h2 className="text-xl font-bold mb-1">Create vault</h2>
        <p className="text-[var(--muted)] text-sm mb-6">
          Choose a strong master password. It cannot be recovered.
        </p>

        <form onSubmit={handleCreate} className="flex flex-col gap-4">
          <PasswordField
            label="Master password"
            value={password}
            onChange={setPassword}
            placeholder="At least 8 characters"
            autoFocus
          />
          <PasswordField
            label="Confirm password"
            value={confirm}
            onChange={setConfirm}
            placeholder="Repeat password"
          />

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
            <div className="text-[var(--danger)] text-sm bg-red-950/30 border border-red-900/40
                            rounded-lg px-3 py-2">
              {error}
            </div>
          )}

          <button
            type="submit"
            disabled={loading || !password || !confirm}
            className="w-full bg-[var(--accent)] hover:bg-[var(--accent-hover)] disabled:opacity-40
                       text-white font-medium py-3 rounded-xl transition-colors mt-2"
          >
            {loading ? "Creating…" : "Create vault"}
          </button>
        </form>
      </div>
    </div>
  );
}
