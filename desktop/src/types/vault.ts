export type ItemType = "login" | "card" | "note" | "identity" | "ssh_key" | "server";

export interface CustomField {
  label: string;
  value: string;
  hidden: boolean;
}

export interface PasswordHistoryEntry {
  password: string;
  changed_at: number;
}

export interface LoginPayload {
  type: "login";
  url: string;
  username: string;
  password: string;
  totp_secret: string | null;
  notes: string | null;
  custom_fields: CustomField[];
  password_history: PasswordHistoryEntry[];
}

export interface CardPayload {
  type: "card";
  cardholder: string;
  number: string;
  expiry_month: number;
  expiry_year: number;
  cvv: string;
  notes: string | null;
}

export interface NotePayload {
  type: "note";
  content: string;
}

export interface IdentityPayload {
  type: "identity";
  first_name: string | null;
  last_name: string | null;
  email: string | null;
  phone: string | null;
  address: string | null;
  passport: string | null;
  notes: string | null;
}

export interface SshKeyPayload {
  type: "ssh_key";
  private_key: string;
  public_key: string | null;
  passphrase: string | null;
  notes: string | null;
}

export interface ServerPayload {
  type: "server";
  host: string;
  port: number | null;
  username: string | null;
  /** "password" | "ssh_key" | "token" */
  auth_type: string;
  password: string | null;
  ssh_private_key: string | null;
  ssh_passphrase: string | null;
  token: string | null;
  notes: string | null;
}

export type ItemPayload =
  | LoginPayload
  | CardPayload
  | NotePayload
  | IdentityPayload
  | SshKeyPayload
  | ServerPayload;

export interface ItemSummary {
  id: string;
  itemType: ItemType;
  title: string;
  subtitle: string | null;
  folderId: string | null;
  favorite: boolean;
  updatedAt: number;
  sourceTag: string | null;
}

export interface Item extends ItemSummary {
  payload: ItemPayload;
  createdAt: number;
}

export interface VaultStatus {
  isLocked: boolean;
}

export interface BrowserConfig {
  chromeIds: string[];
  firefoxIds: string[];
}

export interface ImportRow {
  title: string;
  url: string;
  username: string;
  password: string;
}

export interface ProfileInfo {
  id: string;
  email: string | null;
  name: string | null;
  lastSeenMs: number;
  browserType: string | null;
}
