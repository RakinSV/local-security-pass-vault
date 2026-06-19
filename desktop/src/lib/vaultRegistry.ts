export interface VaultEntry {
  id: string;
  name: string;
  path: string;
  createdAt: number;
  lastOpened?: number;
}

const KEY = "lspv:vaults";

export function listVaults(): VaultEntry[] {
  try {
    return JSON.parse(localStorage.getItem(KEY) ?? "[]");
  } catch {
    return [];
  }
}

export function registerVault(name: string, path: string): VaultEntry {
  const all = listVaults();
  const existing = all.find(v => v.path === path);
  if (existing) {
    touchVault(existing.id);
    return existing;
  }
  const entry: VaultEntry = {
    id: crypto.randomUUID(),
    name,
    path,
    createdAt: Date.now(),
    lastOpened: Date.now(),
  };
  localStorage.setItem(KEY, JSON.stringify([...all, entry]));
  return entry;
}

export function touchVault(id: string): void {
  const all = listVaults().map(v =>
    v.id === id ? { ...v, lastOpened: Date.now() } : v
  );
  localStorage.setItem(KEY, JSON.stringify(all));
}

export function removeVault(id: string): void {
  localStorage.setItem(KEY, JSON.stringify(listVaults().filter(v => v.id !== id)));
}

export function sortedVaults(): VaultEntry[] {
  return listVaults().sort((a, b) => (b.lastOpened ?? b.createdAt) - (a.lastOpened ?? a.createdAt));
}
