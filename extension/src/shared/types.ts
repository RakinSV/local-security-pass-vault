export interface ItemSummary {
  id: string;
  itemType: "login" | "card" | "note" | "identity" | "ssh_key";
  title: string;
  url?: string;
  username?: string;
  favorite: boolean;
}

export interface Credentials {
  username: string;
  password: string;
}

export interface VaultStatus {
  isLocked: boolean;
  itemCount: number;
  /** Desktop Ed25519 public key hex — returned on every status response for TOFU pairing. */
  signingPublicKey?: string;
  /** True when the vault has 2FA enabled — get_credentials will require a TOTP code. */
  has2fa?: boolean;
}

// Native messaging protocol (extension ↔ native host ↔ Tauri pipe)
export interface NativeRequest {
  id: string;
  action: "status" | "search" | "get_credentials" | "lock";
  payload?: unknown;
  profileId?: string;
  profileEmail?: string | null;
  browserType?: string;
  totpCode?: string;
}

export interface NativeResponse {
  id: string;
  success: boolean;
  data?: unknown;
  error?: string;
  signature?: string; // Ed25519 hex-encoded
}

// Messages between popup/content scripts ↔ background service worker
export type ExtensionMessage =
  | { type: "GET_STATUS" }
  | { type: "SEARCH"; query: string; pageUrl: string }
  | { type: "GET_CREDENTIALS"; itemId: string; totpCode?: string }
  | { type: "LOCK" }
  | { type: "FILL"; username: string; password: string }
  | { type: "FILL_PASSWORD_ONLY"; password: string }
  | { type: "DETECT_FORM"; url: string }
  | { type: "FILL_FROM_PROMPT"; itemId: string };

export type ExtensionResponse =
  | { ok: true; data: unknown }
  | { ok: false; error: string };
