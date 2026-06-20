import { useState, useEffect } from "react";
import { decodeQrFromClipboard } from "../api/vault";
import { TotpCode } from "./TotpCode";

interface Props {
  value: string | null;
  onChange: (secret: string | null) => void;
}

export function TotpSetup({ value, onChange }: Props) {
  const [input, setInput]       = useState(value ?? "");
  const [decoding, setDecoding] = useState(false);
  const [error, setError]       = useState("");

  // Sync local input when parent resets value (e.g. loading a different item in the form)
  useEffect(() => {
    setInput(value ?? "");
    setError("");
  }, [value]);

  function handleInput(raw: string) {
    const cleaned = raw.trim().toUpperCase().replace(/\s+/g, "");
    setInput(raw);           // keep raw in input box for UX
    setError("");
    onChange(cleaned || null);
  }

  async function pasteQr() {
    setDecoding(true);
    setError("");
    try {
      const result = await decodeQrFromClipboard();
      setInput(result.secret);
      onChange(result.secret);
    } catch (e) {
      setError(String(e));
    } finally {
      setDecoding(false);
    }
  }

  return (
    <div className="flex flex-col gap-2">
      <label className="text-xs font-medium text-[var(--muted)] uppercase tracking-wide">
        2FA / TOTP (optional)
      </label>

      {/* Input row */}
      <div className="flex gap-2">
        <input
          type="text"
          value={input}
          onChange={e => handleInput(e.target.value)}
          placeholder="Paste Base32 secret (e.g. JBSWY3DPEHPK3PXP)"
          autoComplete="off"
          spellCheck={false}
          className="flex-1 bg-[var(--bg)] border border-[var(--border)] rounded-lg px-3 py-2
                     text-sm text-[var(--text)] placeholder-[var(--muted)] font-mono
                     focus:outline-none focus:border-[var(--accent)] transition-colors"
        />
        <button
          type="button"
          onClick={pasteQr}
          disabled={decoding}
          title="Copy QR code image to clipboard, then click here to decode it"
          className="flex items-center gap-1.5 px-3 py-2 rounded-lg border border-[var(--border)]
                     text-sm text-[var(--muted)] hover:text-[var(--accent)] hover:border-[var(--accent)]
                     transition-colors disabled:opacity-40 whitespace-nowrap"
        >
          {decoding ? "…" : "📷 Paste QR"}
        </button>
        {value && (
          <button
            type="button"
            onClick={() => { setInput(""); onChange(null); }}
            title="Remove TOTP"
            className="px-2 py-2 rounded-lg border border-[var(--border)] text-sm text-[var(--muted)]
                       hover:text-[var(--danger)] hover:border-[var(--danger)] transition-colors"
          >
            ✕
          </button>
        )}
      </div>

      {/* Instructions */}
      <p className="text-[10px] text-[var(--muted)] leading-relaxed">
        Paste the Base32 key shown when setting up 2FA (usually under "Can't scan the QR code?").
        Or: copy the QR code image to clipboard and click <strong>📷 Paste QR</strong>.
      </p>

      {/* Error */}
      {error && (
        <div className="text-xs text-[var(--danger)] bg-[var(--danger)]/10 rounded-lg px-3 py-2">
          {error}
        </div>
      )}

      {/* Live preview when secret is set */}
      {value && !error && (
        <div className="bg-[var(--surface)] border border-[var(--border)] rounded-lg px-3 py-2">
          <div className="text-[10px] text-[var(--muted)] mb-1.5 uppercase tracking-wide font-medium">
            Live preview
          </div>
          <TotpCode secret={value} />
        </div>
      )}
    </div>
  );
}
