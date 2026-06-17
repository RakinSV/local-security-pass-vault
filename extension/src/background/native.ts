import type { NativeRequest, NativeResponse } from "../shared/types";

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

export function sendNative(
  action: NativeRequest["action"],
  payload?: unknown
): Promise<NativeResponse> {
  return new Promise((resolve, reject) => {
    const id = crypto.randomUUID();
    pending.set(id, { resolve, reject });

    const timer = setTimeout(() => {
      if (pending.has(id)) {
        pending.delete(id);
        reject(new Error("Native request timed out"));
      }
    }, 10_000);

    try {
      getPort().postMessage({ id, action, payload } satisfies NativeRequest);
    } catch (e) {
      clearTimeout(timer);
      pending.delete(id);
      reject(e);
    }
  });
}
