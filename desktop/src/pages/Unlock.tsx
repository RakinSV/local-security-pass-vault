import { useState } from "react";
import { PasswordField } from "../components/PasswordField";
import { openVault, getDefaultVaultDir } from "../api/vault";

interface Props {
  onUnlocked: () => void;
  onBack: () => void;
}

export function Unlock({ onUnlocked, onBack }: Props) {
  const [password, setPassword] = useState("");
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(false);

  async function handleUnlock(e: React.FormEvent) {
    e.preventDefault();
    setError("");
    setLoading(true);
    try {
      const dir = await getDefaultVaultDir();
      await openVault(dir, password);
      onUnlocked();
    } catch (err) {
      const msg = String(err).toLowerCase();
      if (msg.includes("decryption")) {
        setError("Wrong password.");
      } else if (msg.includes("not found")) {
        setError("Vault not found. Create a new vault first.");
      } else {
        setError(String(err));
      }
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

        <div className="text-3xl mb-3">🔐</div>
        <h2 className="text-xl font-bold mb-1">Unlock VaultPass</h2>
        <p className="text-[var(--muted)] text-sm mb-6">
          Enter your master password to continue.
        </p>

        <form onSubmit={handleUnlock} className="flex flex-col gap-4">
          <PasswordField
            label="Master password"
            value={password}
            onChange={setPassword}
            placeholder="Enter your master password"
            autoFocus
          />

          {error && (
            <div className="text-[var(--danger)] text-sm bg-red-950/30 border border-red-900/40
                            rounded-lg px-3 py-2">
              {error}
            </div>
          )}

          <button
            type="submit"
            disabled={loading || !password}
            className="w-full bg-[var(--accent)] hover:bg-[var(--accent-hover)] disabled:opacity-40
                       text-white font-medium py-3 rounded-xl transition-colors mt-2"
          >
            {loading ? "Unlocking…" : "Unlock"}
          </button>
        </form>
      </div>
    </div>
  );
}
