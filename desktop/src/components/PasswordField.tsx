import { useState } from "react";
import { setScreenCaptureProtection } from "../api/vault";

interface Props {
  label: string;
  value: string;
  onChange?: (v: string) => void;
  placeholder?: string;
  readOnly?: boolean;
  autoFocus?: boolean;
}

export function PasswordField({ label, value, onChange, placeholder, readOnly, autoFocus }: Props) {
  const [visible, setVisible] = useState(false);

  function toggleVisible() {
    const next = !visible;
    setVisible(next);
    // On Windows: hide this window from screen-capture tools while password
    // is shown in plaintext.  No-op on other platforms.
    setScreenCaptureProtection(next).catch(() => {});
  }

  return (
    <div className="flex flex-col gap-1">
      <label className="text-xs font-medium text-[var(--muted)] uppercase tracking-wide">
        {label}
      </label>
      <div className="relative">
        <input
          type={visible ? "text" : "password"}
          value={value}
          onChange={e => onChange?.(e.target.value)}
          placeholder={placeholder}
          readOnly={readOnly}
          autoFocus={autoFocus}
          autoComplete="new-password"
          spellCheck={false}
          autoCorrect="off"
          autoCapitalize="off"
          className="w-full bg-[var(--bg)] border border-[var(--border)] rounded-lg px-3 py-2 pr-10
                     text-[var(--text)] placeholder-[var(--muted)] text-sm
                     focus:outline-none focus:border-[var(--accent)] transition-colors"
        />
        <button
          type="button"
          onClick={toggleVisible}
          className="absolute right-2 top-1/2 -translate-y-1/2 text-[var(--muted)]
                     hover:text-[var(--text)] transition-colors text-xs select-none"
          tabIndex={-1}
        >
          {visible ? "hide" : "show"}
        </button>
      </div>
    </div>
  );
}
