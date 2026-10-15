// SPDX-License-Identifier: GPL-3.0-or-later
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Link, useNavigate, useParams } from "react-router";
import { ArrowLeft, Ban, Download, RefreshCw } from "lucide-react";
import { api } from "@/api";
import type { CertDetail, ExportResult, KeyAlgorithm, Reason } from "@/api/types";
import { pickDirectory } from "@/api/dialogs";
import { useApp, useCommand } from "@/context/AppContext";
import { Alert, Badge, Button, Card, Checkbox, CopyButton, Field, Input, KeyValue, Modal, Mono, PageHeader, Select, Spinner, Textarea } from "@/components/ui";
import { ErrorBanner } from "@/components/ErrorBanner";
import { useEnsureUnlocked } from "@/components/UnlockDialog";
import { AlgoSelect } from "@/pages/Cas";
import { STATUS_TONE } from "@/pages/Certificates";
import { colonHex, fmtDateTime, sanLabel } from "@/lib/format";

const REASONS: Reason[] = ["unspecified", "keyCompromise", "caCompromise", "affiliationChanged", "superseded", "cessationOfOperation", "privilegeWithdrawn"];

export default function CertificateDetail() {
  const { t, i18n } = useTranslation();
  const { id } = useParams();
  const navigate = useNavigate();
  const { db, settings, refresh } = useApp();
  const { busy, error, run } = useCommand();
  const { ensure, dialog } = useEnsureUnlocked();
  const [d, setD] = useState<CertDetail | null>(null);
  const [modal, setModal] = useState<"export" | "revoke" | "renew" | null>(null);
  const [exp, setExp] = useState({ dir: settings["export.dir"] ?? "", name: "", key: true, der: false, p12: false, p12pw: "", legacy: false });
  const [exported, setExported] = useState<ExportResult | null>(null);
  const [reason, setReason] = useState<Reason>("unspecified");
  const [comment, setComment] = useState("");
  const [renew, setRenew] = useState({ days: "397", newKey: false, algo: "ecdsa-p256" as KeyAlgorithm, revokeOld: true });
  const [notes, setNotes] = useState("");

  const load = async () => { const r = await run(() => api.certGet(Number(id))); if (r) { setD(r); setNotes(r.record.notes); setExp((e) => ({ ...e, name: r.info.subject_common_name ?? "", dir: e.dir || settings["export.dir"] || "" })); } };
  useEffect(() => {
    void load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [id]);
  if (!db) return null;
  if (!d) return <div className="p-8"><ErrorBanner error={error} /><Spinner /></div>;
  const { record, info, revocation } = d;

  const doExport = () => ensure(record.ca_id, async () => {
    const r = await run(() => api.certExport({ cert_id: record.id, dest_dir: exp.dir, base_name: exp.name || null, include_key: exp.key && record.has_key, include_der: exp.der, include_p12: exp.p12 && record.has_key, p12_password: exp.p12pw || null, p12_legacy: exp.legacy }));
    if (r) setExported(r);
  });
  const doRevoke = async () => { const r = await run(() => api.certRevoke(record.id, reason, comment || null)); if (r) { setModal(null); await load(); await refresh(); } };
  const doRenew = () => ensure(record.ca_id, async () => {
    const r = await run(() => api.certRenew({ cert_id: record.id, validity_days: Number(renew.days), new_key_algorithm: renew.newKey || !record.has_key && !record.csr_pem ? renew.algo : null, revoke_old: renew.revokeOld }));
    if (r) { setModal(null); await refresh(); navigate(`/certificates/${r.id}`); }
  });
  const saveNotes = () => run(() => api.certUpdateNotes(record.id, notes));

  return (
    <div className="space-y-6 p-8">
      <Link to="/certificates" className="inline-flex items-center gap-1 text-sm text-muted-foreground hover:underline"><ArrowLeft className="size-4" />{t("nav.certificates")}</Link>
      <PageHeader
        title={info.subject_common_name ?? record.subject}
        subtitle={record.subject}
        actions={
          <>
            <Badge tone={STATUS_TONE[record.status]}>{t(`status.${record.status}`)}</Badge>
            <Button variant="outline" size="sm" onClick={() => { setExported(null); setModal("export"); }}><Download className="size-4" />{t("export.title")}</Button>
            <Button variant="outline" size="sm" onClick={() => setModal("renew")} disabled={db.read_only}><RefreshCw className="size-4" />{t("renew.title")}</Button>
            {record.status === "valid" && <Button variant="destructive" size="sm" onClick={() => setModal("revoke")} disabled={db.read_only}><Ban className="size-4" />{t("revoke.title")}</Button>}
          </>
        }
      />
      <ErrorBanner error={error} />
      {revocation && <Alert tone="error" title={t("revoke.revokedAt", { date: fmtDateTime(revocation.revoked_at, i18n.language) })}>{t(`reason.${revocation.reason}`)}{revocation.comment && ` · ${revocation.comment}`}{revocation.first_crl_id ? ` · ${t("revoke.inCrl")}` : ` · ${t("revoke.notInCrl")}`}</Alert>}
      <div className="grid gap-6 lg:grid-cols-2">
        <Card title={t("ca.details")}>
          <KeyValue rows={[
            [t("cert.profile"), t(`profile.${record.profile}`)],
            [t("cert.ca"), <Link key="ca" to={`/cas?ca=${record.ca_id}`} className="text-primary hover:underline">{d.ca_name}</Link>],
            [t("cert.serial"), <Mono key="s">{colonHex(info.serial_hex)}</Mono>],
            [t("cert.sans"), record.sans.map(sanLabel).join(", ") || "–"],
            [t("cert.notBefore"), fmtDateTime(info.not_before, i18n.language)],
            [t("cert.notAfter"), fmtDateTime(info.not_after, i18n.language)],
            [t("cert.keyAlgo"), info.public_key_algorithm],
            [t("cert.keyUsage"), info.key_usages.join(", ")],
            [t("cert.eku"), info.extended_key_usages.join(", ")],
            [t("cert.fingerprint"), <Mono key="f">{colonHex(info.sha256_fingerprint)}</Mono>],
            [t("cert.keyStored"), record.has_key ? t("common.yes") : t("cert.keyExternal")],
            [t("cert.createdBy"), `${record.created_by} · ${fmtDateTime(record.created_at, i18n.language)}`],
          ]} />
        </Card>
        <Card title={t("cert.notes")} actions={<Button size="sm" variant="outline" onClick={saveNotes} disabled={db.read_only || notes === record.notes}>{t("common.save")}</Button>}>
          <Textarea value={notes} onChange={(e) => setNotes(e.target.value)} className="min-h-32 font-sans text-sm" disabled={db.read_only} />
        </Card>
        <Card title={t("cert.textDump")} className="lg:col-span-2" actions={<CopyButton text={record.cert_pem} label="PEM" />}>
          <Textarea readOnly value={info.text} className="min-h-72" />
        </Card>
      </div>

      <Modal open={modal === "export"} onClose={() => setModal(null)} title={t("export.title")} wide footer={<><Button variant="outline" onClick={() => setModal(null)}>{t("common.close")}</Button><Button onClick={doExport} loading={busy} disabled={!exp.dir}>{t("export.write")}</Button></>}>
        <div className="space-y-4">
          <Field label={t("export.dir")}><div className="flex gap-2"><Input value={exp.dir} onChange={(e) => setExp({ ...exp, dir: e.target.value })} /><Button variant="outline" onClick={async () => { const p = await pickDirectory(t("export.dir")); if (p) setExp({ ...exp, dir: p }); }}>{t("common.browse")}</Button></div></Field>
          <Field label={t("export.baseName")}><Input value={exp.name} onChange={(e) => setExp({ ...exp, name: e.target.value })} /></Field>
          <div className="grid grid-cols-2 gap-2">
            <Checkbox label={t("export.includeKey")} checked={exp.key} onChange={(v) => setExp({ ...exp, key: v })} disabled={!record.has_key} />
            <Checkbox label={t("export.includeDer")} checked={exp.der} onChange={(v) => setExp({ ...exp, der: v })} />
            <Checkbox label={t("export.includeP12")} checked={exp.p12} onChange={(v) => setExp({ ...exp, p12: v })} disabled={!record.has_key} />
            {exp.p12 && <Checkbox label={t("export.p12Legacy")} checked={exp.legacy} onChange={(v) => setExp({ ...exp, legacy: v })} />}
          </div>
          {exp.p12 && <Field label={t("export.p12Password")}><Input type="password" value={exp.p12pw} onChange={(e) => setExp({ ...exp, p12pw: e.target.value })} /></Field>}
          {!record.has_key && <Alert tone="info">{t("export.noKeyHint")}</Alert>}
          {exported && (
            <div className="space-y-3">
              <Alert tone="success" title={t("export.done")}><ul className="list-disc pl-5">{exported.files.map((f) => <li key={f}><Mono>{f}</Mono></li>)}</ul>{exported.key_written && <div className="mt-2">{t("export.keyPerms")}</div>}</Alert>
              {record.profile === "server" && (
                <>
                  <Snippet title="nginx" text={exported.snippets.nginx} />
                  <Snippet title="Apache" text={exported.snippets.apache} />
                </>
              )}
            </div>
          )}
          <ErrorBanner error={error} />
        </div>
      </Modal>

      <Modal open={modal === "revoke"} onClose={() => setModal(null)} title={t("revoke.title")} footer={<><Button variant="outline" onClick={() => setModal(null)}>{t("common.cancel")}</Button><Button variant="destructive" onClick={doRevoke} loading={busy}>{t("revoke.confirm")}</Button></>}>
        <div className="space-y-4">
          <Alert tone="warning">{t("revoke.warning")}</Alert>
          <Field label={t("revoke.reason")}><Select value={reason} onChange={(e) => setReason(e.target.value as Reason)}>{REASONS.map((r) => <option key={r} value={r}>{t(`reason.${r}`)}</option>)}</Select></Field>
          <Field label={t("revoke.comment")}><Input value={comment} onChange={(e) => setComment(e.target.value)} /></Field>
          <ErrorBanner error={error} />
        </div>
      </Modal>

      <Modal open={modal === "renew"} onClose={() => setModal(null)} title={t("renew.title")} footer={<><Button variant="outline" onClick={() => setModal(null)}>{t("common.cancel")}</Button><Button onClick={doRenew} loading={busy}>{t("renew.confirm")}</Button></>}>
        <div className="space-y-4">
          <p className="text-sm text-muted-foreground">{t("renew.hint")}</p>
          <Field label={t("wizard.validity")}><Input type="number" min={1} value={renew.days} onChange={(e) => setRenew({ ...renew, days: e.target.value })} /></Field>
          <Checkbox label={t("renew.newKey")} checked={renew.newKey || (!record.has_key && !record.csr_pem)} onChange={(v) => setRenew({ ...renew, newKey: v })} disabled={!record.has_key && !record.csr_pem} />
          {(renew.newKey || (!record.has_key && !record.csr_pem)) && <Field label={t("cert.keyAlgo")}><AlgoSelect value={renew.algo} onChange={(a) => setRenew({ ...renew, algo: a })} /></Field>}
          <Checkbox label={t("renew.revokeOld")} checked={renew.revokeOld} onChange={(v) => setRenew({ ...renew, revokeOld: v })} disabled={record.status !== "valid"} />
          <ErrorBanner error={error} />
        </div>
      </Modal>
      {dialog}
    </div>
  );
}

function Snippet({ title, text }: { title: string; text: string }) {
  return (
    <div>
      <div className="mb-1 flex items-center justify-between text-sm font-medium">{title}<CopyButton text={text} /></div>
      <pre className="overflow-x-auto rounded-md bg-muted p-3 font-mono text-xs">{text}</pre>
    </div>
  );
}
