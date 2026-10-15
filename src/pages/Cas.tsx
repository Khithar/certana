// SPDX-License-Identifier: GPL-3.0-or-later
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useSearchParams } from "react-router";
import { Download, KeyRound, Plus, Upload, FileSignature, RefreshCw } from "lucide-react";
import { api } from "@/api";
import type { CaDetail, CaRecord, CrlPublished, KeyAlgorithm } from "@/api/types";
import { pickFile, pickSavePath } from "@/api/dialogs";
import { useApp, useCommand } from "@/context/AppContext";
import { Badge, Button, Card, Empty, Field, Input, KeyValue, Mono, PageHeader, Select, Textarea, Alert, CopyButton } from "@/components/ui";
import { ErrorBanner } from "@/components/ErrorBanner";
import { UnlockDialog } from "@/components/UnlockDialog";
import { NoDatabase } from "@/pages/Settings";
import { RootCeremonyDialog, RootImportDialog, IntermediateDialog, RootCrlDialog, ChangePassphraseDialog } from "@/pages/CaDialogs";
import { colonHex, fmtDate, fmtDateTime } from "@/lib/format";

export const ALGOS: KeyAlgorithm[] = ["ecdsa-p256", "ecdsa-p384", "ed25519", "rsa2048", "rsa3072", "rsa4096"];
export const ALGO_LABEL: Record<KeyAlgorithm, string> = { "ecdsa-p256": "ECDSA P-256", "ecdsa-p384": "ECDSA P-384", ed25519: "Ed25519", rsa2048: "RSA 2048", rsa3072: "RSA 3072", rsa4096: "RSA 4096" };

export default function Cas() {
  const { t, i18n } = useTranslation();
  const { db, cas, session, refresh } = useApp();
  const [params, setParams] = useSearchParams();
  const selectedId = Number(params.get("ca")) || cas[0]?.id || null;
  const selected = cas.find((c) => c.id === selectedId) ?? null;
  const [dialog, setDialog] = useState<"root" | "import" | "intermediate" | "unlock" | "rootCrl" | "passphrase" | null>(null);
  const roots = cas.filter((c) => c.kind === "root");
  if (!db) return <NoDatabase />;
  return (
    <div className="flex h-full">
      <aside className="w-72 shrink-0 overflow-auto border-r border-border p-4">
        <div className="mb-3 flex items-center justify-between">
          <h1 className="text-lg font-semibold">{t("nav.cas")}</h1>
        </div>
        <div className="mb-4 flex flex-col gap-2">
          <Button size="sm" onClick={() => setDialog("root")} disabled={db.read_only}><Plus className="size-4" />{t("ca.createRoot")}</Button>
          <Button size="sm" variant="outline" onClick={() => setDialog("import")} disabled={db.read_only}><Upload className="size-4" />{t("ca.importRoot")}</Button>
          <Button size="sm" variant="outline" onClick={() => setDialog("intermediate")} disabled={db.read_only || roots.length === 0}><FileSignature className="size-4" />{t("ca.createIntermediate")}</Button>
        </div>
        {cas.length === 0 ? <Empty>{t("ca.empty")}</Empty> : (
          <ul className="space-y-1">
            {cas.map((ca) => (
              <li key={ca.id}>
                <button onClick={() => setParams({ ca: String(ca.id) })} className={`w-full rounded-md px-3 py-2 text-left text-sm ${ca.id === selectedId ? "bg-primary text-primary-foreground" : "hover:bg-muted"}`}>
                  <div className="flex items-center justify-between gap-2">
                    <span className="truncate font-medium">{ca.name}</span>
                    {ca.kind === "intermediate" && session.unlocked_ca_id === ca.id && <KeyRound className="size-3.5 shrink-0" />}
                  </div>
                  <div className={`text-xs ${ca.id === selectedId ? "opacity-80" : "text-muted-foreground"}`}>{t(`ca.kinds.${ca.kind}`)} · {fmtDate(ca.not_after, i18n.language)}</div>
                </button>
              </li>
            ))}
          </ul>
        )}
      </aside>
      <div className="min-w-0 flex-1 overflow-auto p-8">
        {selected ? <CaView ca={selected} onUnlock={() => setDialog("unlock")} onRootCrl={() => setDialog("rootCrl")} onChangePassphrase={() => setDialog("passphrase")} /> : <Empty>{t("ca.selectHint")}</Empty>}
      </div>
      <RootCeremonyDialog open={dialog === "root"} onClose={() => setDialog(null)} onDone={async (id) => { await refresh(); setParams({ ca: String(id) }); }} />
      <RootImportDialog open={dialog === "import"} onClose={() => setDialog(null)} onDone={async (id) => { await refresh(); setParams({ ca: String(id) }); }} />
      <IntermediateDialog open={dialog === "intermediate"} onClose={() => setDialog(null)} roots={roots} onDone={async (id) => { await refresh(); if (id) setParams({ ca: String(id) }); }} />
      <UnlockDialog ca={selected} open={dialog === "unlock"} onClose={() => setDialog(null)} />
      <RootCrlDialog ca={selected} open={dialog === "rootCrl"} onClose={() => setDialog(null)} onDone={refresh} />
      <ChangePassphraseDialog ca={selected} open={dialog === "passphrase"} onClose={() => setDialog(null)} onDone={refresh} />
    </div>
  );
}

