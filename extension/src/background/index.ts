import { sendNative } from "./native";
import type { ExtensionMessage, ExtensionResponse } from "../shared/types";

// Route messages from popup / content scripts to native host
chrome.runtime.onMessage.addListener(
  (msg: ExtensionMessage, _sender, sendResponse) => {
    handleMessage(msg)
      .then((data) => sendResponse({ ok: true, data } satisfies ExtensionResponse))
      .catch((err) =>
        sendResponse({ ok: false, error: String(err) } satisfies ExtensionResponse)
      );
    return true; // keep channel open for async response
  }
);

async function handleMessage(msg: ExtensionMessage): Promise<unknown> {
  switch (msg.type) {
    case "GET_STATUS": {
      const r = await sendNative("status");
      if (!r.success) throw new Error(r.error ?? "native error");
      return r.data;
    }
    case "SEARCH": {
      const r = await sendNative("search", {
        query: msg.query,
        pageUrl: msg.pageUrl,
      });
      if (!r.success) throw new Error(r.error ?? "native error");
      return r.data;
    }
    case "GET_CREDENTIALS": {
      const r = await sendNative("get_credentials", { itemId: msg.itemId });
      if (!r.success) throw new Error(r.error ?? "native error");
      return r.data;
    }
    case "LOCK": {
      await sendNative("lock");
      return null;
    }
    default:
      return null;
  }
}
