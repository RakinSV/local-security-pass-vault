import { invoke } from "@tauri-apps/api/core";
import type { BrowserConfig, ImportRow, Item, ItemPayload, ItemSummary, VaultStatus } from "../types/vault";

export const vaultStatus = () =>
  invoke<VaultStatus>("vault_status");

export const getDefaultVaultDir = () =>
  invoke<string>("get_default_vault_dir");

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
  favorite: boolean
) =>
  invoke<string>("create_item", {
    title,
    payload,
    folderId,
    favorite,
  });

export const updateItem = (
  id: string,
  title: string,
  payload: ItemPayload,
  folderId: string | null,
  favorite: boolean
) =>
  invoke<void>("update_item", { id, title, payload, folderId, favorite });

export const deleteItem = (id: string) =>
  invoke<void>("delete_item", { id });

export const changeMasterPassword = (
  oldPassword: string,
  newPassword: string
) =>
  invoke<void>("change_master_password", { oldPassword, newPassword });

export const getBrowserIntegrations = () =>
  invoke<BrowserConfig>("get_browser_integrations");

export const saveBrowserIntegrations = (chromeIds: string[], firefoxIds: string[]) =>
  invoke<string>("save_browser_integrations", { chromeIds, firefoxIds });

export const getNativeHostPath = () =>
  invoke<string | null>("get_native_host_path");

export const parseImportCsv = (content: string) =>
  invoke<ImportRow[]>("parse_import_csv", { content });

export const importItemsFromCsv = (items: ImportRow[]) =>
  invoke<number>("import_items_from_csv", { items });
