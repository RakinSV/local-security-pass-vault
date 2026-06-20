import { invoke } from "@tauri-apps/api/core";
import type { BrowserConfig, ImportRow, Item, ItemPayload, ItemSummary, ProfileInfo, VaultStatus } from "../types/vault";

export const vaultStatus = () =>
  invoke<VaultStatus>("vault_status");

export const suggestVaultDir = (name: string) =>
  invoke<string>("suggest_vault_dir", { name });

export const pickFolder = () =>
  invoke<string | null>("pick_folder");

export const openGithub = () =>
  invoke<void>("open_github");

export const createVault = (dirPath: string, password: string, hint?: string) =>
  invoke<void>("create_vault", { dirPath, password, hint: hint ?? null });

export const openVault = (dirPath: string, password: string) =>
  invoke<void>("open_vault", { dirPath, password });

export const lockVault = () =>
  invoke<void>("lock_vault");

export const listItems = () =>
  invoke<ItemSummary[]>("list_items");

export const getItem = (id: string) =>
  invoke<Item>("get_item", { id });

export const createItem = (
  title: string,
  payload: ItemPayload,
  folderId: string | null,
  favorite: boolean,
  sourceTag: string | null = null,
) =>
  invoke<string>("create_item", { title, payload, folderId, favorite, sourceTag });

export const updateItem = (
  id: string,
  title: string,
  payload: ItemPayload,
  folderId: string | null,
  favorite: boolean,
  sourceTag: string | null = null,
) =>
  invoke<void>("update_item", { id, title, payload, folderId, favorite, sourceTag });

export const deleteItem = (id: string) =>
  invoke<void>("delete_item", { id });

export const changeMasterPassword = (oldPassword: string, newPassword: string) =>
  invoke<void>("change_master_password", { oldPassword, newPassword });

export const getBrowserIntegrations = () =>
  invoke<BrowserConfig>("get_browser_integrations");

export const saveBrowserIntegrations = (chromeIds: string[], firefoxIds: string[]) =>
  invoke<string>("save_browser_integrations", { chromeIds, firefoxIds });

export const getNativeHostPath = () =>
  invoke<string | null>("get_native_host_path");

export const parseImportCsv = (content: string) =>
  invoke<ImportRow[]>("parse_import_csv", { content });

export const importItemsFromCsv = (items: ImportRow[], sourceTag: string | null = null) =>
  invoke<number>("import_items_from_csv", { items, sourceTag });

export const listSourceTags = () =>
  invoke<string[]>("list_source_tags");

export const bulkRetagItems = (oldTag: string, newTag: string | null) =>
  invoke<number>("bulk_retag_items", { oldTag, newTag });

export const getProfiles = () =>
  invoke<ProfileInfo[]>("get_profiles");

export const setProfileName = (profileId: string, name: string | null) =>
  invoke<void>("set_profile_name", { profileId, name });

// ── Backup ────────────────────────────────────────────────────────────────────

export const generateSeedPhrase = () =>
  invoke<string>("generate_seed_phrase");

export const validateSeedPhrase = (phrase: string) =>
  invoke<boolean>("validate_seed_phrase", { phrase });

export const exportBackup = (backupPath: string, seedPhrase: string) =>
  invoke<void>("export_backup", { backupPath, seedPhrase });

export const restoreBackup = (backupPath: string, destDir: string, seedPhrase: string) =>
  invoke<void>("restore_backup", { backupPath, destDir, seedPhrase });

// ── Autostart ─────────────────────────────────────────────────────────────────

export const getAutostart = () =>
  invoke<boolean>("get_autostart");

export const setAutostart = (enable: boolean) =>
  invoke<void>("set_autostart", { enable });
