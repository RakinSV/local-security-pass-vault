import { useState, useEffect } from "react";
import { PasswordField } from "../components/PasswordField";
import { TotpSetup } from "../components/TotpSetup";
import { PasswordGenerator } from "../components/PasswordGenerator";
import { createItem, updateItem, getItem, listFolders } from "../api/vault";
import type { FolderInfo } from "../api/vault";
import type { ItemType, ItemPayload } from "../types/vault";

const TYPES: { value: ItemType; label: string }[] = [
  { value: "login",    label: "🔑 Login"       },
  { value: "card",     label: "💳 Card"         },
  { value: "note",     label: "📄 Secure Note"  },
  { value: "identity", label: "👤 Identity"     },
  { value: "ssh_key",  label: "🖥 SSH Key"      },
  { value: "server",   label: "🖧 Server"       },
];

function emptyPayload(type: ItemType): ItemPayload {
  switch (type) {
    case "login":
      return { type: "login", url: "", username: "", password: "", totp_secret: null, notes: null, custom_fields: [], password_history: [] };
    case "card":
      return { type: "card", cardholder: "", number: "", expiry_month: 1, expiry_year: new Date().getFullYear(), cvv: "", notes: null };
    case "note":
      return { type: "note", content: "" };
    case "identity":
      return { type: "identity", first_name: null, last_name: null, email: null, phone: null, address: null, passport: null, notes: null };
    case "ssh_key":
      return { type: "ssh_key", private_key: "", public_key: null, passphrase: null, notes: null };
    case "server":
      return { type: "server", host: "", port: null, username: null, auth_type: "password", password: null, ssh_private_key: null, ssh_passphrase: null, token: null, notes: null };
  }
}

interface Props {
  editId?: string;
  defaultType?: ItemType;
  onSaved: (id: string) => void;
  onBack: () => void;
}

