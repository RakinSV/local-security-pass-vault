interface Props {
  onSetup: () => void;
  onUnlock: () => void;
}

export function Welcome({ onSetup, onUnlock }: Props) {
  return (
    <div className="flex flex-col items-center justify-center h-screen gap-8 px-6">
      <div className="text-center">
        <div className="text-5xl mb-4">🔐</div>
        <h1 className="text-3xl font-bold text-[var(--text)] mb-2">VaultPass</h1>
        <p className="text-[var(--muted)] text-sm">
          Local password manager. No cloud, no telemetry.
        </p>
      </div>

      <div className="flex flex-col gap-3 w-full max-w-xs">
        <button
          onClick={onSetup}
          className="w-full bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white
                     font-medium py-3 px-6 rounded-xl transition-colors"
        >
          Create new vault
        </button>
        <button
          onClick={onUnlock}
          className="w-full bg-[var(--surface)] hover:bg-[var(--border)] text-[var(--text)]
                     font-medium py-3 px-6 rounded-xl border border-[var(--border)] transition-colors"
        >
          Open existing vault
        </button>
      </div>
    </div>
  );
}
