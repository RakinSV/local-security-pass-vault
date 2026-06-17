import { useState, useEffect } from "react";
import { getItem, deleteItem } from "../api/vault";
import { PasswordField } from "../components/PasswordField";
import type { Item } from "../types/vault";

interface Props {
  itemId: string;
  onBack: () => void;
  onEdit: () => void;
  onDeleted: () => void;
}

function CopyBtn({ value }: { value: string }) {
  const [copied, setCopied] = useState(false);
  async function copy() {
    await navigator.clipboard.writeText(value);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  }
  return (
    <button
      onClick={copy}
      className="text-xs text-[var(--muted)] hover:text-[var(--accent)] transition-colors ml-1"
    >
      {copied ? "✓" : "copy"}
    </button>
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
            {p.totp_secret && <Field label="TOTP Secret" value={p.totp_secret} secret />}
            {p.notes && <Field label="Notes" value={p.notes} />}
            {p.custom_fields.map((cf, i) => (
              <Field key={i} label={cf.label} value={cf.value} secret={cf.hidden} />
            ))}
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

        <div className="text-xs text-[var(--muted)] pt-2 border-t border-[var(--border)]">
          Updated: {new Date(item.updatedAt * 1000).toLocaleString()}
        </div>
      </div>
    </div>
  );
}
