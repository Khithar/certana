// SPDX-License-Identifier: GPL-3.0-or-later
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Link } from "react-router";
import { Download, RefreshCw } from "lucide-react";
import { api } from "@/api";
import type { CertificateRecord, CrlPublished, CrlRecord } from "@/api/types";
import { pickSavePath } from "@/api/dialogs";
import { useApp, useCommand } from "@/context/AppContext";
import { Alert, Badge, Button, Card, Empty, Mono, PageHeader, Select } from "@/components/ui";
import { ErrorBanner } from "@/components/ErrorBanner";
import { useEnsureUnlocked } from "@/components/UnlockDialog";
import { NoDatabase } from "@/pages/Settings";
import { fmtDate, fmtDateTime } from "@/lib/format";

export default function Revocations() {
  const { t, i18n } = useTranslation();
  const { db, cas, settings, refresh } = useApp();
  const { busy, error, run } = useCommand();
  const { ensure, dialog } = useEnsureUnlocked();
  const issuing = cas.filter((c) => c.kind === "intermediate");
  const [caId, setCaId] = useState<number | null>(null);
  const [revoked, setRevoked] = useState<CertificateRecord[]>([]);
  const [crls, setCrls] = useState<CrlRecord[]>([]);
  const [published, setPublished] = useState<CrlPublished | null>(null);
  const [now, setNow] = useState(0);
  const ca = cas.find((c) => c.id === caId) ?? null;
  useEffect(() => { if (!caId && issuing[0]) setCaId(issuing[0].id); }, [issuing, caId]);
  const load = async () => {
    if (!caId) return;
    const [r, c] = await Promise.all([api.certList({ ca_id: caId, status: "revoked" }), api.crlList(caId, 20)]);
    setRevoked(r); setCrls(c); setNow(Date.now());
  };
  useEffect(() => {
    void load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [caId]);
  if (!db) return <NoDatabase />;
  const publish = () => caId && ensure(caId, async () => { const r = await run(() => api.crlPublish(caId, settings["crl.export_dir"] || null)); if (r) { setPublished(r); await load(); await refresh(); } });
  const exportLatest = async (pem: boolean) => {
    if (!ca) return;
    const dest = await pickSavePath(t("crl.export"), `${ca.name.replace(/\s+/g, "-")}.crl${pem ? ".pem" : ""}`);
    if (dest) await run(() => api.crlExport(ca.id, dest));
  };
  const latest = crls[0];
  const latestExpired = latest && new Date(latest.next_update).getTime() < now;
  return (
    <div className="space-y-6 p-8">
      <PageHeader title={t("nav.revocations")} actions={<><Select className="w-56" value={caId ?? ""} onChange={(e) => setCaId(Number(e.target.value))}>{issuing.map((c) => <option key={c.id} value={c.id}>{c.name}</option>)}</Select><Button onClick={publish} loading={busy} disabled={!ca || db.read_only}><RefreshCw className="size-4" />{t("crl.publish")}</Button></>} />
      <ErrorBanner error={error} />
      {!ca ? <Empty>{t("crl.noIssuing")}</Empty> : (
        <>
          {published && <Alert tone="success" title={t("crl.published", { number: published.crl.crl_number, count: published.revoked_count })}>{published.der_path ? <Mono>{published.der_path}</Mono> : t("crl.notExported")}</Alert>}
          {latest && latestExpired && <Alert tone="warning">{t("crl.expired", { date: fmtDateTime(latest.next_update, i18n.language) })}</Alert>}
          <div className="grid gap-6 lg:grid-cols-2">
            <Card title={t("crl.revokedList", { count: revoked.length })}>
              {revoked.length === 0 ? <Empty>{t("crl.noneRevoked")}</Empty> : (
                <table className="w-full text-sm"><thead className="text-left text-xs text-muted-foreground"><tr><th className="pb-2">{t("cert.subject")}</th><th className="pb-2">{t("cert.notAfter")}</th></tr></thead>
                  <tbody>{revoked.map((c) => <tr key={c.id} className="border-t border-border"><td className="py-2"><Link to={`/certificates/${c.id}`} className="hover:underline">{c.subject.replace(/^.*CN=/, "")}</Link></td><td className="py-2">{fmtDate(c.not_after, i18n.language)}</td></tr>)}</tbody></table>
              )}
            </Card>
            <Card title={t("crl.history")} actions={latest && <><Button size="sm" variant="outline" onClick={() => exportLatest(false)}><Download className="size-4" />DER</Button><Button size="sm" variant="outline" onClick={() => exportLatest(true)}><Download className="size-4" />PEM</Button></>}>
              {crls.length === 0 ? <Empty>{t("crl.none")}</Empty> : (
                <table className="w-full text-sm"><thead className="text-left text-xs text-muted-foreground"><tr><th className="pb-2">#</th><th className="pb-2">{t("crl.thisUpdate")}</th><th className="pb-2">{t("crl.nextUpdate")}</th><th className="pb-2">{t("crl.exportedTo")}</th></tr></thead>
                  <tbody>{crls.map((c) => { const exp = new Date(c.next_update).getTime() < now; return <tr key={c.id} className="border-t border-border"><td className="py-2">{c.crl_number}</td><td className="py-2">{fmtDateTime(c.this_update, i18n.language)}</td><td className="py-2"><Badge tone={exp ? "red" : "green"}>{fmtDateTime(c.next_update, i18n.language)}</Badge></td><td className="max-w-48 truncate py-2 text-muted-foreground" title={c.exported_path ?? ""}>{c.exported_path ?? "–"}</td></tr>; })}</tbody></table>
              )}
            </Card>
          </div>
          <Alert tone="info">{t("crl.serveHint", { url: ca.crl_url ?? "–" })}</Alert>
        </>
      )}
      {dialog}
    </div>
  );
}
