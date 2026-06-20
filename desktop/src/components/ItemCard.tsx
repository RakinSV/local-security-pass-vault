import type { ItemSummary, ItemType } from "../types/vault";

const TYPE_ICON: Record<ItemType, string> = {
  login:    "🔑",
  card:     "💳",
  note:     "📄",
  identity: "👤",
  ssh_key:  "🖥",
  server:   "🖧",
};

const TYPE_LABEL: Record<ItemType, string> = {
  login:    "Login",
  card:     "Card",
  note:     "Secure Note",
  identity: "Identity",
  ssh_key:  "SSH Key",
  server:   "Server",
};

interface Props {
  item: ItemSummary;
  onClick: () => void;
}

export function ItemCard({ item, onClick }: Props) {
  return (
    <button
      onClick={onClick}
      className="w-full flex items-center gap-3 px-4 py-3 hover:bg-[var(--surface)]
                 rounded-lg transition-colors text-left group"
    >
      <span className="text-xl w-8 text-center flex-shrink-0">
        {TYPE_ICON[item.itemType]}
      </span>
      <div className="flex-1 min-w-0">
        <div className="font-medium text-[var(--text)] truncate">{item.title}</div>
        <div className="text-xs text-[var(--muted)] flex items-center gap-1.5">
          {TYPE_LABEL[item.itemType]}
          {item.sourceTag && (
            <span className="text-[10px] px-1.5 py-0 rounded-full bg-[var(--accent)]/10 text-[var(--accent)]/70 border border-[var(--accent)]/20">
              {item.sourceTag}
            </span>
          )}
        </div>
      </div>
      {item.favorite && (
        <span className="text-yellow-400 text-xs flex-shrink-0">★</span>
      )}
    </button>
  );
}