function CaView({ ca, onUnlock, onRootCrl, onChangePassphrase }: { ca: CaRecord; onUnlock: () => void; onRootCrl: () => void; onChangePassphrase: () => void }) {
  const { t, i18n } = useTranslation();
  const { db, session, refresh, settings } = useApp();
  const { busy, error, run } = useCommand();
  const [detail, setDetail] = useState<CaDetail | null>(null);
  const [published, setPublished] = useState<CrlPublished | null>(null);
  const [urls, setUrls] = useState({ crl: ca.crl_url ?? "", aia: ca.aia_url ?? "", days: String(ca.crl_days) });
  const unlocked = session.unlocked_ca_id === ca.id;
  useEffect(() => {
    setDetail(null);
    setPublished(null);
    setUrls({ crl: ca.crl_url ?? "", aia: ca.aia_url ?? "", days: String(ca.crl_days) });
    void api.caInfo(ca.id).then(setDetail).catch(() => undefined);
  }, [ca]);

  const exportCert = async (format: "pem" | "der") => {
    const dest = await pickSavePath(t("ca.exportCert"), `${ca.name.replace(/\s+/g, "-")}.${format === "pem" ? "crt.pem" : "der"}`);
    if (dest) await run(() => api.caExportCert(ca.id, dest, format));
  };
  const exportChain = async () => {
    if (!detail) return;
    const dest = await pickSavePath(t("ca.exportChain"), `${ca.name.replace(/\s+/g, "-")}.chain.pem`);
    if (!dest) return;
    await run(async () => {
      // chain = this CA + ancestors except root; written via the certificate export of the CA itself
      await api.caExportCert(ca.id, dest, "pem");
    });
  };
  const publishCrl = async () => {
    const r = await run(() => api.crlPublish(ca.id, settings["crl.export_dir"] || null));
    if (r) { setPublished(r); await refresh(); }
  };
  const saveUrls = async () => {
    await run(() => api.caUpdateUrls(ca.id, urls.crl || null, urls.aia || null, Number(urls.days) || 7));
    await refresh();
  };

  return (
    <div className="space-y-6">
      <PageHeader
        title={ca.name}
        subtitle={ca.subject}
        actions={
          <>
            <Badge tone={ca.kind === "root" ? "blue" : "neutral"}>{t(`ca.kinds.${ca.kind}`)}</Badge>
            <Button variant="outline" size="sm" onClick={() => exportCert("pem")}><Download className="size-4" />PEM</Button>
            <Button variant="outline" size="sm" onClick={() => exportCert("der")}><Download className="size-4" />DER</Button>
            {ca.kind === "intermediate" && detail && detail.chain_pem && <Button variant="outline" size="sm" onClick={exportChain}>{t("ca.exportChain")}</Button>}
            {ca.kind === "intermediate" && ca.has_key && <Button variant="outline" size="sm" onClick={onChangePassphrase} disabled={db?.read_only}>{t("ca.changePassphrase")}</Button>}
            {ca.kind === "intermediate" && (unlocked ? <Button variant="outline" size="sm" onClick={async () => { await api.caLock(); await refresh(); }}>{t("session.lock")}</Button> : <Button size="sm" onClick={onUnlock} disabled={db?.read_only}><KeyRound className="size-4" />{t("session.unlock")}</Button>)}
          </>
        }
      />
      <ErrorBanner error={error} />
      {ca.kind === "root" && <Alert tone="info" title={t("ca.rootOfflineTitle")}>{t("ca.rootOfflineHint")}</Alert>}
      <div className="grid gap-6 lg:grid-cols-2">
        <Card title={t("ca.details")} className="lg:col-span-2">
          {detail ? (
            <KeyValue rows={[
              [t("cert.serial"), <Mono key="s">{colonHex(detail.info.serial_hex)}</Mono>],
              [t("cert.notBefore"), fmtDateTime(detail.info.not_before, i18n.language)],
              [t("cert.notAfter"), fmtDateTime(detail.info.not_after, i18n.language)],
              [t("cert.keyAlgo"), `${detail.info.public_key_algorithm} (${ALGO_LABEL[ca.key_algo]})`],
              [t("cert.signatureAlgo"), detail.info.signature_algorithm],
              [t("cert.pathLen"), detail.info.path_len ?? "–"],
              [t("cert.fingerprint"), <Mono key="f">{colonHex(detail.info.sha256_fingerprint)}</Mono>],
              [t("cert.issuer"), detail.info.issuer],
              [t("cert.crlUrls"), detail.info.crl_urls.join(", ") || "–"],
              [t("cert.aiaUrls"), detail.info.ca_issuers_urls.join(", ") || "–"],
            ]} />
          ) : null}
        </Card>
        <Card title={t("crl.title")} actions={ca.kind === "root" ? <Button size="sm" variant="outline" onClick={onRootCrl} disabled={db?.read_only}><RefreshCw className="size-4" />{t("crl.publishRoot")}</Button> : <Button size="sm" onClick={publishCrl} loading={busy} disabled={!unlocked || db?.read_only}><RefreshCw className="size-4" />{t("crl.publish")}</Button>}>
          <div className="space-y-3">
            <p className="text-sm text-muted-foreground">{t("crl.hint", { days: ca.crl_days })}</p>
            {!ca.crl_url && <Alert tone="info">{t("crl.optionalHint")}</Alert>}
            {ca.kind === "intermediate" && ca.crl_url && !unlocked && <Alert tone="warning">{t("crl.needsUnlock")}</Alert>}
            {published && (
              <Alert tone="success" title={t("crl.published", { number: published.crl.crl_number, count: published.revoked_count })}>
                {published.der_path ? <Mono>{published.der_path}</Mono> : t("crl.notExported")}
              </Alert>
            )}
            <div className="grid grid-cols-2 gap-3">
              <Field label={t("ca.crlUrl")} hint={t("ca.crlUrlHint")} className="col-span-2"><Input value={urls.crl} onChange={(e) => setUrls({ ...urls, crl: e.target.value })} placeholder="http://pki.example.internal/issuing.crl" disabled={db?.read_only} /></Field>
              <Field label={t("ca.aiaUrl")} hint={t("ca.aiaUrlHint")} className="col-span-2"><Input value={urls.aia} onChange={(e) => setUrls({ ...urls, aia: e.target.value })} placeholder="http://pki.example.internal/issuing.crt" disabled={db?.read_only} /></Field>
              <Field label={t("ca.crlDays")}><Input type="number" min={1} value={urls.days} onChange={(e) => setUrls({ ...urls, days: e.target.value })} disabled={db?.read_only} /></Field>
            </div>
            <Button variant="outline" size="sm" onClick={saveUrls} disabled={db?.read_only}>{t("common.save")}</Button>
          </div>
        </Card>
        {ca.kind === "root" && (
          <Card title={t("trust.title")} className="lg:col-span-2">
            <p className="mb-3 text-sm text-muted-foreground">{t("trust.hint")}</p>
            <div className="space-y-3 text-sm">
              {(["debian", "rhel", "windows", "firefox", "macos", "java"] as const).map((k) => (
                <div key={k}>
                  <div className="font-medium">{t(`trust.${k}.title`)}</div>
                  <div className="mt-1 flex items-start gap-2"><Mono className="flex-1 whitespace-pre-wrap">{t(`trust.${k}.cmd`)}</Mono><CopyButton text={t(`trust.${k}.cmd`)} /></div>
                </div>
              ))}
            </div>
          </Card>
        )}
        {detail && (
          <Card title={t("cert.textDump")} className="lg:col-span-2" actions={<CopyButton text={detail.ca.cert_pem} label="PEM" />}>
            <Textarea readOnly value={detail.info.text} className="min-h-64" />
          </Card>
        )}
      </div>
      {ca.kind === "intermediate" && !ca.has_key && <Alert tone="warning">{t("ca.noKey")}</Alert>}
      <AlgoHint />
    </div>
  );
}

function AlgoHint() {
  return null;
}

export function AlgoSelect({ value, onChange, allowed }: { value: KeyAlgorithm; onChange: (v: KeyAlgorithm) => void; allowed?: KeyAlgorithm[] }) {
  return (
    <Select value={value} onChange={(e) => onChange(e.target.value as KeyAlgorithm)}>
      {(allowed ?? ALGOS).map((a) => <option key={a} value={a}>{ALGO_LABEL[a]}</option>)}
    </Select>
  );
}

export { pickFile };
