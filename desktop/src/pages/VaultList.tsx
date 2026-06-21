import { useState, useEffect, useMemo } from "react";
import { ItemCard } from "../components/ItemCard";
import { listItems, lockVault, listFolders, addFolder, deleteFolder, deleteItem } from "../api/vault";
import type { FolderInfo } from "../api/vault";
import type { ItemSummary, ItemType } from "../types/vault";

const TYPE_ICONS: Record<ItemType, string> = {
  login: "🔑", card: "💳", note: "📄", identity: "👤", ssh_key: "🖥", server: "🖧",
};

interface Props {
  onSelectItem: (id: string) => void;
  onAddItem: () => void;
  onLocked: () => void;
  onSwitchVault: () => void;
  onSettings: () => void;
  refreshKey: number;
}

export function VaultList({ onSelectItem, onAddItem, onLocked, onSwitchVault, onSettings, refreshKey }: Props) {
  const [items, setItems]               = useState<ItemSummary[]>([]);
  const [folders, setFolders]           = useState<FolderInfo[]>([]);
  const [query, setQuery]               = useState("");
  const [filter, setFilter]             = useState<ItemType | "all">("all");
  const [tagFilter, setTagFilter]       = useState<string | null>(null);
  const [folderFilter, setFolderFilter] = useState<string | null>(null);
  const [showFavOnly, setShowFavOnly]   = useState(false);
  const [loading, setLoading]           = useState(true);
  const [loadError, setLoadError]       = useState("");

  // Folder management
  const [newFolderName, setNewFolderName]   = useState("");
  const [showFolderInput, setShowFolderInput] = useState(false);
  const [folderError, setFolderError]       = useState("");

  // Bulk select for trash
  const [selectMode, setSelectMode]   = useState(false);
  const [selected, setSelected]       = useState<Set<string>>(new Set());
  const [trashingBulk, setTrashingBulk] = useState(false);

  function load() {
    setLoading(true);
    setLoadError("");
    Promise.all([listItems(), listFolders()])
      .then(([its, fols]) => { setItems(its); setFolders(fols); })
      .catch(err => setLoadError(String(err)))
      .finally(() => setLoading(false));
  }

  useEffect(() => { load(); }, [refreshKey]);

  const tags = useMemo(() => {
    const s = new Set<string>();
    for (const i of items) if (i.sourceTag) s.add(i.sourceTag);
    return [...s].sort();
  }, [items]);

  const displayed = useMemo(() => {
    let result = items.filter(i => !i.folderId || true); // show all if no folder filter
    if (showFavOnly) result = result.filter(i => i.favorite);
    if (filter !== "all") result = result.filter(i => i.itemType === filter);
    if (tagFilter) result = result.filter(i => i.sourceTag === tagFilter);
    if (folderFilter) result = result.filter(i => i.folderId === folderFilter);
    if (query.trim()) {
      const q = query.toLowerCase();
      result = result.filter(i =>
        i.title.toLowerCase().includes(q) ||
        (i.subtitle ?? "").toLowerCase().includes(q)
      );
    }
    return result.sort((a, b) => b.updatedAt - a.updatedAt);
  }, [items, filter, tagFilter, folderFilter, showFavOnly, query]);

  async function handleLock() {
    await lockVault().catch(() => {});
    onLocked();
  }

  async function handleAddFolder() {
    const name = newFolderName.trim();
    if (!name) return;
    setFolderError("");
    try {
      await addFolder(name);
      setNewFolderName("");
      setShowFolderInput(false);
      load();
    } catch (err) { setFolderError(String(err)); }
  }

  async function handleDeleteFolder(id: string, name: string) {
    if (!confirm(`Delete folder "${name}"? Items inside will be moved to root.`)) return;
    try {
      await deleteFolder(id);
      if (folderFilter === id) setFolderFilter(null);
      load();
    } catch (err) { alert(String(err)); }
  }

  function toggleSelect(id: string, checked: boolean) {
    setSelected(prev => {
      const next = new Set(prev);
      if (checked) next.add(id);
      else next.delete(id);
      return next;
    });
  }

  function toggleSelectAll() {
    if (selected.size === displayed.length) {
      setSelected(new Set());
    } else {
      setSelected(new Set(displayed.map(i => i.id)));
    }
  }

  async function handleBulkTrash() {
    if (selected.size === 0) return;
    if (!confirm(`Move ${selected.size} item(s) to trash?`)) return;
    setTrashingBulk(true);
    try {
      await Promise.all([...selected].map(id => deleteItem(id)));
      setSelected(new Set());
      setSelectMode(false);
      load();
    } catch (err) { alert(String(err)); }
    finally { setTrashingBulk(false); }
  }

  return (
    <div className="flex h-screen bg-[var(--bg)]">
      {/* Sidebar */}
      <div className="w-48 flex-shrink-0 border-r border-[var(--border)] flex flex-col overflow-hidden">
        <div className="px-3 py-3 border-b border-[var(--border)]">
          <div className="text-xs font-semibold text-[var(--muted)] uppercase tracking-wider px-1">
            LSPV
          </div>
        </div>

        <nav className="flex-1 px-2 py-2 overflow-y-auto flex flex-col gap-0.5">
          {/* All items */}
          <button
            onClick={() => { setFilter("all"); setTagFilter(null); setFolderFilter(null); setShowFavOnly(false); }}
            className={`w-full text-left px-3 py-1.5 rounded-lg text-xs transition-colors
              ${ !showFavOnly && filter === "all" && !tagFilter && !folderFilter
                ? "bg-[var(--accent)]/20 text-[var(--accent)]"
                : "text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface)]"
              }`}
          >
            🔒 All items
          </button>

          {/* Favourites */}
          <button
            onClick={() => { setShowFavOnly(true); setFilter("all"); setTagFilter(null); setFolderFilter(null); }}
            className={`w-full text-left px-3 py-1.5 rounded-lg text-xs transition-colors
              ${ showFavOnly
                ? "bg-[var(--accent)]/20 text-[var(--accent)]"
                : "text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface)]"
              }`}
          >
            ★ Favourites
          </button>

          {/* Type filters */}
          <div className="mt-2 mb-1">
            <div className="text-[10px] font-semibold text-[var(--muted)] uppercase tracking-wider px-3 mb-1">
              Types
            </div>
            {(Object.entries(TYPE_ICONS) as [ItemType, string][]).map(([type, icon]) => {
              const count = items.filter(i => i.itemType === type).length;
              if (count === 0) return null;
              return (
                <button
                  key={type}
                  onClick={() => { setFilter(type); setTagFilter(null); setFolderFilter(null); setShowFavOnly(false); }}
                  className={`w-full text-left px-3 py-1.5 rounded-lg text-xs transition-colors flex items-center gap-1.5
                    ${ filter === type && !showFavOnly && !tagFilter && !folderFilter
                      ? "bg-[var(--accent)]/20 text-[var(--accent)]"
                      : "text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface)]"
                    }`}
                >
                  <span>{icon}</span>
                  <span className="flex-1 truncate capitalize">{type.replace("_", " ")}</span>
                  <span className="text-[10px] opacity-60">{count}</span>
                </button>
              );
            })}
          </div>

          {/* Folders */}
          <div className="mt-2 mb-1">
            <div className="flex items-center justify-between px-3 mb-1">
              <div className="text-[10px] font-semibold text-[var(--muted)] uppercase tracking-wider">
                Folders
              </div>
              <button
                onClick={() => { setShowFolderInput(v => !v); setFolderError(""); setNewFolderName(""); }}
                title="Create folder"
                className="text-[var(--muted)] hover:text-[var(--accent)] transition-colors text-sm leading-none"
              >
                +
              </button>
            </div>

            {showFolderInput && (
              <div className="px-2 mb-1.5 flex flex-col gap-1">
                <div className="flex gap-1">
                  <input
                    autoFocus
                    type="text"
                    value={newFolderName}
                    onChange={e => setNewFolderName(e.target.value)}
                    onKeyDown={e => {
                      if (e.key === "Enter") { e.preventDefault(); handleAddFolder(); }
                      if (e.key === "Escape") { setShowFolderInput(false); setFolderError(""); }
                    }}
                    placeholder="Folder name…"
                    className="flex-1 min-w-0 bg-[var(--bg)] border border-[var(--border)] rounded px-2 py-1
                               text-xs text-[var(--text)] placeholder-[var(--muted)]
                               focus:outline-none focus:border-[var(--accent)]"
                  />
                  <button
                    onClick={handleAddFolder}
                    className="px-2 py-1 bg-[var(--accent)] text-white text-xs rounded hover:bg-[var(--accent-hover)] transition-colors"
                  >
                    ✓
                  </button>
                </div>
                {folderError && <div className="text-[10px] text-[var(--danger)]">{folderError}</div>}
              </div>
            )}

            {folders.map(folder => (
              <div key={folder.id} className="group flex items-center gap-0.5">
                <button
                  onClick={() => {
                    setFolderFilter(folder.id === folderFilter ? null : folder.id);
                    setTagFilter(null); setFilter("all"); setShowFavOnly(false);
                  }}
                  className={`flex-1 min-w-0 text-left px-3 py-1.5 rounded-lg text-xs transition-colors truncate
                    ${ folderFilter === folder.id
                      ? "bg-[var(--accent)]/20 text-[var(--accent)]"
                      : "text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface)]"
                    }`}
                >
                  {folder.icon ?? "📁"} {folder.name}
                </button>
                <button
                  onClick={() => handleDeleteFolder(folder.id, folder.name)}
                  title="Delete folder"
                  className="opacity-0 group-hover:opacity-100 flex-shrink-0 text-[var(--muted)]
                             hover:text-[var(--danger)] transition-all text-xs px-1"
                >
                  ×
                </button>
              </div>
            ))}
          </div>

          {/* Tags */}
          {tags.length > 0 && (
            <div className="mt-2 mb-1">
              <div className="text-[10px] font-semibold text-[var(--muted)] uppercase tracking-wider px-3 mb-1">
                Tags
              </div>
              {tags.map(tag => (
                <button
                  key={tag}
                  onClick={() => { setTagFilter(tag === tagFilter ? null : tag); setFilter("all"); setFolderFilter(null); setShowFavOnly(false); }}
                  className={`w-full text-left px-3 py-1.5 rounded-lg text-xs transition-colors truncate
                    ${ tagFilter === tag
                      ? "bg-[var(--accent)]/20 text-[var(--accent)]"
                      : "text-[var(--muted)] hover:text-[var(--text)] hover:bg-[var(--surface)]"
                    }`}
                >
                  # {tag}
                </button>
              ))}
            </div>
          )}
        </nav>

        <div className="px-2 pt-2 border-t border-[var(--border)] flex flex-col gap-1">
          <button onClick={onSwitchVault}
            className="w-full text-left px-3 py-2 rounded-lg text-sm text-[var(--muted)]
                       hover:text-[var(--text)] hover:bg-[var(--surface)] transition-colors">
            ⇄ Switch vault
          </button>
          <button onClick={onSettings}
            className="w-full text-left px-3 py-2 rounded-lg text-sm text-[var(--muted)]
                       hover:text-[var(--text)] hover:bg-[var(--surface)] transition-colors">
            ⚙ Settings
          </button>
          <button onClick={handleLock}
            className="w-full text-left px-3 py-2 rounded-lg text-sm text-[var(--muted)]
                       hover:text-[var(--danger)] hover:bg-red-950/20 transition-colors">
            🔒 Lock
          </button>
        </div>
      </div>

      {/* Main content */}
      <div className="flex-1 flex flex-col overflow-hidden">
        {/* Top bar */}
        <div className="flex items-center gap-2 px-3 py-2.5 border-b border-[var(--border)]">
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
            onClick={() => {
              setSelectMode(v => { if (v) setSelected(new Set()); return !v; });
            }}
            title={selectMode ? "Cancel selection" : "Select items"}
            className={`px-2.5 py-1.5 rounded-lg border text-xs transition-colors flex-shrink-0
              ${ selectMode
                ? "border-[var(--accent)] bg-[var(--accent)]/10 text-[var(--accent)]"
                : "border-[var(--border)] text-[var(--muted)] hover:border-[var(--accent)] hover:text-[var(--accent)]"
              }`}
          >
            ✓ Select
          </button>
          <button
            onClick={onAddItem}
            className="bg-[var(--accent)] hover:bg-[var(--accent-hover)] text-white
                       text-sm font-medium px-3 py-1.5 rounded-lg transition-colors flex-shrink-0"
          >
            + Add
          </button>
        </div>

        {/* Bulk action bar */}
        {selectMode && (
          <div className="flex items-center gap-3 px-3 py-2 bg-[var(--surface)] border-b border-[var(--border)]">
            <label className="flex items-center gap-2 text-xs text-[var(--muted)] cursor-pointer select-none">
              <input
                type="checkbox"
                checked={selected.size === displayed.length && displayed.length > 0}
                onChange={toggleSelectAll}
                className="w-4 h-4 accent-[var(--accent)]"
              />
              Select all ({displayed.length})
            </label>
            <div className="flex-1" />
            {selected.size > 0 && (
              <button
                onClick={handleBulkTrash}
                disabled={trashingBulk}
                className="px-3 py-1.5 bg-red-900/30 border border-red-800/50 text-red-400
                           hover:bg-red-900/50 rounded-lg text-xs transition-colors disabled:opacity-40"
              >
                🗑 Move {selected.size} to trash
              </button>
            )}
          </div>
        )}

        {/* Item list */}
        <div className="flex-1 overflow-y-auto py-1">
          {loadError ? (
            <div className="m-3 text-[var(--danger)] text-sm bg-red-950/30 border border-red-900/40 rounded-lg px-3 py-2">
              {loadError}
            </div>
          ) : loading ? (
            <div className="flex items-center justify-center h-32 text-[var(--muted)] text-sm">Loading…</div>
          ) : displayed.length === 0 ? (
            <div className="flex flex-col items-center justify-center h-32 gap-2">
              <div className="text-3xl">{showFavOnly ? "★" : "🔍"}</div>
              <div className="text-[var(--muted)] text-sm">
                {showFavOnly ? "No favourites yet" : query ? "No results" : "No items yet"}
              </div>
            </div>
          ) : (
            <div className="px-1">
              {displayed.map(item => (
                <ItemCard
                  key={item.id}
                  item={item}
                  onClick={() => { if (!selectMode) onSelectItem(item.id); else toggleSelect(item.id, !selected.has(item.id)); }}
                  selected={selectMode ? selected.has(item.id) : undefined}
                  onSelect={selectMode ? toggleSelect : undefined}
                />
              ))}
            </div>
          )}
        </div>

        <div className="px-4 py-1.5 border-t border-[var(--border)] text-xs text-[var(--muted)]">
          {displayed.length} item{displayed.length !== 1 ? "s" : ""}
          {selectMode && selected.size > 0 && ` · ${selected.size} selected`}
        </div>
      </div>
    </div>
  );
}
