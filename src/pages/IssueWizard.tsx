// SPDX-License-Identifier: GPL-3.0-or-later
// Five-step wizard: profile → identity → key → review → issue.
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { api } from "@/api";
import type { CaRecord, CsrInfo, KeyAlgorithm, Profile, ProfileInfo, San, Subject } from "@/api/types";
import { emptySubject } from "@/api/types";
import { pickFile } from "@/api/dialogs";
import { useApp, useCommand } from "@/context/AppContext";
import { Alert, Badge, Button, Field, Input, KeyValue, Modal, Select, Steps, Textarea } from "@/components/ui";
import { ErrorBanner } from "@/components/ErrorBanner";
import { SubjectForm } from "@/components/SubjectForm";
import { useEnsureUnlocked } from "@/components/UnlockDialog";
import { AlgoSelect, ALGO_LABEL } from "@/pages/Cas";
import { parseSans, sanLabel } from "@/lib/format";

const PROFILES: Profile[] = ["server", "client", "smime", "code-signing"];

export function IssueWizard({ open, onClose, issuing, defaultCaId, onIssued }: { open: boolean; onClose: () => void; issuing: CaRecord[]; defaultCaId: number | null; onIssued: (id: number) => void }) {
  const { t } = useTranslation();
  const { settings } = useApp();
  const { busy, error, run, clearError } = useCommand();
  const { ensure, dialog } = useEnsureUnlocked();
  const [profiles, setProfiles] = useState<ProfileInfo[]>([]);
  const [step, setStep] = useState(0);
  const [caId, setCaId] = useState<number | null>(defaultCaId);
  const [profile, setProfile] = useState<Profile>("server");
  const [subject, setSubject] = useState<Subject>(emptySubject());
  const [sanText, setSanText] = useState("");
  const [keyMode, setKeyMode] = useState<"generate" | "csr">("generate");
  const [algorithm, setAlgorithm] = useState<KeyAlgorithm>("ecdsa-p256");
  const [csrPem, setCsrPem] = useState("");
  const [csrInfo, setCsrInfo] = useState<CsrInfo | null>(null);
  const [days, setDays] = useState("397");
  const [notes, setNotes] = useState("");
  const pinfo = profiles.find((p) => p.profile === profile);
  const sans: San[] = parseSans(sanText);
  const steps = [t("wizard.steps.profile"), t("wizard.steps.identity"), t("wizard.steps.key"), t("wizard.steps.review"), t("wizard.steps.issue")];

  useEffect(() => { void api.certProfiles().then(setProfiles); }, []);
  useEffect(() => {
    if (!open) return;
    setStep(0); setCsrPem(""); setCsrInfo(null); setNotes(""); setSanText(""); clearError();
    setCaId(defaultCaId);
    setSubject({ ...emptySubject(), organization: settings["subject.organization"] ?? "", organizational_unit: settings["subject.organizational_unit"] ?? "", country: settings["subject.country"] ?? "", state: settings["subject.state"] ?? "", locality: settings["subject.locality"] ?? "" });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);
  useEffect(() => { if (pinfo) { setDays(String(pinfo.default_validity_days)); setAlgorithm(pinfo.default_key_algorithm); } }, [pinfo]);

  const parseCsr = async (pem: string) => {
    setCsrPem(pem);
    setCsrInfo(null);
    if (!pem.includes("BEGIN CERTIFICATE REQUEST")) return;
    const info = await run(() => api.certParseCsr(pem));
    if (info) {
      setCsrInfo(info);
      if (info.sans.length) setSanText(info.sans.map((s) => s.value).join(", "));
      if (info.subject.common_name && !sanText) setSanText((prev) => prev || info.subject.common_name);
      setSubject((prev) => ({ ...prev, common_name: info.subject.common_name || prev.common_name }));
    }
  };
  const loadCsrFile = async () => {
    const p = await pickFile(t("wizard.csrFile"), [{ name: "CSR", extensions: ["csr", "pem", "req"] }]);
    if (!p) return;
    // The file is read through the Rust side to keep the fs plugin out of the webview.
    const text = await run(async () => { const { readTextFile } = await import("@/api/readfile"); return readTextFile(p); });
    if (text) await parseCsr(text);
  };

  const sanProblem = profile === "server" && !sans.some((s) => s.kind === "dns" || s.kind === "ip") ? t("wizard.needHost") : profile === "smime" && !sans.some((s) => s.kind === "email") ? t("wizard.needEmail") : null;
  const daysNum = Number(days);
  const canNext = [!!caId, !sanProblem && (sans.length > 0 || subject.common_name.trim().length > 0), keyMode === "generate" || !!csrInfo, daysNum > 0 && daysNum <= (pinfo?.max_validity_days ?? 825), true][step];

  const issue = () => {
    if (!caId) return;
    ensure(caId, async () => {
      const r = await run(() => api.certIssue({ ca_id: caId, profile, subject, sans, validity_days: daysNum, key: keyMode === "generate" ? { kind: "generate", algorithm } : { kind: "csr", csr_pem: csrPem }, notes: notes || null }));
      if (r) onIssued(r.id);
    });
  };

  return (
    <>
      <Modal open={open} onClose={onClose} title={t("wizard.title")} wide footer={<><Button variant="outline" onClick={() => (step === 0 ? onClose() : setStep(step - 1))}>{step === 0 ? t("common.cancel") : t("common.back")}</Button>{step < 4 ? <Button onClick={() => setStep(step + 1)} disabled={!canNext}>{t("common.next")}</Button> : <Button onClick={issue} loading={busy}>{t("wizard.issue")}</Button>}</>}>
        <Steps steps={steps} current={step} />
        {step === 0 && (
          <div className="space-y-4">
            <Field label={t("cert.ca")}><Select value={caId ?? ""} onChange={(e) => setCaId(Number(e.target.value))}>{issuing.map((c) => <option key={c.id} value={c.id}>{c.name}</option>)}</Select></Field>
            <div className="grid grid-cols-2 gap-3">
              {PROFILES.map((p) => (
                <button key={p} type="button" onClick={() => setProfile(p)} className={`rounded-lg border p-4 text-left transition-colors ${profile === p ? "border-primary bg-primary/10" : "border-border hover:bg-muted"}`}>
                  <div className="font-medium">{t(`profile.${p}`)}</div>
                  <div className="mt-1 text-xs text-muted-foreground">{t(`profileHint.${p}`)}</div>
                </button>
              ))}
            </div>
          </div>
        )}
        {step === 1 && (
          <div className="space-y-4">
            <Field label={t("wizard.sans")} hint={t(`wizard.sansHint.${profile}`)} error={sanProblem ?? undefined}>
              <Input value={sanText} onChange={(e) => setSanText(e.target.value)} placeholder={profile === "smime" ? "alice@example.internal" : "www.example.internal, example.internal, 10.0.0.5"} autoFocus />
            </Field>
            {sans.length > 0 && <div className="flex flex-wrap gap-1">{sans.map((s, i) => <Badge key={i} tone="blue">{sanLabel(s)}</Badge>)}</div>}
            <SubjectForm value={subject} onChange={setSubject} cnHint={t("wizard.cnHint")} />
          </div>
        )}
        {step === 2 && (
          <div className="space-y-4">
            <div className="grid grid-cols-2 gap-3">
              <button type="button" onClick={() => setKeyMode("generate")} className={`rounded-lg border p-4 text-left ${keyMode === "generate" ? "border-primary bg-primary/10" : "border-border hover:bg-muted"}`}><div className="font-medium">{t("wizard.keyGenerate")}</div><div className="mt-1 text-xs text-muted-foreground">{t("wizard.keyGenerateHint")}</div></button>
              <button type="button" onClick={() => setKeyMode("csr")} className={`rounded-lg border p-4 text-left ${keyMode === "csr" ? "border-primary bg-primary/10" : "border-border hover:bg-muted"}`}><div className="font-medium">{t("wizard.keyCsr")}</div><div className="mt-1 text-xs text-muted-foreground">{t("wizard.keyCsrHint")}</div></button>
            </div>
            {keyMode === "generate" ? (
              <Field label={t("cert.keyAlgo")} hint={algorithm === "ed25519" ? t("wizard.ed25519Warn") : undefined}><AlgoSelect value={algorithm} onChange={setAlgorithm} allowed={pinfo?.key_algorithms} /></Field>
            ) : (
              <>
                <Field label={t("wizard.csrPem")}><Textarea value={csrPem} onChange={(e) => void parseCsr(e.target.value)} placeholder="-----BEGIN CERTIFICATE REQUEST-----" className="min-h-40" /></Field>
                <Button variant="outline" size="sm" onClick={loadCsrFile}>{t("wizard.csrFile")}</Button>
                {csrInfo && <Alert tone="success" title={t("wizard.csrOk")}>{t("wizard.csrKey", { algo: csrInfo.key_algorithm })} · CN={csrInfo.subject.common_name || "–"} · {csrInfo.sans.map(sanLabel).join(", ") || t("wizard.csrNoSans")}<div className="mt-1 text-xs">{t("wizard.csrPolicy")}</div></Alert>}
              </>
            )}
          </div>
        )}
        {step === 3 && (
          <div className="space-y-4">
            <Field label={t("wizard.validity")} hint={t("wizard.validityHint", { max: pinfo?.max_validity_days ?? 825 })}><Input type="number" min={1} max={pinfo?.max_validity_days} value={days} onChange={(e) => setDays(e.target.value)} /></Field>
            <Field label={t("cert.notes")}><Input value={notes} onChange={(e) => setNotes(e.target.value)} /></Field>
            <KeyValue rows={[
              [t("cert.ca"), issuing.find((c) => c.id === caId)?.name ?? "–"],
              [t("cert.profile"), t(`profile.${profile}`)],
              [t("wizard.sans"), sans.map(sanLabel).join(", ") || "–"],
              [t("cert.subject"), [subject.common_name && `CN=${subject.common_name}`, subject.organization && `O=${subject.organization}`, subject.organizational_unit && `OU=${subject.organizational_unit}`, subject.locality && `L=${subject.locality}`, subject.state && `ST=${subject.state}`, subject.country && `C=${subject.country}`].filter(Boolean).join(", ") || t("wizard.cnFromSan")],
              [t("wizard.key"), keyMode === "generate" ? `${t("wizard.keyGenerate")} · ${ALGO_LABEL[algorithm]}` : `${t("wizard.keyCsr")} · ${csrInfo?.key_algorithm ?? ""}`],
              [t("wizard.validity"), `${days} ${t("common.days")}`],
            ]} />
          </div>
        )}
        {step === 4 && (
          <div className="space-y-4">
            <Alert tone="info">{t("wizard.issueHint")}</Alert>
            <ErrorBanner error={error} />
          </div>
        )}
        {step !== 4 && <div className="mt-4"><ErrorBanner error={error} /></div>}
      </Modal>
      {dialog}
    </>
  );
}
