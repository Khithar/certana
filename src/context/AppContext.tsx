// SPDX-License-Identifier: GPL-3.0-or-later
// Global app state: open database, unlocked CA session, CA list and settings. Refreshed after
// every mutating command via `refresh()`.
import * as React from "react";
import { api, asCommandError } from "@/api";
import type { CaRecord, CommandError, OpenInfo, SessionStatus } from "@/api/types";
import { SETTING_KEYS } from "@/api/types";
import i18n from "@/i18n";

interface AppCtx {
  db: OpenInfo | null;
  session: SessionStatus;
  cas: CaRecord[];
  settings: Record<string, string>;
  loading: boolean;
  error: CommandError | null;
  refresh: () => Promise<void>;
  openDb: (path: string | null, forceShared?: boolean) => Promise<void>;
  closeDb: () => Promise<void>;
  setSetting: (key: string, value: string) => Promise<void>;
}

const Ctx = React.createContext<AppCtx | null>(null);

export function AppProvider({ children }: { children: React.ReactNode }) {
  const [db, setDb] = React.useState<OpenInfo | null>(null);
  const [session, setSession] = React.useState<SessionStatus>({ unlocked_ca_id: null, idle_seconds_left: null });
  const [cas, setCas] = React.useState<CaRecord[]>([]);
  const [settings, setSettings] = React.useState<Record<string, string>>({});
  const [loading, setLoading] = React.useState(true);
  const [error, setError] = React.useState<CommandError | null>(null);

  const refresh = React.useCallback(async () => {
    try {
      const info = await api.dbInfo();
      setDb(info);
      if (info) {
        const [s, c, st] = await Promise.all([api.caSession(), api.caList(), api.settingsAll()]);
        setSession(s);
        setCas(c);
        setSettings(st);
        const lang = st[SETTING_KEYS.language];
        if (lang && lang !== i18n.language) void i18n.changeLanguage(lang);
      } else {
        setSession({ unlocked_ca_id: null, idle_seconds_left: null });
        setCas([]);
        setSettings({});
      }
      setError(null);
    } catch (e) {
      setError(asCommandError(e));
    }
  }, []);

  const openDb = React.useCallback(
    async (path: string | null, forceShared = false) => {
      setError(null);
      try {
        const info = await api.dbOpen(path, forceShared);
        try {
          localStorage.setItem("certana.lastDb", info.path);
        } catch {
          /* ignore */
        }
      } catch (e) {
        setError(asCommandError(e));
        throw e;
      } finally {
        await refresh();
      }
    },
    [refresh],
  );

  const closeDb = React.useCallback(async () => {
    await api.dbClose();
    await refresh();
  }, [refresh]);

  const setSetting = React.useCallback(
    async (key: string, value: string) => {
      await api.settingsSet(key, value);
      await refresh();
    },
    [refresh],
  );

  // On start: reopen the last database (or the default one if it exists).
  React.useEffect(() => {
    (async () => {
      try {
        let last: string | null = null;
        try {
          last = localStorage.getItem("certana.lastDb");
        } catch {
          /* ignore */
        }
        if (last) {
          try {
            await api.dbOpen(last, false);
          } catch {
            /* fall through to the settings screen */
          }
        }
      } finally {
        await refresh();
        setLoading(false);
      }
    })();
  }, [refresh]);

  // Poll the session so auto-lock is reflected in the UI.
  React.useEffect(() => {
    if (!db) return;
    const id = setInterval(async () => {
      try {
        setSession(await api.caSession());
      } catch {
        /* ignore */
      }
    }, 15_000);
    return () => clearInterval(id);
  }, [db]);

  return <Ctx.Provider value={{ db, session, cas, settings, loading, error, refresh, openDb, closeDb, setSetting }}>{children}</Ctx.Provider>;
}

export function useApp() {
  const ctx = React.useContext(Ctx);
  if (!ctx) throw new Error("useApp outside AppProvider");
  return ctx;
}

/** Run an async command, capturing CommandError for display. */
export function useCommand() {
  const [busy, setBusy] = React.useState(false);
  const [error, setError] = React.useState<CommandError | null>(null);
  const run = React.useCallback(async <T,>(fn: () => Promise<T>): Promise<T | undefined> => {
    setBusy(true);
    setError(null);
    try {
      return await fn();
    } catch (e) {
      setError(asCommandError(e));
      return undefined;
    } finally {
      setBusy(false);
    }
  }, []);
  return { busy, error, run, clearError: () => setError(null) };
}
