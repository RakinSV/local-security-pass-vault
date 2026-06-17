export type BrowserType = "chrome" | "firefox" | "edge";

export interface ProfileInfo {
  profileId: string;
  profileEmail: string | null;
  browserType: BrowserType;
}

function detectBrowserType(): BrowserType {
  // chrome.runtime.getBrowserInfo exists only in Firefox (MV3 109+)
  if (typeof (chrome.runtime as Record<string, unknown>).getBrowserInfo === "function") {
    return "firefox";
  }
  if (navigator.userAgent.includes("Edg/")) return "edge";
  return "chrome";
}

let cached: ProfileInfo | null = null;

export async function getProfileInfo(): Promise<ProfileInfo> {
  if (cached) return cached;

  const STORAGE_KEY = "vaultpass_profile_id";
  const stored = await chrome.storage.local.get(STORAGE_KEY);

  let profileId: string;
  if (typeof stored[STORAGE_KEY] === "string") {
    profileId = stored[STORAGE_KEY] as string;
  } else {
    profileId = crypto.randomUUID();
    await chrome.storage.local.set({ [STORAGE_KEY]: profileId });
  }

  // Google account email — only available in Chrome/Edge, not Firefox
  let profileEmail: string | null = null;
  try {
    const info = await new Promise<chrome.identity.ProfileUserInfo>((resolve) => {
      chrome.identity.getProfileUserInfo({ accountStatus: "ANY" }, resolve);
    });
    profileEmail = info.email || null;
  } catch {
    // identity API unavailable (Firefox, or permission not granted)
  }

  cached = { profileId, profileEmail, browserType: detectBrowserType() };
  return cached;
}

/** Short display label for the profile: email, or null for local profiles. */
export async function getProfileLabel(): Promise<string | null> {
  const { profileEmail } = await getProfileInfo();
  return profileEmail;
}
