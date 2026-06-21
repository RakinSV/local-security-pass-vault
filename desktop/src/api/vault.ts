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

export const openVault = (dirPath: string, password: string, totpCode?: string) =>
  invoke<void>("open_vault", { dirPath, password, totpCode: totpCode ?? null });

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

// ── Auto-lock ─────────────────────────────────────────────────────────────────

export interface AutoLockSettings {
  secs: number;
  lockOnMinimize: boolean;
  lockOnScreensaver: boolean;
}

export const getAutoLockSettings = () =>
  invoke<AutoLockSettings>("get_auto_lock_settings");

export const setAutoLockSettings = (
  secs: number,
  lockOnMinimize: boolean,
  lockOnScreensaver: boolean,
) => invoke<void>("set_auto_lock_settings", { secs, lockOnMinimize, lockOnScreensaver });

export const activityPing = () =>
  invoke<void>("activity_ping");

// ── Keychain vault status ─────────────────────────────────────────────────────

export interface KeychainVaultStatus {
  vaultOpen: boolean;
  vaultUuid: string | null;
  hasCachedKey: boolean;
}

export const keychainVaultStatus = () =>
  invoke<KeychainVaultStatus>("keychain_vault_status");

export const keychainDeleteKey = (vaultUuid: string) =>
  invoke<void>("keychain_delete_key", { vaultUuid });

// ── Auto-backup list ──────────────────────────────────────────────────────────

export interface AutoBackupEntry {
  path: string;
  sizeBytes: number;
  createdAt: number;
}

export const listAutoBackups = () =>
  invoke<AutoBackupEntry[]>("list_auto_backups");

export const pickBackupFile = () =>
  invoke<string | null>("pick_backup_file");

export const pickBackupSavePath = () =>
  invoke<string | null>("pick_backup_save_path");

// ── TOTP 2FA ──────────────────────────────────────────────────────────────────

export interface TotpCode {
  code: string;
  validForSecs: number;
  periodSecs: number;
}

export const generateTotp = (secret: string) =>
  invoke<TotpCode>("generate_totp", { secret });

export interface QrResult {
  secret: string;
  issuer: string;
  account: string;
  rawUri: string;
}

export const decodeQrFromClipboard = () =>
  invoke<QrResult>("decode_qr_from_clipboard");

// ── Browser Extension Installer ───────────────────────────────────────────────

export interface BrowserProfile {
  id: string;
  name: string;
  path: string;
}

export interface DetectedBrowser {
  id: string;
  name: string;
  installed: boolean;
  profiles: BrowserProfile[];
  supportsPerProfile: boolean;
}

export interface InstallRequest {
  browserId: string;
  profileIds: string[];
}

export interface InstallResult {
  browserId: string;
  success: boolean;
  message: string;
}

export const detectBrowsersForExtension = () =>
  invoke<DetectedBrowser[]>("detect_browsers_for_extension");

export const installExtensionToBrowsers = (requests: InstallRequest[]) =>
  invoke<InstallResult[]>("install_extension_to_browsers", { requests });

// ── Trash ──────────────────────────────────────────────────────────────────────

export const listDeletedItems = () =>
  invoke<ItemSummary[]>("list_deleted_items");

export const restoreItem = (id: string) =>
  invoke<void>("restore_item", { id });

export const purgeItem = (id: string) =>
  invoke<void>("purge_item", { id });

export const purgeAllTrash = () =>
  invoke<number>("purge_all_trash");

// ── Folders ────────────────────────────────────────────────────────────────────

export interface FolderInfo {
  id: string;
  name: string;
  icon: string | null;
  createdAt: number;
}

export const listFolders = () =>
  invoke<FolderInfo[]>("list_folders");

export const addFolder = (name: string, icon?: string) =>
  invoke<string>("add_folder", { name, icon: icon ?? null });

export const deleteFolder = (id: string) =>
  invoke<void>("delete_folder", { id });

export const renameFolder = (id: string, name: string) =>
  invoke<void>("rename_folder", { id, name });

export const importBitwardenJson = (content: string, sourceTag?: string) =>
  invoke<number>("import_bitwarden_json", { content, sourceTag: sourceTag ?? null });

// ── Password health ────────────────────────────────────────────────────────────

export interface HealthEntry {
  id: string;
  title: string;
  url: string;
  isWeak: boolean;
  isDuplicate: boolean;
  isOld: boolean;
  updatedAt: number;
}

export const getHealthReport = () =>
  invoke<HealthEntry[]>("get_health_report");

// ── Vault 2FA ─────────────────────────────────────────────────────────────────

export const vaultRequires2fa = (dirPath: string) =>
  invoke<boolean>("vault_requires_2fa", { dirPath });

export const vaultHas2fa = () =>
  invoke<boolean>("vault_has_2fa");

export interface VaultTwoFaSetup {
  secret: string;
  uri: string;
  qrSvg: string;
}

export const setupVault2fa = () =>
  invoke<VaultTwoFaSetup>("setup_vault_2fa");

export const confirmVault2fa = (secret: string, code: string) =>
  invoke<void>("confirm_vault_2fa", { secret, code });

export const disableVault2fa = (code: string) =>
  invoke<void>("disable_vault_2fa", { code });

// ── CSV Export ─────────────────────────────────────────────────────────────────

export const pickCsvSavePath = () =>
  invoke<string | null>("pick_csv_save_path");

export const exportItemsCsv = (path: string) =>
  invoke<void>("export_items_csv", { path });

// ── Screen capture protection ──────────────────────────────────────────────────

export const setScreenCaptureProtection = (enabled: boolean) =>
  invoke<void>("set_screen_capture_protection", { enabled });

// ── HaveIBeenPwned breach check ────────────────────────────────────────────────

export interface HibpResult {
  pwnedCount: number;
  checked: boolean;
}

export const checkPasswordBreach = (password: string) =>
  invoke<HibpResult>("check_password_breach", { password });

