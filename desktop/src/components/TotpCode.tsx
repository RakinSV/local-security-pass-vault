import { useState, useEffect, useCallback } from "react";
import { generateTotp } from "../api/vault";
import type { TotpCode as TotpCodeData } from "../api/vault";

interface Props {
  secret: string;
}

const CLIPBOARD_TTL_MS = 30_000;

export function TotpCode({ secret }: Props) {
  const [data, setData]     = useState<TotpCodeData | null>(null);
  const [error, setError]   = useState("");
  const [copied, setCopied] = useState(false);

  const refresh = useCallback(() => {
    generateTotp(secret)
      .then(d => { setData(d); setError(""); })
      .catch(e => setError(String(e)));
  }, [secret]);

  // Refresh immediately and then every second
  useEffect(() => {
    refresh();
    const id = setInterval(refresh, 1000);
    return () => clearInterval(id);
  }, [refresh]);

  async function copyCode() {
    if (!data) return;
    await navigator.clipboard.writeText(data.code);
    setCopied(true);
    setTimeout(() => {
      navigator.clipboard.writeText("").catch(() => {});
      setCopied(false);
    }, CLIPBOARD_TTL_MS);
  }

  if (error) {
    return (
      <div className="text-xs text-[var(--danger)]">TOTP error: {error}</div>
    );
  }

  if (!data) {
    return <div className="text-xs text-[var(--muted)]">Loading…</div>;
  }

  const { code, validForSecs, periodSecs } = data;
  const progress = validForSecs / periodSecs;
  // SVG ring: radius 10, circumference ≈ 62.83
  const r = 10;
  const circ = 2 * Math.PI * r;
  const dash = circ * progress;
  const gap  = circ - dash;
  const isUrgent = validForSecs <= 5;

  return (
    <div className="flex items-center gap-3">
      {/* Circular countdown ring */}
      <div className="relative flex items-center justify-center w-9 h-9 flex-shrink-0">
        <svg viewBox="0 0 26 26" className="absolute inset-0 w-full h-full -rotate-90">
          {/* Background track */}
          <circle cx="13" cy="13" r={r} fill="none"
            stroke="var(--border)" strokeWidth="2.5" />
          {/* Progress arc */}
          <circle cx="13" cy="13" r={r} fill="none"
            stroke={isUrgent ? "var(--danger)" : "var(--accent)"}
            strokeWidth="2.5"
            strokeDasharray={`${dash} ${gap}`}
            strokeLinecap="round" />
        </svg>
        {/* Seconds remaining */}
        <span className={`relative text-[9px] font-bold tabular-nums leading-none
          ${isUrgent ? "text-[var(--danger)]" : "text-[var(--muted)]"}`}>
          {validForSecs}
        </span>
      </div>

      {/* Code + copy */}
      <div className="flex items-center gap-2 flex-1">
        <span className={`font-mono text-xl font-bold tracking-[0.25em] select-all
          ${isUrgent ? "text-[var(--danger)]" : "text-[var(--text)]"}`}>
          {/* Insert thin space at position 3 for readability: 123 456 */}
          {code.slice(0, 3)}&thinsp;{code.slice(3)}
        </span>
        <button
          onClick={copyCode}
          title="Copy code"
          className="text-xs text-[var(--muted)] hover:text-[var(--accent)] transition-colors ml-auto"
        >
          {copied ? "✓ 30s" : "copy"}
        </button>
      </div>
    </div>
  );
}
