import { useState, useEffect } from "react";
import { getItem, deleteItem, checkPasswordBreach, copyToClipboard } from "../api/vault";
import { PasswordField } from "../components/PasswordField";
import { TotpCode } from "../components/TotpCode";
import type { Item, PasswordHistoryEntry } from "../types/vault";

interface Props {
  itemId: string;
  onBack: () => void;
  onEdit: () => void;
  onDeleted: () => void;
}

const CLIPBOARD_TTL_MS = 30_000;

function CopyBtn({ value }: { value: string }) {
  const [copied, setCopied] = useState(false);
  async function copy() {
    await copyToClipboard(value);
    setCopied(true);
    // Clear clipboard after 30 seconds (security.md).
    setTimeout(() => {
      copyToClipboard("").catch(() => {});
      setCopied(false);
    }, CLIPBOARD_TTL_MS);
  }
  return (
    <button
      onClick={copy}
      className="text-xs text-[var(--muted)] hover:text-[var(--accent)] transition-colors ml-1"
    >
      {copied ? "✓ 30s" : "copy"}
    </button>
  );
}

function PasswordHistorySection({ history }: { history: PasswordHistoryEntry[] }) {
  const [open, setOpen] = useState(false);
  return (
    <div className="flex flex-col gap-2">
      <button
        onClick={() => setOpen(o => !o)}
        className="flex items-center gap-1.5 text-xs font-medium text-[var(--muted)] uppercase
                   tracking-wide hover:text-[var(--text)] transition-colors text-left"
      >
        <span className="text-[10px]">{open ? "▾" : "▸"}</span>
        Password History ({history.length})
      </button>
      {open && (
        <div className="flex flex-col divide-y divide-[var(--border)] border border-[var(--border)]
                        rounded-xl bg-[var(--surface)] overflow-hidden">
          {history.map((entry, i) => (
            <div key={i} className="px-3 py-2 flex flex-col gap-1">
              <div className="text-[10px] text-[var(--muted)]">
                {new Date(entry.changed_at * 1000).toLocaleString()}
              </div>
              <div className="flex items-center gap-2">
                <div className="flex-1">
                  <PasswordField label="" value={entry.password} readOnly />
                </div>
                <CopyBtn value={entry.password} />
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function HibpCheckBtn({ password }: { password: string }) {
  const [state, setState] = useState<"idle" | "checking" | "safe" | "pwned" | "offline">("idle");
  const [count, setCount] = useState(0);

  async function check() {
    setState("checking");
    try {
      const r = await checkPasswordBreach(password);
      if (!r.checked) { setState("offline"); return; }
      if (r.pwnedCount > 0) { setCount(r.pwnedCount); setState("pwned"); }
      else setState("safe");
    } catch { setState("offline"); }
  }

  if (state === "idle") {
    return (
      <button
        onClick={check}
        className="text-xs text-[var(--muted)] hover:text-amber-400 transition-colors"
      >
        Check breaches (HIBP)
      </button>
    );
  }
  if (state === "checking") return <span className="text-xs text-[var(--muted)]">Checking…</span>;
  if (state === "safe")     return <span className="text-xs text-emerald-400">✓ Not found in breaches</span>;
  if (state === "offline")  return <span className="text-xs text-[var(--muted)]">Offline — could not check</span>;
  return (
    <span className="text-xs text-red-400 font-medium">
      ⚠ Compromised in {count.toLocaleString()} breach{count !== 1 ? "es" : ""}
    </span>
  );
}

function Field({ label, value, secret }: { label: string; value: string; secret?: boolean }) {
  if (!value) return null;
  return (
    <div className="flex flex-col gap-1">
      <div className="text-xs font-medium text-[var(--muted)] uppercase tracking-wide">{label}</div>
      {secret ? (
        <div className="flex items-center gap-2">
          <div className="flex-1">
            <PasswordField label="" value={value} readOnly />
          </div>
          <CopyBtn value={value} />
        </div>
      ) : (
        <div className="flex items-center gap-2">
          <div className="flex-1 bg-[var(--bg)] border border-[var(--border)] rounded-lg px-3 py-2
                          text-sm text-[var(--text)] break-all">
            {value}
          </div>
          <CopyBtn value={value} />
        </div>
      )}
    </div>
  );
}

export function ItemDetail({ itemId, onBack, onEdit, onDeleted }: Props) {
  const [item, setItem] = useState<Item | null>(null);
  const [loading, setLoading] = useState(true);
  const [deleting, setDeleting] = useState(false);

  useEffect(() => {
    setLoading(true);
    getItem(itemId)
      .then(setItem)
      .catch(console.error)
      .finally(() => setLoading(false));
  }, [itemId]);

  async function handleDelete() {
    if (!confirm(`Delete "${item?.title}"?`)) return;
    setDeleting(true);
    try {
      await deleteItem(itemId);
      onDeleted();
    } catch (err) {
      console.error(err);
      setDeleting(false);
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-screen text-[var(--muted)] text-sm">
        Loading…
      </div>
    );
  }

  if (!item) {
    return (
      <div className="flex flex-col items-center justify-center h-screen gap-4">
        <div className="text-[var(--muted)]">Item not found</div>
        <button onClick={onBack} className="text-[var(--accent)] text-sm">← Back</button>
      </div>
    );
  }

  const p = item.payload;

  return (
    <div className="flex flex-col h-screen">
      {/* Header */}
      <div className="flex items-center gap-3 px-4 py-3 border-b border-[var(--border)]">
        <button
          onClick={onBack}
          className="text-[var(--muted)] hover:text-[var(--text)] transition-colors text-sm"
        >
          ←
        </button>
        <div className="flex-1 font-medium truncate">{item.title}</div>
        <button
          onClick={onEdit}
          className="text-sm text-[var(--accent)] hover:text-[var(--accent-hover)] transition-colors"
        >
          Edit
        </button>
        <button
          onClick={handleDelete}
          disabled={deleting}
          className="text-sm text-[var(--danger)] hover:text-red-400 transition-colors disabled:opacity-40"
        >
          Delete
        </button>
      </div>

      {/* Fields */}
      <div className="flex-1 overflow-y-auto p-4 flex flex-col gap-4 max-w-lg mx-auto w-full">
        {p.type === "login" && (
          <>
            <Field label="URL" value={p.url} />
            <Field label="Username" value={p.username} />
            <Field label="Password" value={p.password} secret />
            {p.password && (
              <div className="flex items-center gap-2 -mt-2 pl-1">
                <HibpCheckBtn password={p.password} />
              </div>
            )}
            {p.totp_secret && (
              <div className="flex flex-col gap-1">
                <div className="text-xs font-medium text-[var(--muted)] uppercase tracking-wide">2FA Code</div>
                <div className="bg-[var(--bg)] border border-[var(--border)] rounded-lg px-3 py-2">
                  <TotpCode secret={p.totp_secret} />
                </div>
              </div>
            )}
            {p.notes && <Field label="Notes" value={p.notes} />}
            {p.custom_fields.map((cf, i) => (
              <Field key={i} label={cf.label} value={cf.value} secret={cf.hidden} />
            ))}
            {p.password_history.length > 0 && (
              <PasswordHistorySection history={p.password_history} />
            )}
          </>
        )}

        {p.type === "card" && (
          <>
            <Field label="Cardholder" value={p.cardholder} />
            <Field label="Number" value={p.number} secret />
            <Field label="Expiry" value={`${p.expiry_month}/${p.expiry_year}`} />
            <Field label="CVV" value={p.cvv} secret />
            {p.notes && <Field label="Notes" value={p.notes} />}
          </>
        )}

        {p.type === "note" && (
          <div className="flex flex-col gap-1">
            <div className="text-xs font-medium text-[var(--muted)] uppercase tracking-wide">Content</div>
            <div className="bg-[var(--bg)] border border-[var(--border)] rounded-lg px-3 py-2
                            text-sm text-[var(--text)] whitespace-pre-wrap min-h-[120px]">
              {p.content}
            </div>
          </div>
        )}

        {p.type === "identity" && (
          <>
            {p.first_name && <Field label="First name" value={p.first_name} />}
            {p.last_name && <Field label="Last name" value={p.last_name} />}
            {p.email && <Field label="Email" value={p.email} />}
            {p.phone && <Field label="Phone" value={p.phone} />}
            {p.address && <Field label="Address" value={p.address} />}
            {p.passport && <Field label="Passport" value={p.passport} secret />}
            {p.notes && <Field label="Notes" value={p.notes} />}
          </>
        )}

        {p.type === "ssh_key" && (
          <>
            <div className="flex flex-col gap-1">
              <div className="text-xs font-medium text-[var(--muted)] uppercase tracking-wide">
                Private Key
              </div>
              <textarea
                readOnly
                value={p.private_key}
                rows={6}
                className="w-full bg-[var(--bg)] border border-[var(--border)] rounded-lg px-3 py-2
                           text-xs text-[var(--text)] font-mono resize-none
                           focus:outline-none"
              />
              <CopyBtn value={p.private_key} />
            </div>
            {p.public_key && (
              <div className="flex flex-col gap-1">
                <div className="text-xs font-medium text-[var(--muted)] uppercase tracking-wide">
                  Public Key
                </div>
                <textarea
                  readOnly
                  value={p.public_key}
                  rows={3}
                  className="w-full bg-[var(--bg)] border border-[var(--border)] rounded-lg px-3 py-2
                             text-xs text-[var(--text)] font-mono resize-none focus:outline-none"
                />
              </div>
            )}
            {p.passphrase && <Field label="Passphrase" value={p.passphrase} secret />}
            {p.notes && <Field label="Notes" value={p.notes} />}
          </>
        )}

        {p.type === "server" && (
          <>
            <Field label="Host" value={p.host} />
            {p.port != null && <Field label="Port" value={String(p.port)} />}
            {p.username && <Field label="Username" value={p.username} />}
            <div className="flex flex-col gap-1">
              <div className="text-xs font-medium text-[var(--muted)] uppercase tracking-wide">Auth type</div>
              <div className="text-sm text-[var(--text)]">
                {p.auth_type === "password" ? "Password" : p.auth_type === "ssh_key" ? "SSH Key" : "Token / API key"}
              </div>
            </div>
            {p.auth_type === "password" && p.password && <Field label="Password" value={p.password} secret />}
            {p.auth_type === "ssh_key" && p.ssh_private_key && (
              <div className="flex flex-col gap-1">
                <div className="text-xs font-medium text-[var(--muted)] uppercase tracking-wide">Private Key</div>
                <textarea
                  readOnly
                  value={p.ssh_private_key}
                  rows={6}
                  className="w-full bg-[var(--bg)] border border-[var(--border)] rounded-lg px-3 py-2
                             text-xs text-[var(--text)] font-mono resize-none focus:outline-none"
                />
                <CopyBtn value={p.ssh_private_key} />
              </div>
            )}
            {p.auth_type === "ssh_key" && p.ssh_passphrase && <Field label="Key passphrase" value={p.ssh_passphrase} secret />}
            {p.auth_type === "token" && p.token && <Field label="Token / API key" value={p.token} secret />}
            {p.notes && <Field label="Notes" value={p.notes} />}
          </>
        )}

        {/* Source tag badge */}
        {item.sourceTag && (
          <div className="flex items-center gap-2 pt-1">
            <div className="text-xs font-medium text-[var(--muted)] uppercase tracking-wide">Source</div>
            <span className="text-xs px-2 py-0.5 rounded-full bg-[var(--accent)]/15 text-[var(--accent)] border border-[var(--accent)]/30">
              {item.sourceTag}
            </span>
          </div>
        )}

        <div className="text-xs text-[var(--muted)] pt-2 border-t border-[var(--border)]">
          Updated: {new Date(item.updatedAt * 1000).toLocaleString()}
        </div>
      </div>
    </div>
  );
}
