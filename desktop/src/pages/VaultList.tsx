import { useState, useEffect, useMemo } from "react";
import { ItemCard } from "../components/ItemCard";
import { listItems, lockVault } from "../api/vault";
import type { ItemSummary, ItemType } from "../types/vault";

const FILTERS: { label: string; value: ItemType | "all" }[] = [
  { label: "All", value: "all" },
  { label: "🔑 Logins", value: "login" },
  { label: "💳 Cards", value: "card" },
  { label: "📄 Notes", value: "note" },
  { label: "👤 Identities", value: "identity" },
  { label: "🖥 SSH Keys", value: "ssh_key" },
];

interface Props {
  onSelectItem: (id: string) => void;
  onAddItem: () => void;
  onLocked: () => void;
  onSettings: () => void;
  refreshKey: number;
}

export function VaultList({ onSelectItem, onAddItem, onLocked, onSettings, refreshKey }: Props) {
  const [items, setItems] = useState<ItemSummary[]>([]);
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<ItemType | "all">("all");
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    setLoading(true);
    listItems()
      .then(setItems)
      .catch(console.error)
      .finally(() => setLoading(false));
  }, [refreshKey]);

  const displayed = useMemo(() => {
    let result = items;
    if (filter !== "all") result = result.filter(i => i.itemType === filter);
    if (query.trim()) {
      const q = query.toLowerCase();
      result = result.filter(i => i.title.toLowerCase().includes(q));
    }
    return result.sort((a, b) => b.updatedAt - a.updatedAt);
  }, [items, filter, query]);

  async function handleLock() {
    await lockVault();
    onLocked();
  }

  return (
    <div className="flex h-screen overflow-hidden">
      {/* Sidebar */}
      <div className="w-44 flex-shrink-0 border-r border-[var(--border)] flex flex-col py-4">
        <div className="px-4 mb-4">
          <div className="text-sm font-bold text-[var(--text)]">🔐 VaultPass</div>
        </div>

        <nav className="flex-1 px-2">
          {FILTERS.map(f => (
            <button
              key={f.value}
              onClick={() => setFilter(f.value)}
              className={`w-full text-left px-3 py-2 rounded-lg text-sm mb-0.5 transition-colors
                ${filter === f.value
                  ? "bg-[var(--accent)]/20 text-[var(--accent)]"
                  : "text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface)]"
                }`}
            >
              {f.label}
            </button>
          ))}
        </nav>

        <div className="px-2 pt-2 border-t border-[var(--border)] flex flex-col gap-1">
          <button
            onClick={onSettings}
            className="w-full text-left px-3 py-2 rounded-lg text-sm text-[var(--muted)]
                       hover:text-[var(--text)] hover:bg-[var(--surface)] transition-colors"
          >
            ⚙ Settings
          </button>
          <button
            onClick={handleLock}
            className="w-full text-left px-3 py-2 rounded-lg text-sm text-[var(--muted)]
                       hover:text-[var(--danger)] hover:bg-red-950/20 transition-colors"
          >
            🔒 Lock
          </button>
        </div>
      </div>

      {/* Main content */}
      <div className="flex-1 flex flex-col overflow-hidden">
        {/* Toolbar */}
        <div className="flex items-center gap-3 px-4 py-3 border-b border-[var(--border)]">
          <input
            type="text"
            value={query}
            onChange={e => setQuery(e.target.value)}
            placeholder="Search…"
            className="flex-1 bg-[var(--surface)] border border-[var(--border)] rounded-lg px-3 py-1.5
                       text-sm text-[var(--text)] placeholder-[var(--muted)]
                       focus:outline-none focus:border-[var(--accent)] transition-colors"
          />
          <button
            onClick={onAddItem}
            className="bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white
                       text-sm font-medium px-4 py-1.5 rounded-lg transition-colors flex-shrink-0"
          >
            + Add
          </button>
        </div>

        {/* Item list */}
        <div className="flex-1 overflow-y-auto p-2">
          {loading ? (
            <div className="flex items-center justify-center h-32 text-[var(--muted)] text-sm">
              Loading…
            </div>
          ) : displayed.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-32 gap-2">
              <div className="text-3xl">🔍</div>
              <div className="text-[var(--muted)] text-sm">
                {query ? "No results" : "No items yet"}
              </div>
            </div>
          ) : (
            displayed.map(item => (
              <ItemCard
                key={item.id}
                item={item}
                onClick={() => onSelectItem(item.id)}
              />
            ))
          )}
        </div>

        <div className="px-4 py-2 border-t border-[var(--border)] text-xs text-[var(--muted)]">
          {displayed.length} item{displayed.length !== 1 ? "s" : ""}
        </div>
      </div>
    </div>
  );
}
