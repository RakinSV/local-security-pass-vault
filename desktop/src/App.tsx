import { useState, useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { vaultStatus, activityPing } from "./api/vault";
import { VaultPicker } from "./pages/VaultPicker";
import { Setup } from "./pages/Setup";
import { Unlock } from "./pages/Unlock";
import { VaultList } from "./pages/VaultList";
import { ItemDetail } from "./pages/ItemDetail";
import { ItemForm } from "./pages/ItemForm";
import { Settings } from "./pages/Settings";
import type { VaultEntry } from "./lib/vaultRegistry";

type Page =
  | { name: "loading" }
  | { name: "picker" }
  | { name: "setup" }
  | { name: "unlock"; entry: VaultEntry }
  | { name: "vault" }
  | { name: "detail"; id: string }
  | { name: "edit"; id: string }
  | { name: "add" }
  | { name: "settings" };

export default function App() {
  const [page, setPage] = useState<Page>({ name: "loading" });
  const [refreshKey, setRefreshKey] = useState(0);

  useEffect(() => {
    (async () => {
      try {
        const status = await vaultStatus();
        if (!status.isLocked) {
          setPage({ name: "vault" });
        } else {
          setPage({ name: "picker" });
        }
      } catch {
        setPage({ name: "picker" });
      }
    })();
  }, []);

  // Tray "Lock & Hide" + auto-lock timer both emit this event from Rust
  useEffect(() => {
    const unlisten = listen("vault-locked", () => {
      setPage({ name: "picker" });
    });
    return () => { unlisten.then(f => f()); };
  }, []);

  // Reset the Rust-side idle timer on user interaction (throttled to once/30 s)
  useEffect(() => {
    let lastPing = 0;
    const ping = () => {
      const now = Date.now();
      if (now - lastPing > 30_000) {
        lastPing = now;
        activityPing().catch(() => {});
      }
    };
    window.addEventListener("mousemove", ping, { passive: true });
    window.addEventListener("keydown", ping, { passive: true });
    return () => {
      window.removeEventListener("mousemove", ping);
      window.removeEventListener("keydown", ping);
    };
  }, []);

  function refresh() {
    setRefreshKey(k => k + 1);
  }

  if (page.name === "loading") {
    return (
      <div className="flex items-center justify-center h-screen">
        <div className="text-[var(--muted)] text-sm">Loading…</div>
      </div>
    );
  }

  if (page.name === "picker") {
    return (
      <VaultPicker
        onCreateVault={() => setPage({ name: "setup" })}
        onOpenVault={entry => setPage({ name: "unlock", entry })}
      />
    );
  }

  if (page.name === "setup") {
    return (
      <Setup
        onCreated={() => setPage({ name: "vault" })}
        onBack={() => setPage({ name: "picker" })}
      />
    );
  }

  if (page.name === "unlock") {
    return (
      <Unlock
        entry={page.entry}
        onUnlocked={() => setPage({ name: "vault" })}
        onBack={() => setPage({ name: "picker" })}
      />
    );
  }

  if (page.name === "vault") {
    return (
      <VaultList
        refreshKey={refreshKey}
        onSelectItem={id => setPage({ name: "detail", id })}
        onAddItem={() => setPage({ name: "add" })}
        onLocked={() => setPage({ name: "picker" })}
        onSwitchVault={() => setPage({ name: "picker" })}
        onSettings={() => setPage({ name: "settings" })}
      />
    );
  }

  if (page.name === "detail") {
    return (
      <ItemDetail
        itemId={page.id}
        onBack={() => setPage({ name: "vault" })}
        onEdit={() => setPage({ name: "edit", id: page.id })}
        onDeleted={() => { refresh(); setPage({ name: "vault" }); }}
      />
    );
  }

  if (page.name === "edit") {
    return (
      <ItemForm
        editId={page.id}
        onSaved={id => { refresh(); setPage({ name: "detail", id }); }}
        onBack={() => setPage({ name: "detail", id: page.id })}
      />
    );
  }

  if (page.name === "add") {
    return (
      <ItemForm
        onSaved={id => { refresh(); setPage({ name: "detail", id }); }}
        onBack={() => setPage({ name: "vault" })}
      />
    );
  }

  if (page.name === "settings") {
    return (
      <Settings
        onBack={() => setPage({ name: "vault" })}
        onImported={refresh}
      />
    );
  }

  return null;
}
