// SPDX-License-Identifier: GPL-3.0-or-later
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Link } from "react-router";
import { api } from "@/api";
import type { DashboardSummary } from "@/api/types";
import { useApp, useCommand } from "@/context/AppContext";
import { Badge, Button, Card, Empty, PageHeader, Spinner } from "@/components/ui";
import { ErrorBanner } from "@/components/ErrorBanner";
import { NoDatabase } from "@/pages/Settings";
import { daysUntil, fmtDate, fmtDateTime } from "@/lib/format";
import { notify } from "@/api/dialogs";

export default function Dashboard() {
  const { t, i18n } = useTranslation();
  const { db, session, settings } = useApp();
  const { error, run } = useCommand();
  const [data, setData] = useState<DashboardSummary | null>(null);
  const [notified, setNotified] = useState(false);

  const load = async () => {
    const d = await run(() => api.dashboardSummary());
    if (d) setData(d);
  };
  useEffect(() => {
    if (db) void load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [db, session.unlocked_ca_id]);

  useEffect(() => {
    if (!data || notified) return;
    const warn = Number(settings["notify.expiry_warn_days"] ?? 14);
    const soon = data.expiring_30.filter((c) => daysUntil(c.not_after) <= warn);
    if (soon.length > 0) void notify(t("app.name"), t("dashboard.notifyExpiring", { count: soon.length, days: warn }));
    setNotified(true);
  }, [data, notified, settings, t]);

  if (!db) return <NoDatabase />;
  return (
    <div className="p-8">
      <PageHeader title={t("nav.dashboard")} actions={<Button variant="outline" onClick={load}>{t("common.refresh")}</Button>} />
      <ErrorBanner error={error} />
      {!data ? (
        <Spinner />
      ) : (
        <div className="grid gap-6 lg:grid-cols-3">
          <div className="grid grid-cols-3 gap-4 lg:col-span-3">
            <Stat label={t("status.valid")} value={data.valid} tone="green" />
            <Stat label={t("status.revoked")} value={data.revoked} tone="red" />
            <Stat label={t("status.expired")} value={data.expired} tone="neutral" />
            <Stat label={t("dashboard.expiring", { days: 30 })} value={data.expiring_30.length} tone={data.expiring_30.length ? "amber" : "neutral"} />
            <Stat label={t("dashboard.expiring", { days: 14 })} value={data.expiring_14} tone={data.expiring_14 ? "amber" : "neutral"} />
            <Stat label={t("dashboard.expiring", { days: 7 })} value={data.expiring_7} tone={data.expiring_7 ? "red" : "neutral"} />
          </div>

          <Card title={t("dashboard.cas")} className="lg:col-span-2">
            {data.cas.length === 0 ? (
              <Empty>
                {t("dashboard.noCas")} <Link className="text-primary underline" to="/cas">{t("nav.cas")}</Link>
              </Empty>
            ) : (
              <table className="w-full text-sm">
                <thead className="text-left text-xs text-muted-foreground">
                  <tr><th className="pb-2">{t("ca.name")}</th><th className="pb-2">{t("ca.kind")}</th><th className="pb-2">{t("ca.validUntil")}</th><th className="pb-2">{t("crl.title")}</th><th className="pb-2">{t("session.title")}</th></tr>
                </thead>
                <tbody>
                  {data.cas.map(({ ca, latest_crl, crl_expires_soon, unlocked }) => (
                    <tr key={ca.id} className="border-t border-border">
                      <td className="py-2 font-medium"><Link to={`/cas?ca=${ca.id}`} className="hover:underline">{ca.name}</Link></td>
                      <td className="py-2"><Badge tone={ca.kind === "root" ? "blue" : "neutral"}>{t(`ca.kinds.${ca.kind}`)}</Badge></td>
                      <td className="py-2">{fmtDate(ca.not_after, i18n.language)}</td>
                      <td className="py-2">{latest_crl ? <Badge tone={crl_expires_soon ? "amber" : "green"}>#{latest_crl.crl_number} · {fmtDate(latest_crl.next_update, i18n.language)}</Badge> : <Badge tone="neutral">{t("crl.none")}</Badge>}</td>
                      <td className="py-2">{ca.kind === "root" ? <span className="text-muted-foreground">{t("ca.offline")}</span> : unlocked ? <Badge tone="green">{t("session.unlocked")}</Badge> : <Badge tone="neutral">{t("session.locked")}</Badge>}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </Card>

          <Card title={t("dashboard.recentAudit")}>
            {data.recent_audit.length === 0 ? <Empty>{t("common.nothingYet")}</Empty> : (
              <ul className="space-y-2 text-xs">
                {data.recent_audit.map((a) => (
                  <li key={a.id} className="flex flex-col">
                    <span className="font-medium">{a.action} <span className="text-muted-foreground">{a.target_type} {a.target_id}</span></span>
                    <span className="text-muted-foreground">{fmtDateTime(a.ts, i18n.language)} · {a.actor}@{a.host}</span>
                  </li>
                ))}
              </ul>
            )}
          </Card>

          <Card title={t("dashboard.expiringList")} className="lg:col-span-3">
            {data.expiring_30.length === 0 ? <Empty>{t("dashboard.nothingExpiring")}</Empty> : (
              <table className="w-full text-sm">
                <thead className="text-left text-xs text-muted-foreground"><tr><th className="pb-2">{t("cert.subject")}</th><th className="pb-2">{t("cert.profile")}</th><th className="pb-2">{t("cert.notAfter")}</th><th className="pb-2">{t("cert.daysLeft")}</th></tr></thead>
                <tbody>
                  {data.expiring_30.map((c) => {
                    const d = daysUntil(c.not_after);
                    return (
                      <tr key={c.id} className="border-t border-border">
                        <td className="py-2"><Link to={`/certificates/${c.id}`} className="hover:underline">{c.subject}</Link></td>
                        <td className="py-2">{t(`profile.${c.profile}`)}</td>
                        <td className="py-2">{fmtDate(c.not_after, i18n.language)}</td>
                        <td className="py-2"><Badge tone={d <= 7 ? "red" : "amber"}>{d}</Badge></td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
            )}
          </Card>
        </div>
      )}
    </div>
  );
}

function Stat({ label, value, tone }: { label: string; value: number; tone: "green" | "red" | "amber" | "neutral" }) {
  const colors = { green: "text-emerald-600", red: "text-red-600", amber: "text-amber-600", neutral: "text-foreground" };
  return (
    <div className="rounded-lg border border-border bg-background p-4">
      <div className="text-xs text-muted-foreground">{label}</div>
      <div className={`mt-1 text-2xl font-semibold ${colors[tone]}`}>{value}</div>
    </div>
  );
}
