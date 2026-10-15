// SPDX-License-Identifier: GPL-3.0-or-later
// Root ceremony, root import, intermediate creation and root-CRL dialogs.
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { api } from "@/api";
import type { CaRecord, KeyAlgorithm, Subject } from "@/api/types";
import { emptySubject } from "@/api/types";
import { pickFile, pickSavePath } from "@/api/dialogs";
import { useApp, useCommand } from "@/context/AppContext";
import { Alert, Button, Checkbox, Field, Input, Modal, Mono, Select, Steps } from "@/components/ui";
import { ErrorBanner } from "@/components/ErrorBanner";
import { PassphraseInput, passphraseOk } from "@/components/PassphraseInput";
import { SubjectForm } from "@/components/SubjectForm";
import { AlgoSelect } from "@/pages/Cas";
import { colonHex } from "@/lib/format";

function useSubjectDefaults(cn: string): Subject {
  const { settings } = useApp();
  return { ...emptySubject(), common_name: cn, organization: settings["subject.organization"] ?? "", organizational_unit: settings["subject.organizational_unit"] ?? "", country: settings["subject.country"] ?? "", state: settings["subject.state"] ?? "", locality: settings["subject.locality"] ?? "" };
}

/* ---------------- Root ceremony ---------------- */
export function RootCeremonyDialog({ open, onClose, onDone }: { open: boolean; onClose: () => void; onDone: (id: number) => void }) {
  const { t } = useTranslation();
  const defaults = useSubjectDefaults(`${useApp().settings["subject.organization"] || "Internal"} Root CA`);
  const { busy, error, run } = useCommand();
  const [step, setStep] = useState(0);
  const [name, setName] = useState("Root CA");
  const [subject, setSubject] = useState<Subject>(defaults);
  const [algorithm, setAlgorithm] = useState<KeyAlgorithm>("ecdsa-p384");
  const [years, setYears] = useState("15");
  const [crlUrl, setCrlUrl] = useState("");
  const [aiaUrl, setAiaUrl] = useState("");
  const [pass, setPass] = useState("");
  const [bundlePath, setBundlePath] = useState("");
  const [offlineAck, setOfflineAck] = useState(false);
  const [result, setResult] = useState<{ fingerprint: string; bundle: string } | null>(null);
  const steps = [t("ceremony.steps.warn"), t("ceremony.steps.params"), t("ceremony.steps.passphrase"), t("ceremony.steps.bundle"), t("ceremony.steps.done")];

  const reset = () => { setStep(0); setResult(null); setPass(""); setOfflineAck(false); };
  const close = () => { reset(); onClose(); };
  const chooseBundle = async () => {
    const p = await pickSavePath(t("ceremony.bundlePath"), `${name.replace(/\s+/g, "-")}.certana-root`, [{ name: "Certana root bundle", extensions: ["certana-root"] }]);
    if (p) setBundlePath(p);
  };
  const create = async () => {
    const r = await run(() => api.caCreateRoot({ name, subject, algorithm, validity_days: Math.round(Number(years) * 365), crl_url: crlUrl || null, aia_url: aiaUrl || null, passphrase: pass, bundle_path: bundlePath }));
    if (r) { setResult({ fingerprint: r.fingerprint_sha256, bundle: r.bundle_path }); setStep(4); onDone(r.ca.id); }
  };
  const next = () => setStep(step + 1);
  const canNext = [offlineAck, name.trim() && subject.common_name.trim() && Number(years) > 0, passphraseOk(pass), bundlePath.trim().length > 0, true][step];

  return (
    <Modal open={open} onClose={close} title={t("ceremony.title")} wide footer={step < 4 ? <><Button variant="outline" onClick={() => (step === 0 ? close() : setStep(step - 1))}>{step === 0 ? t("common.cancel") : t("common.back")}</Button>{step < 3 ? <Button onClick={next} disabled={!canNext}>{t("common.next")}</Button> : <Button onClick={create} loading={busy} disabled={!canNext}>{t("ceremony.create")}</Button>}</> : <Button onClick={close}>{t("common.close")}</Button>}>
      <Steps steps={steps} current={step} />
      {step === 0 && (
        <div className="space-y-4">
          <Alert tone="warning" title={t("ceremony.warnTitle")}>{t("ceremony.warnText")}</Alert>
          <ul className="list-disc space-y-1 pl-5 text-sm">{[1, 2, 3, 4].map((i) => <li key={i}>{t(`ceremony.warnList.${i}`)}</li>)}</ul>
          <Checkbox label={t("ceremony.ack")} checked={offlineAck} onChange={setOfflineAck} />
        </div>
      )}
      {step === 1 && (
        <div className="space-y-4">
          <Field label={t("ca.name")} hint={t("ca.nameHint")}><Input value={name} onChange={(e) => setName(e.target.value)} /></Field>
          <SubjectForm value={subject} onChange={setSubject} />
          <div className="grid grid-cols-2 gap-3">
            <Field label={t("cert.keyAlgo")} hint={t("ceremony.algoHint")}><AlgoSelect value={algorithm} onChange={setAlgorithm} allowed={["ecdsa-p384", "ecdsa-p256", "rsa4096", "rsa3072"]} /></Field>
            <Field label={t("ceremony.years")}><Input type="number" min={1} max={30} value={years} onChange={(e) => setYears(e.target.value)} /></Field>
            <Field label={t("ca.crlUrl")} hint={t("ceremony.rootCrlUrlHint")}><Input value={crlUrl} onChange={(e) => setCrlUrl(e.target.value)} placeholder="http://pki.example.internal/root.crl" /></Field>
            <Field label={t("ca.aiaUrl")}><Input value={aiaUrl} onChange={(e) => setAiaUrl(e.target.value)} placeholder="http://pki.example.internal/root.crt" /></Field>
          </div>
        </div>
      )}
      {step === 2 && (
        <div className="space-y-4">
          <p className="text-sm text-muted-foreground">{t("ceremony.passphraseHint")}</p>
          <PassphraseInput label={t("ceremony.passphrase")} value={pass} onChange={setPass} confirm autoFocus />
        </div>
      )}
      {step === 3 && (
        <div className="space-y-4">
          <p className="text-sm text-muted-foreground">{t("ceremony.bundleHint")}</p>
          <Field label={t("ceremony.bundlePath")}>
            <div className="flex gap-2"><Input value={bundlePath} onChange={(e) => setBundlePath(e.target.value)} /><Button variant="outline" onClick={chooseBundle}>{t("common.browse")}</Button></div>
          </Field>
          <ErrorBanner error={error} />
        </div>
      )}
      {step === 4 && result && (
        <div className="space-y-4">
          <Alert tone="success" title={t("ceremony.doneTitle")}>{t("ceremony.doneText")}</Alert>
          <div className="text-sm"><div className="text-muted-foreground">{t("ceremony.bundlePath")}</div><Mono>{result.bundle}</Mono></div>
          <div className="text-sm"><div className="text-muted-foreground">{t("cert.fingerprint")}</div><Mono>{colonHex(result.fingerprint)}</Mono></div>
          <Alert tone="warning">{t("ceremony.backupReminder")}</Alert>
        </div>
      )}
    </Modal>
  );
}

