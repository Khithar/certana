// SPDX-License-Identifier: GPL-3.0-or-later
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Link, useNavigate } from "react-router";
import { Plus, Search } from "lucide-react";
import { api } from "@/api";
import type { CertStatus, CertificateFilter, CertificateRecord, Profile } from "@/api/types";
import { useApp, useCommand } from "@/context/AppContext";
import { Badge, Button, Empty, Input, PageHeader, Select, Spinner } from "@/components/ui";
import { ErrorBanner } from "@/components/ErrorBanner";
import { NoDatabase } from "@/pages/Settings";
import { IssueWizard } from "@/pages/IssueWizard";
import { daysUntil, fmtDate, sanLabel } from "@/lib/format";

export const STATUS_TONE: Record<CertStatus, "green" | "red" | "neutral"> = { valid: "green", revoked: "red", expired: "neutral" };

export default function Certificates() {
  const { t, i18n } = useTranslation();
  const { db, cas, session } = useApp();
  const { error, run } = useCommand();
  const navigate = useNavigate();
  const [rows, setRows] = useState<CertificateRecord[] | null>(null);
  const [filter, setFilter] = useState<CertificateFilter>({});
  const [search, setSearch] = useState("");
  const [wizard, setWizard] = useState(false);
  const issuing = cas.filter((c) => c.kind === "intermediate" && c.has_key);

  const load = async () => {
    const r = await run(() => api.certList({ ...filter, search: search || null }));
    if (r) setRows(r);
  };
  useEffect(() => {
    if (db) void load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [db, filter, search]);
  const caName = useMemo(() => Object.fromEntries(cas.map((c) => [c.id, c.name])), [cas]);
  if (!db) return <NoDatabase />;

  return (
    <div className="p-8">
      <PageHeader title={t("nav.certificates")} actions={<Button onClick={() => setWizard(true)} disabled={db.read_only || issuing.length === 0}><Plus className="size-4" />{t("wizard.new")}</Button>} />
      {issuing.length === 0 && <p className="mb-4 text-sm text-muted-foreground">{t("cert.noIssuingCa")} <Link to="/cas" className="text-primary underline">{t("nav.cas")}</Link></p>}
      <div className="mb-4 flex flex-wrap gap-2">
        <div className="relative min-w-64 flex-1"><Search className="pointer-events-none absolute top-2.5 left-3 size-4 text-muted-foreground" /><Input className="pl-9" placeholder={t("cert.searchPlaceholder")} value={search} onChange={(e) => setSearch(e.target.value)} /></div>
        <Select className="w-44" value={filter.ca_id ?? ""} onChange={(e) => setFilter({ ...filter, ca_id: e.target.value ? Number(e.target.value) : null })}><option value="">{t("cert.allCas")}</option>{cas.filter((c) => c.kind === "intermediate").map((c) => <option key={c.id} value={c.id}>{c.name}</option>)}</Select>
        <Select className="w-40" value={filter.profile ?? ""} onChange={(e) => setFilter({ ...filter, profile: (e.target.value || null) as Profile | null })}><option value="">{t("cert.allProfiles")}</option>{(["server", "client", "smime", "code-signing"] as Profile[]).map((p) => <option key={p} value={p}>{t(`profile.${p}`)}</option>)}</Select>
        <Select className="w-36" value={filter.status ?? ""} onChange={(e) => setFilter({ ...filter, status: (e.target.value || null) as CertStatus | null })}><option value="">{t("cert.allStatus")}</option>{(["valid", "revoked", "expired"] as CertStatus[]).map((s) => <option key={s} value={s}>{t(`status.${s}`)}</option>)}</Select>
        <Select className="w-40" value={filter.expiring_within_days ?? ""} onChange={(e) => setFilter({ ...filter, expiring_within_days: e.target.value ? Number(e.target.value) : null })}><option value="">{t("cert.anyExpiry")}</option>{[7, 14, 30, 90].map((d) => <option key={d} value={d}>{t("dashboard.expiring", { days: d })}</option>)}</Select>
      </div>
      <ErrorBanner error={error} />
      {!rows ? <Spinner /> : rows.length === 0 ? <Empty>{t("cert.empty")}</Empty> : (
        <div className="overflow-hidden rounded-lg border border-border">
          <table className="w-full text-sm">
            <thead className="bg-muted text-left text-xs text-muted-foreground"><tr><th className="px-4 py-2">{t("cert.subject")}</th><th className="px-4 py-2">{t("cert.sans")}</th><th className="px-4 py-2">{t("cert.profile")}</th><th className="px-4 py-2">{t("cert.ca")}</th><th className="px-4 py-2">{t("cert.notAfter")}</th><th className="px-4 py-2">{t("cert.status")}</th></tr></thead>
            <tbody>
              {rows.map((c) => {
                const d = daysUntil(c.not_after);
                return (
                  <tr key={c.id} className="cursor-pointer border-t border-border hover:bg-muted/60" onClick={() => navigate(`/certificates/${c.id}`)}>
                    <td className="px-4 py-2 font-medium">{c.subject.replace(/^.*CN=/, "")}</td>
                    <td className="max-w-64 truncate px-4 py-2 text-muted-foreground" title={c.sans.map(sanLabel).join(", ")}>{c.sans.map((s) => s.value).join(", ")}</td>
                    <td className="px-4 py-2">{t(`profile.${c.profile}`)}</td>
                    <td className="px-4 py-2">{caName[c.ca_id] ?? c.ca_id}</td>
                    <td className="px-4 py-2">{fmtDate(c.not_after, i18n.language)}{c.status === "valid" && d <= 30 && <Badge tone={d <= 7 ? "red" : "amber"} className="ml-2">{d}d</Badge>}</td>
                    <td className="px-4 py-2"><Badge tone={STATUS_TONE[c.status]}>{t(`status.${c.status}`)}</Badge></td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
      <IssueWizard open={wizard} onClose={() => setWizard(false)} issuing={issuing} defaultCaId={session.unlocked_ca_id ?? issuing[0]?.id ?? null} onIssued={(id) => { setWizard(false); navigate(`/certificates/${id}`); }} />
    </div>
  );
}