export function ItemForm({ editId, defaultType = "login", onSaved, onBack }: Props) {
  const [itemType, setItemType] = useState<ItemType>(defaultType);
  const [title, setTitle] = useState("");
  const [payload, setPayload] = useState<ItemPayload>(emptyPayload(defaultType));
  const [favorite, setFavorite] = useState(false);
  const [folderId, setFolderId] = useState<string | null>(null);
  const [sourceTag, setSourceTag] = useState<string>("");
  const [folders, setFolders] = useState<FolderInfo[]>([]);
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);
  const [showGenerator, setShowGenerator] = useState(false);
  const [loading, setLoading] = useState(!!editId);

  useEffect(() => {
    listFolders().then(setFolders).catch(() => {});
  }, []);

  useEffect(() => {
    if (!editId) return;
    setLoading(true);
    getItem(editId)
      .then(item => {
        setTitle(item.title);
        setItemType(item.itemType);
        setPayload(item.payload);
        setFavorite(item.favorite);
        setFolderId(item.folderId);
        setSourceTag(item.sourceTag ?? "");
      })
      .catch(err => setError(String(err)))
      .finally(() => setLoading(false));
  }, [editId]);

  function changeType(t: ItemType) {
    setItemType(t);
    setPayload(emptyPayload(t));
  }

  function setField(key: string, value: unknown) {
    setPayload(prev => ({ ...prev, [key]: value } as ItemPayload));
  }

  async function handleSave(e: React.FormEvent) {
    e.preventDefault();
    setError("");
    if (!title.trim()) { setError("Title is required."); return; }

    setSaving(true);
    const tag = sourceTag.trim() || null;
    try {
      if (editId) {
        await updateItem(editId, title, payload, folderId, favorite, tag);
        onSaved(editId);
      } else {
        const id = await createItem(title, payload, folderId, favorite, tag);
        onSaved(id);
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setSaving(false);
    }
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center h-screen text-[var(--muted)] text-sm">
        Loading…
      </div>
    );
  }

  const p = payload;

  return (
    <div className="flex flex-col h-screen">
      <div className="flex items-center gap-3 px-4 py-3 border-b border-[var(--border)]">
        <button
          onClick={onBack}
          className="text-[var(--muted)] hover:text-[var(--text)] transition-colors text-sm"
        >
          ←
        </button>
        <div className="flex-1 font-medium">
          {editId ? "Edit item" : "New item"}
        </div>
        <label className="flex items-center gap-2 text-sm text-[var(--muted)] cursor-pointer">
          <input
            type="checkbox"
            checked={favorite}
            onChange={e => setFavorite(e.target.checked)}
            className="accent-yellow-400"
          />
          Favourite
        </label>
      </div>

      <div className="flex-1 overflow-y-auto p-4 max-w-lg mx-auto w-full">
        <form onSubmit={handleSave} className="flex flex-col gap-4">
          {/* Type selector (only on create) */}
          {!editId && (
            <div className="flex flex-col gap-1">
              <label className="text-xs font-medium text-[var(--muted)] uppercase tracking-wide">
                Type
              </label>
              <div className="flex flex-wrap gap-2">
                {TYPES.map(t => (
                  <button
                    key={t.value}
                    type="button"
                    onClick={() => changeType(t.value)}
                    className={`px-3 py-1.5 rounded-lg text-sm transition-colors border
                      ${itemType === t.value
                        ? "bg-[var(--accent)]/20 border-[var(--accent)] text-[var(--accent)]"
                        : "border-[var(--border)] text-[var(--muted)] hover:text-[var(--text)]"
                      }`}
                  >
                    {t.label}
                  </button>
                ))}
              </div>
            </div>
          )}

          {/* Title */}
          <div className="flex flex-col gap-1">
            <label className="text-xs font-medium text-[var(--muted)] uppercase tracking-wide">
              Title *
            </label>
            <input
              type="text"
              value={title}
              onChange={e => setTitle(e.target.value)}
              placeholder="e.g. GitHub, Bank of America"
              autoFocus
              className="w-full bg-[var(--bg)] border border-[var(--border)] rounded-lg px-3 py-2
                         text-sm text-[var(--text)] placeholder-[var(--muted)]
                         focus:outline-none focus:border-[var(--accent)] transition-colors"
            />
          </div>

          {/* Login fields */}
          {p.type === "login" && (
            <>
              <TextField label="URL" value={p.url} onChange={v => setField("url", v)} placeholder="https://example.com" />
              <TextField label="Username" value={p.username} onChange={v => setField("username", v)} placeholder="user@example.com" />
              {/* Password field + generator toggle */}
              <div className="flex flex-col gap-2">
                <div className="flex items-end gap-2">
                  <div className="flex-1">
                    <PasswordField label="Password" value={p.password} onChange={v => setField("password", v)} placeholder="Password" />
                  </div>
                  <button
                    type="button"
                    onClick={() => setShowGenerator(prev => !prev)}
                    title="Generate secure password"
                    className={`mb-0.5 px-3 py-2 rounded-lg border text-xs font-medium transition-colors
                      ${showGenerator
                        ? "border-[var(--accent)] bg-[var(--accent)]/20 text-[var(--accent)]"
                        : "border-[var(--border)] text-[var(--muted)] hover:border-[var(--accent)] hover:text-[var(--accent)]"
                      }`}
                  >
                    ⚡ Generate
                  </button>
                </div>
                {showGenerator && (
                  <PasswordGenerator
                    onUse={pw => {
                      setField("password", pw);
                      setShowGenerator(false);
                    }}
                  />
                )}
              </div>
              <TotpSetup value={p.totp_secret ?? null} onChange={v => setField("totp_secret", v)} />
              <TextareaField label="Notes (optional)" value={p.notes ?? ""} onChange={v => setField("notes", v || null)} />
              {/* Custom fields */}
              <div className="flex flex-col gap-2 pt-1 border-t border-[var(--border)]/40">
                <div className="flex items-center justify-between">
                  <label className="text-xs font-medium text-[var(--muted)] uppercase tracking-wide">
                    Custom fields
                  </label>
                  <button
                    type="button"
                    onClick={() => setField("custom_fields", [...p.custom_fields, { label: "", value: "", hidden: false }])}
                    className="text-xs text-[var(--accent)] hover:text-[var(--accent-hover)] transition-colors"
                  >
                    + Add field
                  </button>
                </div>
                {p.custom_fields.map((cf, i) => (
                  <div key={i} className="flex items-center gap-2">
                    <input
                      type="text"
                      value={cf.label}
                      onChange={e => {
                        const updated = p.custom_fields.map((f, j) => j === i ? { ...f, label: e.target.value } : f);
                        setField("custom_fields", updated);
                      }}
                      placeholder="Field name"
                      className="w-28 flex-shrink-0 bg-[var(--bg)] border border-[var(--border)] rounded-lg px-2 py-1.5
                                 text-xs text-[var(--text)] placeholder-[var(--muted)]
                                 focus:outline-none focus:border-[var(--accent)] transition-colors"
                    />
                    <input
                      type={cf.hidden ? "password" : "text"}
                      value={cf.value}
                      onChange={e => {
                        const updated = p.custom_fields.map((f, j) => j === i ? { ...f, value: e.target.value } : f);
                        setField("custom_fields", updated);
                      }}
                      placeholder="Value"
                      className="flex-1 bg-[var(--bg)] border border-[var(--border)] rounded-lg px-2 py-1.5
                                 text-xs text-[var(--text)] placeholder-[var(--muted)]
                                 focus:outline-none focus:border-[var(--accent)] transition-colors"
                    />
                    <button
                      type="button"
                      title={cf.hidden ? "Show value" : "Treat as secret (hidden)"}
                      onClick={() => {
                        const updated = p.custom_fields.map((f, j) => j === i ? { ...f, hidden: !f.hidden } : f);
                        setField("custom_fields", updated);
                      }}
                      className={`flex-shrink-0 px-1.5 py-1.5 rounded-lg border text-xs transition-colors
                        ${cf.hidden
                          ? "border-[var(--accent)] text-[var(--accent)]"
                          : "border-[var(--border)] text-[var(--muted)]"
                        }`}
                    >
                      {cf.hidden ? "🔒" : "👁"}
                    </button>
                    <button
                      type="button"
                      onClick={() => setField("custom_fields", p.custom_fields.filter((_, j) => j !== i))}
                      className="flex-shrink-0 text-[var(--muted)] hover:text-[var(--danger)] transition-colors text-base"
                    >
                      ×
                    </button>
                  </div>
                ))}
              </div>
            </>
          )}

          {/* Card fields */}
          {p.type === "card" && (
            <>
              <TextField label="Cardholder" value={p.cardholder} onChange={v => setField("cardholder", v)} />
              <PasswordField label="Card number" value={p.number} onChange={v => setField("number", v)} placeholder="1234 5678 9012 3456" />
              <div className="grid grid-cols-2 gap-3">
                <div className="flex flex-col gap-1">
                  <label className="text-xs font-medium text-[var(--muted)] uppercase tracking-wide">Expiry month</label>
                  <input type="number" min={1} max={12} value={p.expiry_month}
                    onChange={e => setField("expiry_month", parseInt(e.target.value))}
                    className="bg-[var(--bg)] border border-[var(--border)] rounded-lg px-3 py-2 text-sm text-[var(--text)] focus:outline-none focus:border-[var(--accent)]" />
                </div>
                <div className="flex flex-col gap-1">
                  <label className="text-xs font-medium text-[var(--muted)] uppercase tracking-wide">Expiry year</label>
                  <input type="number" min={2024} max={2050} value={p.expiry_year}
                    onChange={e => setField("expiry_year", parseInt(e.target.value))}
                    className="bg-[var(--bg)] border border-[var(--border)] rounded-lg px-3 py-2 text-sm text-[var(--text)] focus:outline-none focus:border-[var(--accent)]" />
                </div>
              </div>
              <PasswordField label="CVV" value={p.cvv} onChange={v => setField("cvv", v)} placeholder="123" />
              <TextareaField label="Notes (optional)" value={p.notes ?? ""} onChange={v => setField("notes", v || null)} />
            </>
          )}

          {/* Note fields */}
          {p.type === "note" && (
            <div className="flex flex-col gap-1">
              <label className="text-xs font-medium text-[var(--muted)] uppercase tracking-wide">Content</label>
              <textarea
                value={p.content}
                onChange={e => setField("content", e.target.value)}
                rows={10}
                placeholder="Secure note content…"
                className="w-full bg-[var(--bg)] border border-[var(--border)] rounded-lg px-3 py-2
                           text-sm text-[var(--text)] placeholder-[var(--muted)] resize-none
                           focus:outline-none focus:border-[var(--accent)] transition-colors"
              />
            </div>
          )}

          {/* Identity fields */}
          {p.type === "identity" && (
            <>
              <div className="grid grid-cols-2 gap-3">
                <TextField label="First name" value={p.first_name ?? ""} onChange={v => setField("first_name", v || null)} />
                <TextField label="Last name" value={p.last_name ?? ""} onChange={v => setField("last_name", v || null)} />
              </div>
              <TextField label="Email" value={p.email ?? ""} onChange={v => setField("email", v || null)} placeholder="user@example.com" />
              <TextField label="Phone" value={p.phone ?? ""} onChange={v => setField("phone", v || null)} />
              <TextField label="Address" value={p.address ?? ""} onChange={v => setField("address", v || null)} />
              <PasswordField label="Passport (optional)" value={p.passport ?? ""} onChange={v => setField("passport", v || null)} />
              <TextareaField label="Notes (optional)" value={p.notes ?? ""} onChange={v => setField("notes", v || null)} />
            </>
          )}

          {/* SSH Key fields */}
          {p.type === "ssh_key" && (
            <>
              <div className="flex flex-col gap-1">
                <label className="text-xs font-medium text-[var(--muted)] uppercase tracking-wide">Private key (PEM)</label>
                <textarea
                  value={p.private_key}
                  onChange={e => setField("private_key", e.target.value)}
                  rows={8}
                  placeholder="-----BEGIN OPENSSH PRIVATE KEY-----"
                  className="w-full bg-[var(--bg)] border border-[var(--border)] rounded-lg px-3 py-2
                             text-xs text-[var(--text)] placeholder-[var(--muted)] font-mono resize-none
                             focus:outline-none focus:border-[var(--accent)] transition-colors"
                />
              </div>
              <div className="flex flex-col gap-1">
                <label className="text-xs font-medium text-[var(--muted)] uppercase tracking-wide">Public key (optional)</label>
                <textarea
                  value={p.public_key ?? ""}
                  onChange={e => setField("public_key", e.target.value || null)}
                  rows={3}
                  placeholder="ssh-ed25519 AAAA…"
                  className="w-full bg-[var(--bg)] border border-[var(--border)] rounded-lg px-3 py-2
                             text-xs text-[var(--text)] placeholder-[var(--muted)] font-mono resize-none
                             focus:outline-none focus:border-[var(--accent)] transition-colors"
                />
              </div>
              <PasswordField label="Passphrase (optional)" value={p.passphrase ?? ""} onChange={v => setField("passphrase", v || null)} />
              <TextareaField label="Notes (optional)" value={p.notes ?? ""} onChange={v => setField("notes", v || null)} />
            </>
          )}

          {/* Server / Infrastructure fields */}
          {p.type === "server" && (
            <>
              <div className="grid grid-cols-3 gap-3">
                <div className="col-span-2">
                  <TextField label="Host / IP" value={p.host} onChange={v => setField("host", v)} placeholder="192.168.1.1 or server.example.com" />
                </div>
                <div className="flex flex-col gap-1">
                  <label className="text-xs font-medium text-[var(--muted)] uppercase tracking-wide">Port</label>
                  <input
                    type="number"
                    min={1} max={65535}
                    value={p.port ?? ""}
                    onChange={e => setField("port", e.target.value ? parseInt(e.target.value) : null)}
                    placeholder="22"
                    className="w-full bg-[var(--bg)] border border-[var(--border)] rounded-lg px-3 py-2
                               text-sm text-[var(--text)] placeholder-[var(--muted)]
                               focus:outline-none focus:border-[var(--accent)] transition-colors"
                  />
                </div>
              </div>
              <TextField label="Username (optional)" value={p.username ?? ""} onChange={v => setField("username", v || null)} placeholder="root, admin…" />

              {/* Auth type selector */}
              <div className="flex flex-col gap-1">
                <label className="text-xs font-medium text-[var(--muted)] uppercase tracking-wide">Auth type</label>
                <div className="flex gap-2">
                  {(["password", "ssh_key", "token"] as const).map(at => (
                    <button
                      key={at}
                      type="button"
                      onClick={() => setField("auth_type", at)}
                      className={`px-3 py-1.5 rounded-lg text-sm transition-colors border
                        ${p.auth_type === at
                          ? "bg-[var(--accent)]/20 border-[var(--accent)] text-[var(--accent)]"
                          : "border-[var(--border)] text-[var(--muted)] hover:text-[var(--text)]"
                        }`}
                    >
                      {at === "password" ? "Password" : at === "ssh_key" ? "SSH Key" : "Token / API key"}
                    </button>
                  ))}
                </div>
              </div>

              {/* Progressive disclosure by auth type */}
              {p.auth_type === "password" && (
                <PasswordField label="Password" value={p.password ?? ""} onChange={v => setField("password", v || null)} />
              )}
              {p.auth_type === "ssh_key" && (
                <>
                  <div className="flex flex-col gap-1">
                    <label className="text-xs font-medium text-[var(--muted)] uppercase tracking-wide">Private key (PEM)</label>
                    <textarea
                      value={p.ssh_private_key ?? ""}
                      onChange={e => setField("ssh_private_key", e.target.value || null)}
                      rows={6}
                      placeholder="-----BEGIN OPENSSH PRIVATE KEY-----"
                      className="w-full bg-[var(--bg)] border border-[var(--border)] rounded-lg px-3 py-2
                                 text-xs text-[var(--text)] placeholder-[var(--muted)] font-mono resize-none
                                 focus:outline-none focus:border-[var(--accent)] transition-colors"
                    />
                  </div>
                  <PasswordField label="Key passphrase (optional)" value={p.ssh_passphrase ?? ""} onChange={v => setField("ssh_passphrase", v || null)} />
                </>
              )}
              {p.auth_type === "token" && (
                <PasswordField label="Token / API key" value={p.token ?? ""} onChange={v => setField("token", v || null)} placeholder="eyJ… or glpat-…" />
              )}

              <TextareaField label="Notes (optional)" value={p.notes ?? ""} onChange={v => setField("notes", v || null)} />
            </>
          )}

          {/* Folder assignment */}
          {folders.length > 0 && (
            <div className="flex flex-col gap-1 pt-1 border-t border-[var(--border)]/40 mt-1">
              <label className="text-xs font-medium text-[var(--muted)] uppercase tracking-wide">
                Folder (optional)
              </label>
              <select
                value={folderId ?? ""}
                onChange={e => setFolderId(e.target.value || null)}
                className="w-full bg-[var(--bg)] border border-[var(--border)] rounded-lg px-3 py-2
                           text-sm text-[var(--text)] focus:outline-none focus:border-[var(--accent)] transition-colors"
              >
                <option value="">— No folder —</option>
                {folders.map(f => (
                  <option key={f.id} value={f.id}>
                    {f.icon ?? "📁"} {f.name}
                  </option>
                ))}
              </select>
            </div>
          )}

          {/* Source tag — optional label for all item types */}
          <div className="flex flex-col gap-1 pt-1 border-t border-[var(--border)]/40 mt-1">
            <label className="text-xs font-medium text-[var(--muted)] uppercase tracking-wide">
              Source / Profile tag (optional)
            </label>
            <input
              type="text"
              value={sourceTag}
              onChange={e => setSourceTag(e.target.value)}
              placeholder="e.g. Work Chrome, Personal Firefox"
              className="w-full bg-[var(--bg)] border border-[var(--border)] rounded-lg px-3 py-2
                         text-sm text-[var(--text)] placeholder-[var(--muted)]
                         focus:outline-none focus:border-[var(--accent)] transition-colors"
            />
          </div>

          {error && (
            <div className="text-[var(--danger)] text-sm bg-red-950/30 border border-red-900/40 rounded-lg px-3 py-2">
              {error}
            </div>
          )}

          <div className="flex gap-3 pt-2">
            <button
              type="button"
              onClick={onBack}
              className="flex-1 border border-[var(--border)] text-[var(--muted)]
                         hover:text-[var(--text)] py-2.5 rounded-xl text-sm transition-colors"
            >
              Cancel
            </button>
            <button
              type="submit"
              disabled={saving}
              className="flex-1 bg-[var(--accent)] hover:bg-[var(--accent-hover)] disabled:opacity-40
                         text-white font-medium py-2.5 rounded-xl text-sm transition-colors"
            >
              {saving ? "Saving…" : editId ? "Save changes" : "Add item"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

function TextField({ label, value, onChange, placeholder }: {
  label: string; value: string; onChange: (v: string) => void; placeholder?: string;
}) {
  return (
    <div className="flex flex-col gap-1">
      <label className="text-xs font-medium text-[var(--muted)] uppercase tracking-wide">{label}</label>
      <input
        type="text"
        value={value}
        onChange={e => onChange(e.target.value)}
        placeholder={placeholder}
        className="w-full bg-[var(--bg)] border border-[var(--border)] rounded-lg px-3 py-2
                   text-sm text-[var(--text)] placeholder-[var(--muted)]
                   focus:outline-none focus:border-[var(--accent)] transition-colors"
      />
    </div>
  );
}

function TextareaField({ label, value, onChange }: {
  label: string; value: string; onChange: (v: string) => void;
}) {
  return (
    <div className="flex flex-col gap-1">
      <label className="text-xs font-medium text-[var(--muted)] uppercase tracking-wide">{label}</label>
      <textarea
        value={value}
        onChange={e => onChange(e.target.value)}
        rows={3}
        className="w-full bg-[var(--bg)] border border-[var(--border)] rounded-lg px-3 py-2
                   text-sm text-[var(--text)] placeholder-[var(--muted)] resize-none
                   focus:outline-none focus:border-[var(--accent)] transition-colors"
      />
    </div>
  );
}
