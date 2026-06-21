export type ThemeMode = "dark" | "light" | "system";

const STORAGE_THEME = "lspv_theme";
const STORAGE_ACCENT = "lspv_accent";
const DEFAULT_ACCENT_DARK = "#6366f1";
const DEFAULT_ACCENT_LIGHT = "#6366f1";

function systemPrefersDark(): boolean {
  return window.matchMedia("(prefers-color-scheme: dark)").matches;
}

function resolveTheme(mode: ThemeMode): "dark" | "light" {
  if (mode === "system") return systemPrefersDark() ? "dark" : "light";
  return mode;
}

export function applyTheme(mode: ThemeMode, accent: string): void {
  const resolved = resolveTheme(mode);
  document.documentElement.setAttribute("data-theme", resolved);
  document.documentElement.style.setProperty("--accent", accent);
  // derive hover from accent (slightly brighter/darker)
  document.documentElement.style.setProperty("--accent-hover", accent);
}

export function loadAndApplyTheme(): void {
  const mode = (localStorage.getItem(STORAGE_THEME) as ThemeMode) ?? "dark";
  const defaultAccent =
    resolveTheme(mode) === "light" ? DEFAULT_ACCENT_LIGHT : DEFAULT_ACCENT_DARK;
  const accent = localStorage.getItem(STORAGE_ACCENT) ?? defaultAccent;
  applyTheme(mode, accent);
}

export function getThemeMode(): ThemeMode {
  return (localStorage.getItem(STORAGE_THEME) as ThemeMode) ?? "dark";
}

export function getAccentColor(): string {
  const mode = getThemeMode();
  const defaultAccent =
    resolveTheme(mode) === "light" ? DEFAULT_ACCENT_LIGHT : DEFAULT_ACCENT_DARK;
  return localStorage.getItem(STORAGE_ACCENT) ?? defaultAccent;
}

export function saveTheme(mode: ThemeMode, accent: string): void {
  localStorage.setItem(STORAGE_THEME, mode);
  localStorage.setItem(STORAGE_ACCENT, accent);
  applyTheme(mode, accent);
}
