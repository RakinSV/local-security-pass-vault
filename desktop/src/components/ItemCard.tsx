import type { ItemSummary, ItemType } from "../types/vault";

const TYPE_ICON: Record<ItemType, string> = {
  login:    "🔑",
  card:     "💳",
  note:     "📄",
  identity: "👤",
  ssh_key:  "🖥",
  server:   "🖧",
};

interface Props {
  item: ItemSummary;
  onClick: () => void;
  selected?: boolean;
  onSelect?: (id: string, checked: boolean) => void;
}

export function ItemCard({ item, onClick, selected, onSelect }: Props) {
  const subtitle = item.subtitle;

  return (
    <div className="flex items-center gap-1 group">
      {onSelect !== undefined && (
        <input
          type="checkbox"
          checked={selected ?? false}
          onChange={e => { e.stopPropagation(); onSelect(item.id, e.target.checked); }}
          onClick={e => e.stopPropagation()}
          className="ml-1 w-4 h-4 flex-shrink-0 accent-[var(--accent)] cursor-pointer"
        />
      )}
      <button
        onClick={onClick}
        className="flex-1 flex items-center gap-3 px-3 py-2.5 hover:bg-[var(--surface)]
                   rounded-lg transition-colors text-left min-w-0"
      >
        <span className="text-xl w-8 text-center flex-shrink-0">
          {TYPE_ICON[item.itemType]}
        </span>
        <div className="flex-1 min-w-0">
          <div className="font-medium text-[var(--text)] truncate leading-snug">{item.title}</div>
          {subtitle ? (
            <div className="text-xs text-[var(--muted)] truncate leading-snug mt-0.5">
              {subtitle}
              {item.sourceTag && (
                <span className="ml-1.5 text-[10px] px-1 py-0 rounded-full
                                  bg-[var(--accent)]/10 text-[var(--accent)]/70
                                  border border-[var(--accent)]/20">
                  {item.sourceTag}
                </span>
              )}
            </div>
          ) : (
            item.sourceTag && (
              <div className="text-xs text-[var(--muted)] mt-0.5">
                <span className="text-[10px] px-1.5 py-0 rounded-full
                                  bg-[var(--accent)]/10 text-[var(--accent)]/70
                                  border border-[var(--accent)]/20">
                  {item.sourceTag}
                </span>
              </div>
            )
          )}
        </div>
        {item.favorite && (
          <span className="text-yellow-400 text-xs flex-shrink-0">★</span>
        )}
      </button>
    </div>
  );
}
