import type { ItemSummary, ItemType } from "../types/vault";

const TYPE_ICON: Record<ItemType, string> = {
  login: "🔑",
  card: "💳",
  note: "📄",
  identity: "👤",
  ssh_key: "🖥",
};

const TYPE_LABEL: Record<ItemType, string> = {
  login: "Login",
  card: "Card",
  note: "Secure Note",
  identity: "Identity",
  ssh_key: "SSH Key",
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
        <div className="text-xs text-[var(--muted)]">{TYPE_LABEL[item.itemType]}</div>
      </div>
      {item.favorite && (
        <span className="text-yellow-400 text-xs flex-shrink-0">★</span>
      )}
    </button>
  );
}
