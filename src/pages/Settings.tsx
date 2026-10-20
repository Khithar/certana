// SPDX-License-Identifier: GPL-3.0-or-later
import { useEffect, useState } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useTranslation } from "react-i18next";
import { api } from "@/api";
import { SETTING_KEYS } from "@/api/types";
import type { AppInfo } from "@/api/types";
import { THEMES, getTheme, setTheme, type Theme } from "@/lib/theme";
import { pickDirectory, pickFile, pickSavePath } from "@/api/dialogs";
import { useApp, useCommand } from "@/context/AppContext";
import { Alert, Button, Card, Checkbox, Field, Input, PageHeader, Select } from "@/components/ui";
import { ErrorBanner } from "@/components/ErrorBanner";

/** Shown by every page when no database is open. */
export function NoDatabase() {
  const { t } = useTranslation();
  return (
    <div className="p-8">
      <PageHeader title={t("db.title")} subtitle={t("db.noneHint")} />
      <DatabaseCard />
    </div>
  );
}

function DatabaseCard() {
  const { t } = useTranslation();
  const { db, lastDbPath, openDb, closeDb, error } = useApp();
  const { busy, error: cmdError, run } = useCommand();
  const [path, setPath] = useState("");
  const [forceShared, setForceShared] = useState(false);
  const [backupDone, setBackupDone] = useState<string | null>(null);
  useEffect(() => {
    if (db) setPath(db.path);
    else if (lastDbPath) setPath(lastDbPath);
    else void api.dbDefaultPath().then(setPath);
  }, [db, lastDbPath]);

  const choose = async () => {
    const p = await pickSavePath(t("db.choose"), path, [{ name: "SQLite", extensions: ["db"] }]);
    if (p) setPath(p);
  };
  const chooseExisting = async () => {
    const p = await pickFile(t("db.openExisting"), [{ name: "SQLite", extensions: ["db"] }]);
    if (p) setPath(p);
  };
  const backup = async () => {
    const dest = await pickSavePath(t("db.backup"), `certana-backup-${new Date().toISOString().slice(0, 10)}.db`);
    if (!dest) return;
    const r = await run(() => api.dbBackup(dest));
    if (r) setBackupDone(r);
  };

  return (
    <Card title={t("db.title")}>
      <div className="space-y-4">
        {db?.read_only && db.held_by && (
          <Alert tone="warning" title={t("db.readOnly")}>{t("db.heldBy", { user: db.held_by.user, host: db.held_by.host })}</Alert>
        )}
        {db?.lock_lost && <Alert tone="warning" title={t("db.lockLostTitle")}>{t("db.lockLost")}</Alert>}
        {db?.mode === "shared" ? <Alert tone="info">{t("db.sharedHint")}</Alert> : <Alert tone="info" title={t("db.teamTitle")}>{t("db.teamHint")}</Alert>}
        <Field label={t("db.path")} hint={t("db.pathHint")}>
          <div className="flex gap-2">
            <Input value={path} onChange={(e) => setPath(e.target.value)} />
            <Button variant="outline" onClick={chooseExisting}>{t("common.browse")}</Button>
            <Button variant="outline" onClick={choose}>{t("db.new")}</Button>
          </div>
        </Field>
        <Checkbox label={t("db.forceShared")} checked={forceShared} onChange={setForceShared} />
        <div className="flex flex-wrap gap-2">
          <Button loading={busy} onClick={() => run(() => openDb(path, forceShared))}>{db ? t("db.switch") : t("db.open")}</Button>
          {db && <Button variant="outline" onClick={() => run(closeDb)}>{t("db.close")}</Button>}
          {db && <Button variant="outline" onClick={backup}>{t("db.backup")}</Button>}
        </div>
        {backupDone && <Alert tone="success">{t("db.backupDone", { path: backupDone })}</Alert>}
        <ErrorBanner error={cmdError ?? error} />
      </div>
    </Card>
  );
}

function AppearanceCard() {
  const { t } = useTranslation();
  const [theme, setThemeState] = useState<Theme>(getTheme());
  const change = (v: Theme) => { setTheme(v); setThemeState(v); };
  return (
    <Card title={t("settings.appearance")}>
      <div className="grid grid-cols-2 gap-3">
        <Field label={t("settings.theme")}>
          <Select value={theme} onChange={(e) => change(e.target.value as Theme)}>
            {THEMES.map((x) => <option key={x} value={x}>{t(`settings.themes.${x}`)}</option>)}
          </Select>
        </Field>
      </div>
    </Card>
  );
}

function AboutCard() {
  const { t } = useTranslation();
  const [info, setInfo] = useState<AppInfo | null>(null);
  useEffect(() => { void api.appInfo().then(setInfo).catch(() => undefined); }, []);
  if (!info) return null;
  return (
    <Card title={t("settings.about")}>
      <div className="grid grid-cols-[auto_1fr] gap-x-6 gap-y-1 text-sm">
        <span className="text-muted-foreground">{t("settings.version")}</span><span>{info.name} {info.version}</span>
        <span className="text-muted-foreground">{t("settings.license")}</span><span>{info.license}</span>
        <span className="text-muted-foreground">{t("settings.repository")}</span>
        <button type="button" className="text-left text-primary hover:underline" onClick={() => void openUrl(info.repository)}>{info.repository}</button>
      </div>
      <p className="mt-3 text-xs text-muted-foreground">{t("settings.licenseHint")}</p>
    </Card>
  );
}

