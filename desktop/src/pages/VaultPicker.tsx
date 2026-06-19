import { useState, useEffect } from "react";
import { sortedVaults, removeVault, registerVault } from "../lib/vaultRegistry";
import type { VaultEntry } from "../lib/vaultRegistry";
import { pickFolder } from "../api/vault";

interface Props {
  onCreateVault: () => void;
  onOpenVault: (entry: VaultEntry) => void;
}

function formatDate(ts?: number): string {
  if (!ts) return "Never";
  const d = new Date(ts);
  return d.toLocaleDateString(undefined, { day: "2-digit", month: "short", year: "numeric" });
}

export function VaultPicker({ onCreateVault, onOpenVault }: Props) {
  const [vaults, setVaults] = useState<VaultEntry[]>([]);
  const [adding, setAdding] = useState(false);
  const [addName, setAddName] = useState("");
  const [addPath, setAddPath] = useState("");
  const [addError, setAddError] = useState("");

  useEffect(() => {
    setVaults(sortedVaults());
  }, []);

  function handleRemove(id: string, e: React.MouseEvent) {
    e.stopPropagation();
    if (!confirm("Remove vault from list?\n\nThe vault files will NOT be deleted — only removed from this list.")) return;
    removeVault(id);
    setVaults(sortedVaults());
  }

  async function handleBrowse() {
    const path = await pickFolder();
    if (path) setAddPath(path);
  }

  function handleAdd() {
    setAddError("");
    if (!addName.trim()) { setAddError("Enter a vault name."); return; }
    if (!addPath.trim()) { setAddError("Choose the vault folder path."); return; }
    registerVault(addName.trim(), addPath.trim());
    setVaults(sortedVaults());
    setAdding(false);
    setAddName("");
    setAddPath("");
  }

  return (
    <div className="flex flex-col h-screen bg-[var(--bg)]">
      {/* Header */}
      <div className="px-6 pt-8 pb-5 border-b border-[var(--border)]">
        <div className="flex items-center gap-3 mb-1">
          <span className="text-3xl">🔐</span>
          <div>
            <h1 className="text-lg font-bold text-[var(--text)] leading-tight">
              Local Security Pass Vault
            </h1>
            <p className="text-xs text-[var(--muted)]">Local password manager · No cloud · No telemetry</p>
          </div>
        </div>
      </div>

      {/* Vault list */}
      <div className="flex-1 overflow-y-auto px-5 py-4">
        {vaults.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-40 text-center">
            <div className="text-4xl mb-3">🗄</div>
            <p className="text-sm text-[var(--muted)]">No vaults yet. Create your first vault below.</p>
          </div>
        ) : (
          <div className="flex flex-col gap-2 max-w-xl mx-auto">
            <div className="text-xs font-medium text-[var(--muted)] uppercase tracking-wider mb-1 px-1">
              Your vaults
            </div>
            {vaults.map(v => (
              <button
                key={v.id}
                onClick={() => onOpenVault(v)}
                className="w-full text-left rounded-xl border border-[var(--border)] bg-[var(--surface)]
                           hover:border-[var(--accent)] hover:bg-[var(--surface)] p-4
                           transition-colors group relative"
              >
                <div className="flex items-start gap-3">
                  <div className="text-2xl mt-0.5 flex-shrink-0">🔒</div>
                  <div className="flex-1 min-w-0">
                    <div className="font-semibold text-[var(--text)] text-sm">{v.name}</div>
                    <div className="text-xs text-[var(--muted)] mt-0.5 font-mono truncate">{v.path}</div>
                    <div className="text-xs text-[var(--muted)] mt-1">
                      Last opened: {formatDate(v.lastOpened)}
                    </div>
                  </div>
                  <div className="flex flex-col items-end gap-1 flex-shrink-0 ml-2">
                    <button
                      onClick={e => handleRemove(v.id, e)}
                      className="opacity-0 group-hover:opacity-100 text-[var(--muted)] hover:text-[var(--danger)]
                                 text-xs px-2 py-1 rounded transition-all"
                      title="Remove from list"
                    >
                      ✕
                    </button>
                    <span className="text-xs text-[var(--accent)] font-medium opacity-0 group-hover:opacity-100 transition-opacity">
                      Open →
                    </span>
                  </div>
                </div>
              </button>
            ))}
          </div>
        )}
      </div>

      {/* Add existing vault form */}
      {adding && (
        <div className="border-t border-[var(--border)] px-5 py-4 bg-[var(--surface)]">
          <div className="max-w-xl mx-auto">
            <div className="text-sm font-semibold mb-3 text-[var(--text)]">Add existing vault</div>
            <div className="flex flex-col gap-3">
              <input
                type="text"
                value={addName}
                onChange={e => setAddName(e.target.value)}
                placeholder="Vault name (e.g. Personal, Work)"
                autoFocus
                className="w-full bg-[var(--bg)] border border-[var(--border)] rounded-lg px-3 py-2
                           text-sm text-[var(--text)] placeholder-[var(--muted)]
                           focus:outline-none focus:border-[var(--accent)] transition-colors"
              />
              <div className="flex gap-2">
                <input
                  type="text"
                  value={addPath}
                  onChange={e => setAddPath(e.target.value)}
                  placeholder="Path to vault folder…"
                  className="flex-1 bg-[var(--bg)] border border-[var(--border)] rounded-lg px-3 py-2
                             text-sm font-mono text-[var(--text)] placeholder-[var(--muted)]
                             focus:outline-none focus:border-[var(--accent)] transition-colors"
                />
                <button
                  onClick={handleBrowse}
                  className="px-4 py-2 bg-[var(--surface)] border border-[var(--border)] rounded-lg text-sm
                             hover:border-[var(--accent)] hover:text-[var(--accent)] transition-colors flex-shrink-0"
                >
                  Browse…
                </button>
              </div>
              {addError && (
                <div className="text-[var(--danger)] text-xs bg-red-950/30 border border-red-900/40 rounded-lg px-3 py-2">
                  {addError}
                </div>
              )}
              <div className="flex gap-2">
                <button
                  onClick={handleAdd}
                  className="flex-1 bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white
                             font-medium py-2 rounded-lg text-sm transition-colors"
                >
                  Add vault
                </button>
                <button
                  onClick={() => { setAdding(false); setAddName(""); setAddPath(""); setAddError(""); }}
                  className="flex-1 bg-[var(--surface)] border border-[var(--border)] text-[var(--muted)]
                             hover:text-[var(--text)] font-medium py-2 rounded-lg text-sm transition-colors"
                >
                  Cancel
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Bottom action buttons */}
      {!adding && (
        <div className="border-t border-[var(--border)] px-5 py-4">
          <div className="max-w-xl mx-auto flex gap-3">
            <button
              onClick={onCreateVault}
              className="flex-1 bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white
                         font-medium py-3 rounded-xl text-sm transition-colors"
            >
              + Create new vault
            </button>
            <button
              onClick={() => setAdding(true)}
              className="flex-1 bg-[var(--surface)] border border-[var(--border)]
                         text-[var(--text)] hover:border-[var(--accent)] hover:text-[var(--accent)]
                         font-medium py-3 rounded-xl text-sm transition-colors"
            >
              Add by path
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
