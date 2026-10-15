// SPDX-License-Identifier: GPL-3.0-or-later
import { NavLink, Outlet } from "react-router";
import { useTranslation } from "react-i18next";
import { LayoutDashboard, Landmark, FileBadge, Ban, Settings, Lock, Unlock, Database } from "lucide-react";
import { cn } from "@/lib/utils";
import { useApp } from "@/context/AppContext";
import { Button, Spinner } from "@/components/ui";
import { api } from "@/api";

const items = [
  { to: "/", key: "nav.dashboard", icon: LayoutDashboard, end: true },
  { to: "/cas", key: "nav.cas", icon: Landmark },
  { to: "/certificates", key: "nav.certificates", icon: FileBadge },
  { to: "/revocations", key: "nav.revocations", icon: Ban },
  { to: "/settings", key: "nav.settings", icon: Settings },
] as const;

export default function Layout() {
  const { t } = useTranslation();
  const { db, session, cas, loading, refresh } = useApp();
  const unlocked = cas.find((c) => c.id === session.unlocked_ca_id);
  return (
    <div className="flex h-full">
      <aside className="flex w-64 shrink-0 flex-col border-r border-border bg-sidebar">
        <div className="px-5 py-5">
          <div className="text-lg font-semibold tracking-tight">{t("app.name")}</div>
          <div className="text-xs text-muted-foreground">{t("app.tagline")}</div>
        </div>
        <nav className="flex flex-col gap-1 px-3">
          {items.map(({ to, key, icon: Icon, ...rest }) => (
            <NavLink key={to} to={to} {...rest} className={({ isActive }) => cn("flex items-center gap-3 rounded-md px-3 py-2 text-sm transition-colors", isActive ? "bg-primary text-primary-foreground" : "hover:bg-muted")}>
              <Icon className="size-4" />
              {t(key)}
            </NavLink>
          ))}
        </nav>
        <div className="mt-auto space-y-3 border-t border-border p-4 text-xs">
          <div className="flex items-start gap-2">
            <Database className="mt-0.5 size-3.5 shrink-0 text-muted-foreground" />
            {db ? (
              <div className="min-w-0">
                <div className="truncate" title={db.path}>{db.path.split("/").pop()}</div>
                <div className="text-muted-foreground">
                  {t(`db.mode.${db.mode}`)}
                  {db.read_only && <span className="ml-1 text-amber-600">· {t("db.readOnly")}</span>}
                </div>
              </div>
            ) : (
              <span className="text-muted-foreground">{t("db.none")}</span>
            )}
          </div>
          <div className="flex items-center gap-2">
            {unlocked ? (
              <>
                <Unlock className="size-3.5 text-emerald-600" />
                <span className="min-w-0 flex-1 truncate">{unlocked.name}</span>
                <Button size="sm" variant="outline" onClick={async () => { await api.caLock(); await refresh(); }}>
                  {t("session.lock")}
                </Button>
              </>
            ) : (
              <>
                <Lock className="size-3.5 text-muted-foreground" />
                <span className="text-muted-foreground">{t("session.locked")}</span>
              </>
            )}
          </div>
        </div>
      </aside>
      <main className="min-w-0 flex-1 overflow-auto">{loading ? <div className="flex h-full items-center justify-center"><Spinner /></div> : <Outlet />}</main>
    </div>
  );
}