export default function Settings() {
  const { t, i18n } = useTranslation();
  const { db, settings, setSetting } = useApp();
  const { error, run } = useCommand();
  const [draft, setDraft] = useState<Record<string, string>>({});
  const [saved, setSaved] = useState(false);
  useEffect(() => setDraft(settings), [settings]);
  const v = (k: string) => draft[k] ?? "";
  const set = (k: string) => (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) => setDraft({ ...draft, [k]: e.target.value });

  const save = async () => {
    const changed = Object.entries(draft).filter(([k, val]) => (settings[k] ?? "") !== val);
    await run(async () => {
      for (const [k, val] of changed) await setSetting(k, val);
    });
    if (draft[SETTING_KEYS.language]) void i18n.changeLanguage(draft[SETTING_KEYS.language]);
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };
  const auditExport = async () => {
    const dest = await pickSavePath(t("settings.auditExport"), "certana-audit.csv", [{ name: "CSV", extensions: ["csv"] }]);
    if (dest) await run(() => api.auditExportCsv(dest));
  };

  return (
    <div className="space-y-6 p-8">
      <PageHeader title={t("nav.settings")} />
      <DatabaseCard />
      <AppearanceCard />
      {db && (
        <>
          <Card title={t("settings.subjectDefaults")}>
            <div className="grid grid-cols-2 gap-3">
              <Field label={t("subject.o")}><Input value={v(SETTING_KEYS.organization)} onChange={set(SETTING_KEYS.organization)} disabled={db.read_only} /></Field>
              <Field label={t("subject.ou")}><Input value={v(SETTING_KEYS.organizationalUnit)} onChange={set(SETTING_KEYS.organizationalUnit)} disabled={db.read_only} /></Field>
              <Field label={t("subject.c")}><Input value={v(SETTING_KEYS.country)} maxLength={2} onChange={set(SETTING_KEYS.country)} disabled={db.read_only} /></Field>
              <Field label={t("subject.st")}><Input value={v(SETTING_KEYS.state)} onChange={set(SETTING_KEYS.state)} disabled={db.read_only} /></Field>
              <Field label={t("subject.l")} className="col-span-2"><Input value={v(SETTING_KEYS.locality)} onChange={set(SETTING_KEYS.locality)} disabled={db.read_only} /></Field>
            </div>
          </Card>
          <Card title={t("settings.paths")}>
            <div className="space-y-3">
              <DirField label={t("settings.exportDir")} value={v(SETTING_KEYS.exportDir)} onChange={(x) => setDraft({ ...draft, [SETTING_KEYS.exportDir]: x })} disabled={db.read_only} />
              <DirField label={t("settings.crlExportDir")} hint={t("settings.crlExportDirHint")} value={v(SETTING_KEYS.crlExportDir)} onChange={(x) => setDraft({ ...draft, [SETTING_KEYS.crlExportDir]: x })} disabled={db.read_only} />
            </div>
          </Card>
          <Card title={t("settings.app")}>
            <div className="grid grid-cols-2 gap-3">
              <Field label={t("settings.autoLock")} hint={t("settings.autoLockHint")}><Input type="number" min={1} value={v(SETTING_KEYS.autoLockMinutes) || "15"} onChange={set(SETTING_KEYS.autoLockMinutes)} disabled={db.read_only} /></Field>
              <Field label={t("settings.expiryWarn")}><Input type="number" min={1} value={v(SETTING_KEYS.expiryWarnDays) || "14"} onChange={set(SETTING_KEYS.expiryWarnDays)} disabled={db.read_only} /></Field>
              <Field label={t("settings.language")}>
                <Select value={v(SETTING_KEYS.language) || i18n.language.slice(0, 2)} onChange={set(SETTING_KEYS.language)} disabled={db.read_only}>
                  <option value="en">English</option>
                  <option value="de">Deutsch</option>
                </Select>
              </Field>
            </div>
          </Card>
          <div className="flex items-center gap-3">
            <Button onClick={save} disabled={db.read_only}>{t("common.save")}</Button>
            <Button variant="outline" onClick={auditExport}>{t("settings.auditExport")}</Button>
            {saved && <span className="text-sm text-emerald-600">{t("common.saved")}</span>}
          </div>
          <ErrorBanner error={error} />
        </>
      )}
      <AboutCard />
    </div>
  );
}

function DirField({ label, hint, value, onChange, disabled }: { label: string; hint?: string; value: string; onChange: (v: string) => void; disabled?: boolean }) {
  const { t } = useTranslation();
  return (
    <Field label={label} hint={hint}>
      <div className="flex gap-2">
        <Input value={value} onChange={(e) => onChange(e.target.value)} disabled={disabled} />
        <Button variant="outline" disabled={disabled} onClick={async () => { const d = await pickDirectory(label); if (d) onChange(d); }}>{t("common.browse")}</Button>
      </div>
    </Field>
  );
}
