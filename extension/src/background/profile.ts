export interface ProfileInfo {
  profileId: string;
  profileEmail: string | null;
}

let cached: ProfileInfo | null = null;

export async function getProfileInfo(): Promise<ProfileInfo> {
  if (cached) return cached;

  // Per-profile UUID stored in chrome.storage.local (isolated per Chrome profile)
  const STORAGE_KEY = "vaultpass_profile_id";
  const stored = await chrome.storage.local.get(STORAGE_KEY);

  let profileId: string;
  if (typeof stored[STORAGE_KEY] === "string") {
    profileId = stored[STORAGE_KEY] as string;
  } else {
    profileId = crypto.randomUUID();
    await chrome.storage.local.set({ [STORAGE_KEY]: profileId });
  }

  // Try to get signed-in Google account email (empty string if not signed in)
  let profileEmail: string | null = null;
  try {
    const info = await new Promise<chrome.identity.ProfileUserInfo>((resolve) => {
      chrome.identity.getProfileUserInfo({ accountStatus: "ANY" }, resolve);
    });
    profileEmail = info.email || null;
  } catch {
    // identity API unavailable in this context
  }

  cached = { profileId, profileEmail };
  return cached;
}

/** Short display label for the profile: email, or null for local profiles. */
export async function getProfileLabel(): Promise<string | null> {
  const { profileEmail } = await getProfileInfo();
  return profileEmail;
}
