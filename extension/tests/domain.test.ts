// Domain-matching unit tests (testing.md — Фаза 2, Extension tests).
// Run: npm test  (or: npx vitest run)
import { describe, it, expect } from "vitest";
import { domainsMatch } from "../src/shared/domain";

describe("domainsMatch — eTLD+1 comparison", () => {
  const cases: [string, string, boolean, string][] = [
    ["https://google.com",       "https://accounts.google.com",  true,  "поддомен совпадает"],
    ["https://google.com",       "https://google.com.evil.ru",   false, "омограф через поддомен"],
    ["https://paypal.com",       "https://paypa1.com",           false, "typosquat"],
    ["https://github.com",       "https://github.io",            false, "разные eTLD+1"],
    ["https://amazon.co.uk",     "https://www.amazon.co.uk",     true,  "cc-TLD поддомен"],
    ["https://bank.com/login",   "https://bank.com/dashboard",   true,  "разные пути, один домен"],
    ["https://evil.com",         "https://notevil.com",          false, "суффикс в имени"],
  ];

  it.each(cases)("%s  vs  %s  →  %s  (%s)", (vault, page, expected) => {
    expect(domainsMatch(vault, page)).toBe(expected);
  });
});
