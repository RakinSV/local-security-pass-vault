import { useState, useEffect } from "react";
import {
  detectBrowsersForExtension,
  installExtensionToBrowsers,
} from "../api/vault";
import type { DetectedBrowser, InstallRequest, InstallResult } from "../api/vault";

const BROWSER_ICONS: Record<string, string> = {
  chrome:  "🔵",
  edge:    "🟦",
  brave:   "🦁",
  firefox: "🦊",
};

export function ExtensionInstaller() {
  const [browsers,     setBrowsers]     = useState<DetectedBrowser[]>([]);
  const [selected,     setSelected]     = useState<Set<string>>(new Set());
  const [ffProfiles,   setFfProfiles]   = useState<Record<string, Set<string>>>({}); // browserId → Set<profileId>
  const [loading,      setLoading]      = useState(true);
  const [installing,   setInstalling]   = useState(false);
  const [results,      setResults]      = useState<InstallResult[]>([]);
  const [detectError,  setDetectError]  = useState("");

  useEffect(() => {
    detectBrowsersForExtension()
      .then(bs => {
        setBrowsers(bs);
        // Pre-select all installed browsers; pre-select all Firefox profiles
        const sel = new Set<string>();
        const ffp: Record<string, Set<string>> = {};
        for (const b of bs) {
          if (b.installed) {
            sel.add(b.id);
            if (b.supportsPerProfile) {
              ffp[b.id] = new Set(b.profiles.map(p => p.id));
            }
          }
        }
        setSelected(sel);
        setFfProfiles(ffp);
      })
      .catch(e => setDetectError(String(e)))
      .finally(() => setLoading(false));
  }, []);

  function toggleBrowser(id: string) {
    setSelected(prev => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
    setResults([]);
  }

  function toggleProfile(browserId: string, profileId: string) {
    setFfProfiles(prev => {
      const current = new Set(prev[browserId] ?? []);
      if (current.has(profileId)) current.delete(profileId);
      else current.add(profileId);
      return { ...prev, [browserId]: current };
    });
    setResults([]);
  }

  async function handleInstall() {
    const requests: InstallRequest[] = [];
    for (const browserId of selected) {
      const browser = browsers.find(b => b.id === browserId);
      if (!browser) continue;
      requests.push({
        browserId,
        profileIds: browser.supportsPerProfile
          ? Array.from(ffProfiles[browserId] ?? [])
          : [],
      });
    }
    if (!requests.length) return;

    setInstalling(true);
    setResults([]);
    try {
      const res = await installExtensionToBrowsers(requests);
      setResults(res);
    } catch (e) {
      setResults([{ browserId: "error", success: false, message: String(e) }]);
    } finally {
      setInstalling(false);
    }
  }

  if (loading) {
    return <div className="text-sm text-[var(--muted)] py-2">Detecting browsers…</div>;
  }

  if (detectError) {
    return (
      <div className="text-sm text-[var(--danger)] bg-red-950/20 border border-red-900/30
                      rounded-xl px-3 py-2">
        Could not detect browsers: {detectError}
      </div>
    );
  }

  const installedBrowsers = browsers.filter(b => b.installed);
  if (!installedBrowsers.length) {
    return (
      <div className="text-sm text-[var(--muted)] italic">
        No supported browsers found (Chrome, Edge, Brave, Firefox).
      </div>
    );
  }

  const anySelected = selected.size > 0;

  return (
    <div className="flex flex-col gap-4">
      {/* Browser list */}
      <div className="flex flex-col gap-2">
        {installedBrowsers.map(browser => {
          const isSelected  = selected.has(browser.id);
          const icon        = BROWSER_ICONS[browser.id] ?? "🌐";
          const hasProfiles = browser.supportsPerProfile && browser.profiles.length > 0;

          return (
            <div key={browser.id}
              className={`rounded-xl border transition-colors overflow-hidden
                ${isSelected ? "border-[var(--accent)]" : "border-[var(--border)]"}`}
            >
              {/* Browser header */}
              <label className="flex items-center gap-3 px-4 py-3 cursor-pointer
                                bg-[var(--surface)] select-none">
                <input
                  type="checkbox"
                  checked={isSelected}
                  onChange={() => toggleBrowser(browser.id)}
                  className="w-4 h-4 accent-[var(--accent)]"
                />
                <span className="text-base">{icon}</span>
                <div className="flex-1">
                  <div className="text-sm font-medium text-[var(--text)]">{browser.name}</div>
                  {hasProfiles && (
                    <div className="text-[10px] text-[var(--muted)] mt-0.5">
                      {browser.profiles.length} profile{browser.profiles.length !== 1 ? "s" : ""} found
                    </div>
                  )}
                </div>
              </label>

              {/* Firefox profile picker */}
              {isSelected && hasProfiles && (
                <div className="border-t border-[var(--border)] px-4 py-2.5 flex flex-col gap-1.5
                                bg-[var(--bg)]">
                  <div className="text-[10px] text-[var(--muted)] uppercase tracking-wide font-medium mb-0.5">
                    Install to profiles:
                  </div>
                  {browser.profiles.map(profile => {
                    const isProfileSelected =
                      ffProfiles[browser.id]?.has(profile.id) ?? false;
                    return (
                      <label key={profile.id}
                        className="flex items-center gap-2.5 cursor-pointer select-none group">
                        <input
                          type="checkbox"
                          checked={isProfileSelected}
                          onChange={() => toggleProfile(browser.id, profile.id)}
                          className="w-3.5 h-3.5 accent-[var(--accent)]"
                        />
                        <div className="flex-1 min-w-0">
                          <span className="text-sm text-[var(--text)] group-hover:text-[var(--accent)]
                                          transition-colors">
                            {profile.name}
                          </span>
                          <span className="text-[10px] text-[var(--muted)] ml-2 font-mono">
                            {profile.id}
                          </span>
                        </div>
                      </label>
                    );
                  })}
                </div>
              )}
            </div>
          );
        })}
      </div>

      {/* Info note */}
      <div className="text-[10px] text-[var(--muted)] leading-relaxed">
        Chrome / Edge / Brave: registers via registry key — browser installs the extension on
        next launch.{" "}
        Firefox: copies the XPI into the selected profile's extensions/ folder — restart Firefox
        to activate. Requires <code className="font-mono">extension.xpi</code> (or{" "}
        <code className="font-mono">extension.crx</code> for Chromium) next to this app.
      </div>

      {/* Install button */}
      <button
        onClick={handleInstall}
        disabled={!anySelected || installing}
        className="w-full bg-[var(--accent)] hover:bg-[var(--accent-hover)] disabled:opacity-40
                   text-white font-medium py-3 rounded-xl transition-colors"
      >
        {installing ? "Installing…" : "Install extension"}
      </button>

      {/* Results */}
      {results.length > 0 && (
        <div className="flex flex-col gap-2">
          {results.map((r, i) => (
            <div key={i}
              className={`rounded-xl px-3 py-2 text-sm border ${
                r.success
                  ? "bg-green-950/20 border-green-900/30 text-[var(--success)]"
                  : "bg-red-950/20 border-red-900/30 text-[var(--danger)]"
              }`}
            >
              <strong className="capitalize">
                {r.browserId === "error" ? "Error" : r.browserId}:
              </strong>{" "}
              {r.message}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
