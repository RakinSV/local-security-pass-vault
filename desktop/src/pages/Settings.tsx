import { useState } from "react";
import { PasswordField } from "../components/PasswordField";
import { changeMasterPassword } from "../api/vault";

interface Props {
  onBack: () => void;
}

export function Settings({ onBack }: Props) {
  const [oldPw, setOldPw] = useState("");
  const [newPw, setNewPw] = useState("");
  const [confirmPw, setConfirmPw] = useState("");
  const [error, setError] = useState("");
  const [success, setSuccess] = useState(false);
  const [loading, setLoading] = useState(false);

  async function handleChange(e: React.FormEvent) {
    e.preventDefault();
    setError("");
    setSuccess(false);

    if (newPw.length < 8) { setError("New password must be at least 8 characters."); return; }
    if (newPw !== confirmPw) { setError("Passwords do not match."); return; }

    setLoading(true);
    try {
      await changeMasterPassword(oldPw, newPw);
      setSuccess(true);
      setOldPw(""); setNewPw(""); setConfirmPw("");
    } catch (err) {
      const msg = String(err);
      if (msg.includes("DecryptionFailed") || msg.includes("decryption")) {
        setError("Current password is incorrect.");
      } else {
        setError(msg);
      }
    } finally {
      setLoading(false);
    }
  }

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

      <div className="flex-1 overflow-y-auto p-6 max-w-md mx-auto w-full">
        <h3 className="font-semibold mb-4">Change master password</h3>

        <form onSubmit={handleChange} className="flex flex-col gap-4">
          <PasswordField label="Current password" value={oldPw} onChange={setOldPw} autoFocus />
          <PasswordField label="New password" value={newPw} onChange={setNewPw} placeholder="At least 8 characters" />
          <PasswordField label="Confirm new password" value={confirmPw} onChange={setConfirmPw} />

          {error && (
            <div className="text-[var(--danger)] text-sm bg-red-950/30 border border-red-900/40 rounded-lg px-3 py-2">
              {error}
            </div>
          )}
          {success && (
            <div className="text-[var(--success)] text-sm bg-green-950/30 border border-green-900/40 rounded-lg px-3 py-2">
              Password changed successfully.
            </div>
          )}

          <button
            type="submit"
            disabled={loading || !oldPw || !newPw || !confirmPw}
            className="w-full bg-[var(--accent)] hover:bg-[var(--accent-hover)] disabled:opacity-40
                       text-white font-medium py-3 rounded-xl transition-colors"
          >
            {loading ? "Changing…" : "Change password"}
          </button>
        </form>
      </div>
    </div>
  );
}
