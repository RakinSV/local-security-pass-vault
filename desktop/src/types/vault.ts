export type ItemType = "login" | "card" | "note" | "identity" | "ssh_key";

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

export type ItemPayload =
  | LoginPayload
  | CardPayload
  | NotePayload
  | IdentityPayload
  | SshKeyPayload;

export interface ItemSummary {
  id: string;
  itemType: ItemType;
  title: string;
  folderId: string | null;
  favorite: boolean;
  updatedAt: number;
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