function FilePick({ label, value, onChange, filters }: { label: string; value: string; onChange: (v: string) => void; filters?: { name: string; extensions: string[] }[] }) {
  const { t } = useTranslation();
  return (
    <Field label={label}><div className="flex gap-2"><Input value={value} onChange={(e) => onChange(e.target.value)} /><Button variant="outline" onClick={async () => { const p = await pickFile(label, filters); if (p) onChange(p); }}>{t("common.browse")}</Button></div></Field>
  );
}

/* ---------------- Root import ---------------- */
export function RootImportDialog({ open, onClose, onDone }: { open: boolean; onClose: () => void; onDone: (id: number) => void }) {
  const { t } = useTranslation();
  const { busy, error, run } = useCommand();
  const [mode, setMode] = useState<"pem" | "p12" | "certonly">("pem");
  const [name, setName] = useState("Imported Root CA");
  const [certPath, setCertPath] = useState("");
  const [keyPath, setKeyPath] = useState("");
  const [keyPass, setKeyPass] = useState("");
  const [p12Path, setP12Path] = useState("");
  const [p12Pass, setP12Pass] = useState("");
  const [bundlePath, setBundlePath] = useState("");
  const [bundlePass, setBundlePass] = useState("");
  const withKey = mode !== "certonly";
  const ok = name.trim() && (mode === "p12" ? p12Path : certPath) && (!withKey || (bundlePath && passphraseOk(bundlePass))) && (mode !== "pem" || keyPath);
  const submit = async () => {
    const r = await run(() => api.caImportRoot({ name, cert_path: certPath, key_path: mode === "pem" ? keyPath : null, key_passphrase: keyPass || null, p12_path: mode === "p12" ? p12Path : null, p12_password: p12Pass || null, bundle_path: withKey ? bundlePath : null, bundle_passphrase: withKey ? bundlePass : null, crl_url: null, aia_url: null }));
    if (r) { onDone(r.ca.id); onClose(); }
  };
  return (
    <Modal open={open} onClose={onClose} title={t("import.title")} footer={<><Button variant="outline" onClick={onClose}>{t("common.cancel")}</Button><Button onClick={submit} loading={busy} disabled={!ok}>{t("import.submit")}</Button></>}>
      <div className="space-y-4">
        <Field label={t("import.mode")}>
          <Select value={mode} onChange={(e) => setMode(e.target.value as typeof mode)}>
            <option value="pem">{t("import.modes.pem")}</option>
            <option value="p12">{t("import.modes.p12")}</option>
            <option value="certonly">{t("import.modes.certonly")}</option>
          </Select>
        </Field>
        <Field label={t("ca.name")}><Input value={name} onChange={(e) => setName(e.target.value)} /></Field>
        {mode !== "p12" && <FilePick label={t("import.certPath")} value={certPath} onChange={setCertPath} filters={[{ name: "PEM", extensions: ["pem", "crt", "cer"] }]} />}
        {mode === "pem" && (
          <>
            <FilePick label={t("import.keyPath")} value={keyPath} onChange={setKeyPath} filters={[{ name: "PEM", extensions: ["pem", "key"] }]} />
            <Field label={t("import.keyPass")} hint={t("import.keyPassHint")}><Input type="password" value={keyPass} onChange={(e) => setKeyPass(e.target.value)} /></Field>
          </>
        )}
        {mode === "p12" && (
          <>
            <FilePick label={t("import.p12Path")} value={p12Path} onChange={setP12Path} filters={[{ name: "PKCS#12", extensions: ["p12", "pfx"] }]} />
            <Field label={t("import.p12Pass")}><Input type="password" value={p12Pass} onChange={(e) => setP12Pass(e.target.value)} /></Field>
          </>
        )}
        {withKey && (
          <>
            <Alert tone="info">{t("import.bundleHint")}</Alert>
            <Field label={t("ceremony.bundlePath")}><div className="flex gap-2"><Input value={bundlePath} onChange={(e) => setBundlePath(e.target.value)} /><Button variant="outline" onClick={async () => { const p = await pickSavePath(t("ceremony.bundlePath"), `${name.replace(/\s+/g, "-")}.certana-root`); if (p) setBundlePath(p); }}>{t("common.browse")}</Button></div></Field>
            <PassphraseInput label={t("ceremony.passphrase")} value={bundlePass} onChange={setBundlePass} confirm />
          </>
        )}
        {mode === "certonly" && <Alert tone="info">{t("import.certOnlyHint")}</Alert>}
        <ErrorBanner error={error} />
      </div>
    </Modal>
  );
}

