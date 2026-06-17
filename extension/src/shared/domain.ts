import { parse } from "tldts";

// Compare vault URL vs page URL using eTLD+1 (per browser-extension.md)
// google.com vs accounts.google.com → match
// google.com vs google.com.evil.ru  → no match
export function domainsMatch(vaultUrl: string, pageUrl: string): boolean {
  try {
    const vaultDomain = parse(vaultUrl).domain;
    const pageDomain = parse(pageUrl).domain;
    if (!vaultDomain || !pageDomain) return false;
    return vaultDomain === pageDomain;
  } catch {
    return false;
  }
}
