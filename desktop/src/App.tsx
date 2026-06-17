import { useState, useEffect } from "react";
import { vaultStatus } from "./api/vault";
import { Welcome } from "./pages/Welcome";
import { Setup } from "./pages/Setup";
import { Unlock } from "./pages/Unlock";
import { VaultList } from "./pages/VaultList";
import { ItemDetail } from "./pages/ItemDetail";
import { ItemForm } from "./pages/ItemForm";
import { Settings } from "./pages/Settings";

type Page =
  | { name: "loading" }
  | { name: "welcome" }
  | { name: "setup" }
  | { name: "unlock" }
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
          return;
        }
        // Can't check filesystem from JS — use localStorage as marker.
        const hasVault = localStorage.getItem("vaultpass:hasVault") === "1";
        if (hasVault) {
          setPage({ name: "unlock" });
        } else {
          setPage({ name: "welcome" });
        }
      } catch {
        setPage({ name: "welcome" });
      }
    })();
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

  if (page.name === "welcome") {
    return (
      <Welcome
        onSetup={() => setPage({ name: "setup" })}
        onUnlock={() => setPage({ name: "unlock" })}
      />
    );
  }

  if (page.name === "setup") {
    return (
      <Setup
        onCreated={() => {
          localStorage.setItem("vaultpass:hasVault", "1");
          setPage({ name: "vault" });
        }}
        onBack={() => setPage({ name: "welcome" })}
      />
    );
  }

  if (page.name === "unlock") {
    return (
      <Unlock
        onUnlocked={() => setPage({ name: "vault" })}
        onBack={() => setPage({ name: "welcome" })}
      />
    );
  }

  if (page.name === "vault") {
    return (
      <VaultList
        refreshKey={refreshKey}
        onSelectItem={id => setPage({ name: "detail", id })}
        onAddItem={() => setPage({ name: "add" })}
        onLocked={() => setPage({ name: "unlock" })}
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