/* ---------------- Intermediate ---------------- */
export function IntermediateDialog({ open, onClose, roots, onDone }: { open: boolean; onClose: () => void; roots: CaRecord[]; onDone: (id: number | null) => void }) {
  const { t } = useTranslation();
  const defaults = useSubjectDefaults(`${useApp().settings["subject.organization"] || "Internal"} Issuing CA`);
  const { busy, error, run } = useCommand();
  const [flow, setFlow] = useState<"ceremony" | "csr" | "import">("ceremony");
  const [name, setName] = useState("Issuing CA");
  const [subject, setSubject] = useState<Subject>(defaults);
  const [algorithm, setAlgorithm] = useState<KeyAlgorithm>("ecdsa-p384");
  const [years, setYears] = useState("5");
  const [crlUrl, setCrlUrl] = useState("");
  const [aiaUrl, setAiaUrl] = useState("");
  const [crlDays, setCrlDays] = useState("7");
  const [pass, setPass] = useState("");
  const [rootId, setRootId] = useState<number>(roots[0]?.id ?? 0);
  const [bundlePath, setBundlePath] = useState("");
  const [rootPass, setRootPass] = useState("");
  const [csrPath, setCsrPath] = useState("");
  const [certPath, setCertPath] = useState("");
  const [csrDone, setCsrDone] = useState<string | null>(null);
  const root = roots.find((r) => r.id === (rootId || roots[0]?.id));

  const submit = async () => {
    if (flow === "ceremony") {
      const r = await run(() => api.caCreateIntermediate({ name, subject, algorithm, validity_days: Math.round(Number(years) * 365), crl_url: crlUrl || null, aia_url: aiaUrl || null, crl_days: Number(crlDays) || 7, passphrase: pass, root_ca_id: root!.id, root_bundle_path: bundlePath, root_passphrase: rootPass }));
      if (r) { onDone(r.id); onClose(); }
    } else if (flow === "csr") {
      const r = await run(() => api.caCreateIntermediateCsr({ name, subject, algorithm, passphrase: pass, csr_path: csrPath }));
      if (r) setCsrDone(r.csr_path);
    } else {
      const r = await run(() => api.caImportIntermediateCert({ name, cert_path: certPath, root_ca_id: root!.id, crl_url: crlUrl || null, aia_url: aiaUrl || null, crl_days: Number(crlDays) || 7 }));
      if (r) { onDone(r.id); onClose(); }
    }
  };
  const ok = name.trim() && (flow === "import" ? certPath && root : subject.common_name.trim() && passphraseOk(pass) && (flow === "csr" ? csrPath : bundlePath && rootPass && root));
  return (
    <Modal open={open} onClose={onClose} title={t("intermediate.title")} wide footer={<><Button variant="outline" onClick={onClose}>{t("common.cancel")}</Button><Button onClick={submit} loading={busy} disabled={!ok}>{t(`intermediate.submit.${flow}`)}</Button></>}>
      <div className="space-y-4">
        <Field label={t("intermediate.flow")}>
          <Select value={flow} onChange={(e) => setFlow(e.target.value as typeof flow)}>
            <option value="ceremony">{t("intermediate.flows.ceremony")}</option>
            <option value="csr">{t("intermediate.flows.csr")}</option>
            <option value="import">{t("intermediate.flows.import")}</option>
          </Select>
        </Field>
        <p className="text-sm text-muted-foreground">{t(`intermediate.flowHint.${flow}`)}</p>
        <Field label={t("ca.name")} hint={flow === "import" ? t("intermediate.importNameHint") : undefined}><Input value={name} onChange={(e) => setName(e.target.value)} /></Field>
        {flow !== "import" && (
          <>
            <SubjectForm value={subject} onChange={setSubject} />
            <div className="grid grid-cols-2 gap-3">
              <Field label={t("cert.keyAlgo")}><AlgoSelect value={algorithm} onChange={setAlgorithm} allowed={["ecdsa-p384", "ecdsa-p256", "rsa4096", "rsa3072"]} /></Field>
              {flow === "ceremony" && <Field label={t("ceremony.years")}><Input type="number" min={1} max={20} value={years} onChange={(e) => setYears(e.target.value)} /></Field>}
            </div>
            <PassphraseInput label={t("intermediate.passphrase")} hint={t("intermediate.passphraseHint")} value={pass} onChange={setPass} confirm />
          </>
        )}
        {flow !== "csr" && (
          <div className="grid grid-cols-2 gap-3">
            <Field label={t("intermediate.root")} className="col-span-2"><Select value={root?.id ?? ""} onChange={(e) => setRootId(Number(e.target.value))}>{roots.map((r) => <option key={r.id} value={r.id}>{r.name}</option>)}</Select></Field>
            <Field label={t("ca.crlUrl")} hint={t("ca.crlUrlHint")}><Input value={crlUrl} onChange={(e) => setCrlUrl(e.target.value)} placeholder="http://pki.example.internal/issuing.crl" /></Field>
            <Field label={t("ca.aiaUrl")}><Input value={aiaUrl} onChange={(e) => setAiaUrl(e.target.value)} placeholder="http://pki.example.internal/issuing.crt" /></Field>
            <Field label={t("ca.crlDays")}><Input type="number" min={1} value={crlDays} onChange={(e) => setCrlDays(e.target.value)} /></Field>
          </div>
        )}
        {flow === "ceremony" && (
          <>
            <Alert tone="warning" title={t("ceremony.signTitle")}>{t("ceremony.signHint")}</Alert>
            <Field label={t("ceremony.bundlePath")}><div className="flex gap-2"><Input value={bundlePath} onChange={(e) => setBundlePath(e.target.value)} /><Button variant="outline" onClick={async () => { const p = await pickFile(t("ceremony.bundlePath"), [{ name: "Certana root bundle", extensions: ["certana-root"] }]); if (p) setBundlePath(p); }}>{t("common.browse")}</Button></div></Field>
            <Field label={t("ceremony.rootPassphrase")}><Input type="password" value={rootPass} onChange={(e) => setRootPass(e.target.value)} /></Field>
          </>
        )}
        {flow === "csr" && (
          <Field label={t("intermediate.csrPath")}><div className="flex gap-2"><Input value={csrPath} onChange={(e) => setCsrPath(e.target.value)} /><Button variant="outline" onClick={async () => { const p = await pickSavePath(t("intermediate.csrPath"), `${name.replace(/\s+/g, "-")}.csr.pem`); if (p) setCsrPath(p); }}>{t("common.browse")}</Button></div></Field>
        )}
        {flow === "import" && (
          <Field label={t("intermediate.certPath")}><div className="flex gap-2"><Input value={certPath} onChange={(e) => setCertPath(e.target.value)} /><Button variant="outline" onClick={async () => { const p = await pickFile(t("intermediate.certPath"), [{ name: "PEM", extensions: ["pem", "crt", "cer"] }]); if (p) setCertPath(p); }}>{t("common.browse")}</Button></div></Field>
        )}
        {csrDone && <Alert tone="success" title={t("intermediate.csrDone")}><Mono>{csrDone}</Mono><div className="mt-1">{t("intermediate.csrDoneHint")}</div></Alert>}
        <ErrorBanner error={error} />
      </div>
    </Modal>
  );
}

