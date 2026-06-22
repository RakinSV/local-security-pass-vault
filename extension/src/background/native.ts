import type { NativeRequest, NativeResponse, VaultStatus } from "../shared/types";
import { getProfileInfo } from "./profile";

const HOST_NAME = "com.vaultpass.native";

let port: chrome.runtime.Port | null = null;
const pending = new Map<
  string,
  { resolve: (r: NativeResponse) => void; reject: (e: unknown) => void }
>();

function getPort(): chrome.runtime.Port {
  if (!port) {
    port = chrome.runtime.connectNative(HOST_NAME);

    port.onMessage.addListener((msg: NativeResponse) => {
      const cb = pending.get(msg.id);
      if (cb) {
        pending.delete(msg.id);
        cb.resolve(msg);
      }
    });

    port.onDisconnect.addListener(() => {
      const err = chrome.runtime.lastError?.message ?? "Native host disconnected";
      port = null;
      for (const cb of pending.values()) {
        cb.reject(new Error(err));
      }
      pending.clear();
    });
  }
  return port;
}

// ── Ed25519 TOFU signature verification ───────────────────────────────────────

const SIGN_PK_STORAGE_KEY = "vaultpass_signing_pk_hex";

/** Decode hex string → ArrayBuffer (avoids Uint8Array<ArrayBufferLike> type issue with SubtleCrypto). */
function hexToBuffer(hex: string): ArrayBuffer {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.slice(i, i + 2), 16);
  }
  return bytes.buffer as ArrayBuffer;
}

/**
 * TOFU: store the desktop's signing public key on first sight.
 * Subsequent calls are no-ops — the stored key is never overwritten automatically.
 */
async function storeSigningKeyIfNew(pkHex: string): Promise<void> {
  const stored = await chrome.storage.local.get(SIGN_PK_STORAGE_KEY);
  if (!stored[SIGN_PK_STORAGE_KEY]) {
    await chrome.storage.local.set({ [SIGN_PK_STORAGE_KEY]: pkHex });
  }
}

/**
 * Verify the Ed25519 signature on a response against the stored TOFU key.
 * Silently skips if: no signature present, no stored key yet, or browser
 * doesn't support Ed25519 Web Crypto (Firefox < 130, Chrome < 113).
 * Throws if verification fails — caller should treat the response as tainted.
 */
async function verifyResponseSignature(response: NativeResponse): Promise<void> {
  if (!response.signature) return;

  const stored = await chrome.storage.local.get(SIGN_PK_STORAGE_KEY);
  const pkHex = stored[SIGN_PK_STORAGE_KEY] as string | undefined;
  // TOFU grace period: on first run the key hasn't been stored yet, so we accept
  // the unsigned response and let storeSigningKeyIfNew() pin it below.  After the
  // first successful response the key is pinned and all future responses must be
  // signed — an attacker who does not control the local machine cannot intercept
  // the first response without also controlling the pipe or native host binary.
  if (!pkHex) return;

  try {
    const cryptoKey = await crypto.subtle.importKey(
      "raw",
      hexToBuffer(pkHex),
      { name: "Ed25519" },
      false,
      ["verify"]
    );
    const msgBuf = new TextEncoder()
      .encode(response.id + JSON.stringify(response.data))
      .buffer as ArrayBuffer;
    const valid = await crypto.subtle.verify(
      "Ed25519",
      cryptoKey,
      hexToBuffer(response.signature),
      msgBuf
    );
    if (!valid) {
      throw new Error("IPC signature invalid — response may be tampered");
    }
  } catch (e) {
    // Re-throw verification failures; silently ignore algorithm-not-supported.
    if ((e as DOMException)?.name === "NotSupportedError") return;
    throw e;
  }
}

// ── Public API ─────────────────────────────────────────────────────────────────

export async function sendNative(
  action: NativeRequest["action"],
  payload?: unknown,
  totpCode?: string,
): Promise<NativeResponse> {
  const profile = await getProfileInfo();

  const response = await new Promise<NativeResponse>((resolve, reject) => {
    const id = crypto.randomUUID();
    pending.set(id, { resolve, reject });

    const timer = setTimeout(() => {
      if (pending.has(id)) {
        pending.delete(id);
        reject(new Error("Native request timed out"));
      }
    }, 10_000);

    const req: NativeRequest = {
      id,
      action,
      payload,
      profileId: profile.profileId,
      profileEmail: profile.profileEmail,
      browserType: profile.browserType,
      totpCode,
    };

    try {
      getPort().postMessage(req);
      // timer auto-expires: pending.has(id) check prevents double-reject on success
    } catch (e) {
      clearTimeout(timer);
      pending.delete(id);
      reject(e);
    }
  });

  // TOFU: on first status response, persist the desktop's signing public key.
  if (action === "status" && response.success) {
    const data = response.data as VaultStatus | undefined;
    if (data?.signingPublicKey) {
      await storeSigningKeyIfNew(data.signingPublicKey);
    }
  }

  // Verify Ed25519 signature on every signed response.
  await verifyResponseSignature(response);

  return response;
}
