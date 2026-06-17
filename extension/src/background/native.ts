import type { NativeRequest, NativeResponse } from "../shared/types";
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

export async function sendNative(
  action: NativeRequest["action"],
  payload?: unknown
): Promise<NativeResponse> {
  // Attach profile context so the Tauri app can identify which Chrome profile is talking
  const profile = await getProfileInfo();

  return new Promise((resolve, reject) => {
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
    };

    try {
      getPort().postMessage(req);
    } catch (e) {
      clearTimeout(timer);
      pending.delete(id);
      reject(e);
    }
  });
}