/* ---------------- Change issuing-CA passphrase ---------------- */
export function ChangePassphraseDialog({ ca, open, onClose, onDone }: { ca: CaRecord | null; open: boolean; onClose: () => void; onDone: () => void }) {
  const { t } = useTranslation();
  const { busy, error, run } = useCommand();
  const [oldPass, setOldPass] = useState("");
  const [newPass, setNewPass] = useState("");
  const [done, setDone] = useState<number | null>(null);
  if (!ca) return null;
  const close = () => { setOldPass(""); setNewPass(""); setDone(null); onClose(); };
  const submit = async () => {
    const r = await run(() => api.caChangePassphrase({ ca_id: ca.id, old_passphrase: oldPass, new_passphrase: newPass }));
    if (r) { setDone(r.rewrapped_leaf_keys); setOldPass(""); setNewPass(""); onDone(); }
  };
  const ok = oldPass.length > 0 && passphraseOk(newPass) && oldPass !== newPass;
  return (
    <Modal open={open} onClose={close} title={t("ca.changePassphrase")} footer={<><Button variant="outline" onClick={close}>{t("common.close")}</Button>{done === null && <Button onClick={submit} loading={busy} disabled={!ok}>{t("ca.changePassphrase")}</Button>}</>}>
      <form className="space-y-4" onSubmit={(e) => { e.preventDefault(); if (ok && done === null) void submit(); }}>
        <p className="text-sm text-muted-foreground">{t("ca.changePassphraseHint")}</p>
        {done === null ? (
          <>
            <Field label={t("ca.oldPassphrase")}><Input type="password" value={oldPass} autoFocus onChange={(e) => setOldPass(e.target.value)} autoComplete="current-password" /></Field>
            <PassphraseInput label={t("ca.newPassphrase")} hint={t("intermediate.passphraseHint")} value={newPass} onChange={setNewPass} confirm />
          </>
        ) : (
          <Alert tone="success">{t("ca.passphraseChanged", { count: done })}</Alert>
        )}
        <ErrorBanner error={error} />
      </form>
    </Modal>
  );
}

