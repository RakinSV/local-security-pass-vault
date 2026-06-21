import { useState, useCallback, useEffect } from "react";
import { copyToClipboard } from "../api/vault";

interface Props {
  onUse: (password: string) => void;
}

const CHARS = {
  upper:   "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
  lower:   "abcdefghijklmnopqrstuvwxyz",
  digits:  "0123456789",
  symbols: "!@#$%^&*()-_=+[]{}|;:,.<>?",
};

interface Options {
  length:  number;
  upper:   boolean;
  lower:   boolean;
  digits:  boolean;
  symbols: boolean;
}

function generatePassword(opts: Options): string {
  const sets: Array<[boolean, string]> = [
    [opts.upper,   CHARS.upper],
    [opts.lower,   CHARS.lower],
    [opts.digits,  CHARS.digits],
    [opts.symbols, CHARS.symbols],
  ];

  let fullPool = "";
  for (const [enabled, chars] of sets) {
    if (enabled) fullPool += chars;
  }
  if (!fullPool) return "";

  // Fill with random characters from the full pool
  const buf = new Uint32Array(opts.length);
  crypto.getRandomValues(buf);
  const result = Array.from(buf, n => fullPool[n % fullPool.length]);

  // Overwrite positions with guaranteed chars from each set (using separate random values)
  const guaranteed: string[] = [];
  for (const [enabled, chars] of sets) {
    if (!enabled) continue;
    const pick = new Uint32Array(1);
    crypto.getRandomValues(pick);
    guaranteed.push(chars[pick[0] % chars.length]);
  }

  // Shuffle guaranteed characters into random positions
  const positions = new Uint32Array(guaranteed.length);
  crypto.getRandomValues(positions);
  const usedPositions = new Set<number>();
  for (let i = 0; i < guaranteed.length; i++) {
    let pos = positions[i] % opts.length;
    // Resolve collision — linear probe
    while (usedPositions.has(pos)) pos = (pos + 1) % opts.length;
    usedPositions.add(pos);
    result[pos] = guaranteed[i];
  }

  return result.join("");
}

function entropy(opts: Options): number {
  let poolSize = 0;
  if (opts.upper)   poolSize += CHARS.upper.length;
  if (opts.lower)   poolSize += CHARS.lower.length;
  if (opts.digits)  poolSize += CHARS.digits.length;
  if (opts.symbols) poolSize += CHARS.symbols.length;
  if (!poolSize)    return 0;
  return opts.length * Math.log2(poolSize);
}

function strengthLabel(bits: number): { label: string; color: string; width: string } {
  if (bits < 40)  return { label: "Weak",      color: "bg-red-500",    width: "w-1/4"   };
  if (bits < 60)  return { label: "Fair",       color: "bg-orange-400", width: "w-2/4"   };
  if (bits < 80)  return { label: "Strong",     color: "bg-yellow-400", width: "w-3/4"   };
  return           { label: "Very Strong", color: "bg-green-500",  width: "w-full"  };
}

const CLIPBOARD_TTL_MS = 30_000;

export function PasswordGenerator({ onUse }: Props) {
  const [opts, setOpts] = useState<Options>({
    length:  16,
    upper:   true,
    lower:   true,
    digits:  true,
    symbols: true,
  });
  const [password, setPassword] = useState("");
  const [copied, setCopied]     = useState(false);

  const regenerate = useCallback(() => {
    setPassword(generatePassword(opts));
  }, [opts]);

  // Auto-generate on first render and whenever options change
  useEffect(() => {
    regenerate();
  }, [regenerate]);

  async function copy() {
    if (!password) return;
    await copyToClipboard(password);
    setCopied(true);
    setTimeout(() => {
      copyToClipboard("").catch(() => {});
      setCopied(false);
    }, CLIPBOARD_TTL_MS);
  }

  function toggleOpt(key: "upper" | "lower" | "digits" | "symbols") {
    // Prevent disabling all sets
    const next = { ...opts, [key]: !opts[key] };
    const anyEnabled = next.upper || next.lower || next.digits || next.symbols;
    if (!anyEnabled) return;
    setOpts(next);
  }

  const bits = entropy(opts);
  const strength = strengthLabel(bits);

  return (
    <div className="flex flex-col gap-3 bg-[var(--surface)] border border-[var(--border)]
                    rounded-xl p-4">
      {/* Generated password display */}
      <div className="flex items-center gap-2">
        <div className="flex-1 bg-[var(--bg)] border border-[var(--border)] rounded-lg px-3 py-2
                        font-mono text-sm text-[var(--text)] break-all select-all min-h-[2.5rem]
                        flex items-center">
          {password || <span className="text-[var(--muted)]">—</span>}
        </div>
        <button
          type="button"
          onClick={copy}
          disabled={!password}
          title="Copy to clipboard (clears after 30 s)"
          className="px-3 py-2 rounded-lg border border-[var(--border)] text-sm
                     text-[var(--muted)] hover:text-[var(--accent)] hover:border-[var(--accent)]
                     transition-colors disabled:opacity-40 flex-shrink-0"
        >
          {copied ? "✓" : "copy"}
        </button>
        <button
          type="button"
          onClick={regenerate}
          title="Generate new password"
          className="px-3 py-2 rounded-lg border border-[var(--border)] text-sm
                     text-[var(--muted)] hover:text-[var(--accent)] hover:border-[var(--accent)]
                     transition-colors flex-shrink-0"
        >
          ↺
        </button>
      </div>

      {/* Strength bar */}
      <div className="flex items-center gap-2">
        <div className="flex-1 h-1.5 bg-[var(--border)] rounded-full overflow-hidden">
          <div className={`h-full rounded-full transition-all duration-300 ${strength.color} ${strength.width}`} />
        </div>
        <span className="text-[10px] text-[var(--muted)] w-20 text-right flex-shrink-0">
          {strength.label} · {Math.round(bits)} bits
        </span>
      </div>

      {/* Length slider */}
      <div className="flex items-center gap-3">
        <label className="text-xs text-[var(--muted)] w-14 flex-shrink-0">
          Length: <strong className="text-[var(--text)]">{opts.length}</strong>
        </label>
        <input
          type="range"
          min={8}
          max={64}
          value={opts.length}
          onChange={e => setOpts(prev => ({ ...prev, length: parseInt(e.target.value) }))}
          className="flex-1 accent-[var(--accent)] h-1.5"
        />
        <span className="text-[10px] text-[var(--muted)] w-4 text-right">64</span>
      </div>

      {/* Character set toggles */}
      <div className="flex flex-wrap gap-2">
        {([
          ["upper",   "A–Z"],
          ["lower",   "a–z"],
          ["digits",  "0–9"],
          ["symbols", "!@#"],
        ] as const).map(([key, label]) => (
          <button
            key={key}
            type="button"
            onClick={() => toggleOpt(key)}
            className={`px-3 py-1 rounded-lg text-xs font-medium border transition-colors
              ${opts[key]
                ? "bg-[var(--accent)]/20 border-[var(--accent)] text-[var(--accent)]"
                : "border-[var(--border)] text-[var(--muted)]"
              }`}
          >
            {label}
          </button>
        ))}
      </div>

      {/* Use button */}
      <button
        type="button"
        onClick={() => { if (password) onUse(password); }}
        disabled={!password}
        className="w-full bg-[var(--accent)] hover:bg-[var(--accent-hover)] disabled:opacity-40
                   text-white font-medium py-2.5 rounded-xl text-sm transition-colors"
      >
        Use this password
      </button>
    </div>
  );
}
