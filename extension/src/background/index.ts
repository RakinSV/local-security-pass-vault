import { sendNative } from "./native";
import { getProfileLabel } from "./profile";
import { domainsMatch } from "../shared/domain";
import type { ExtensionMessage, ExtensionResponse, ItemSummary } from "../shared/types";

chrome.runtime.onMessage.addListener(
  (msg: ExtensionMessage, sender, sendResponse) => {
    // ── Content-script messages ────────────────────────────────────────────────

    if (msg.type === "DETECT_FORM") {
      const url = msg.url;
      (async () => {
        try {
          const [profileLabel, r] = await Promise.all([
            getProfileLabel(),
            sendNative("search", { query: "", pageUrl: url }),
          ]);
          if (!r.success || !r.data) {
            sendResponse({ ok: true, data: { items: [], profileLabel } });
            return;
          }
          const all = (r.data as ItemSummary[]) ?? [];
          // Keep only login items whose domain matches the current page
          const matched = all.filter(
            (it) =>
              it.itemType === "login" &&
              it.url != null &&
              domainsMatch(it.url, url)
          );
          sendResponse({ ok: true, data: { items: matched, profileLabel } });
        } catch {
          sendResponse({ ok: true, data: { items: [], profileLabel: null } });
        }
      })();
      return true; // async response
    }

    if (msg.type === "FILL_FROM_PROMPT") {
      const tabId = sender.tab?.id;
      if (tabId == null) return false;

      (async () => {
        try {
          const r = await sendNative("get_credentials", { itemId: msg.itemId });
          if (r.success && r.data) {
            const creds = r.data as { username: string; password: string };
            await chrome.tabs.sendMessage(tabId, {
              type: "FILL",
              username: creds.username,
              password: creds.password,
            });
          } else if (r.error === "TotpRequired") {
            // Vault 2FA is enabled — user must use the popup to enter a TOTP code.
            await chrome.tabs.sendMessage(tabId, { type: "TOTP_HINT" }).catch(() => {});
          }
        } catch {
          // best-effort
        }
      })();
      return false; // no response needed
    }

    // ── Popup messages ─────────────────────────────────────────────────────────

    handlePopupMessage(msg)
      .then((data) => sendResponse({ ok: true, data } satisfies ExtensionResponse))
      .catch((err) =>
        sendResponse({ ok: false, error: String(err) } satisfies ExtensionResponse)
      );
    return true;
  }
);

async function handlePopupMessage(msg: ExtensionMessage): Promise<unknown> {
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
      const r = await sendNative("get_credentials", { itemId: msg.itemId }, msg.totpCode);
      if (!r.success) {
        if (r.error === "TotpRequired") throw new Error("TotpRequired");
        if (r.error === "TotpInvalid")  throw new Error("TotpInvalid");
        throw new Error(r.error ?? "native error");
      }
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