/* ---------------- Root CRL ceremony ---------------- */
export function RootCrlDialog({ ca, open, onClose, onDone }: { ca: CaRecord | null; open: boolean; onClose: () => void; onDone: () => void }) {
  const { t } = useTranslation();
  const { settings } = useApp();
  const { busy, error, run } = useCommand();
  const [bundlePath, setBundlePath] = useState("");
  const [pass, setPass] = useState("");
  const [done, setDone] = useState<string | null>(null);
  if (!ca) return null;
  const submit = async () => {
    const r = await run(() => api.crlPublishRoot(ca.id, bundlePath, pass, settings["crl.export_dir"] || null));
    if (r) { setDone(t("crl.published", { number: r.crl.crl_number, count: r.revoked_count })); setPass(""); onDone(); }
  };
  return (
    <Modal open={open} onClose={onClose} title={t("crl.publishRoot")} footer={<><Button variant="outline" onClick={onClose}>{t("common.close")}</Button><Button onClick={submit} loading={busy} disabled={!bundlePath || !pass}>{t("crl.publish")}</Button></>}>
      <div className="space-y-4">
        <Alert tone="warning" title={t("ceremony.signTitle")}>{t("crl.rootHint")}</Alert>
        <Field label={t("ceremony.bundlePath")}><div className="flex gap-2"><Input value={bundlePath} onChange={(e) => setBundlePath(e.target.value)} /><Button variant="outline" onClick={async () => { const p = await pickFile(t("ceremony.bundlePath"), [{ name: "Certana root bundle", extensions: ["certana-root"] }]); if (p) setBundlePath(p); }}>{t("common.browse")}</Button></div></Field>
        <Field label={t("ceremony.rootPassphrase")}><Input type="password" value={pass} onChange={(e) => setPass(e.target.value)} /></Field>
        {done && <Alert tone="success">{done}</Alert>}
        <ErrorBanner error={error} />
      </div>
    </Modal>
  );
}
