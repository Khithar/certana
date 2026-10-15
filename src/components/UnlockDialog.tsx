// SPDX-License-Identifier: GPL-3.0-or-later
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { api } from "@/api";
import type { CaRecord } from "@/api/types";
import { useApp, useCommand } from "@/context/AppContext";
import { Button, Field, Input, Modal } from "@/components/ui";
import { ErrorBanner } from "@/components/ErrorBanner";

export function UnlockDialog({ ca, open, onClose, onUnlocked }: { ca: CaRecord | null; open: boolean; onClose: () => void; onUnlocked?: () => void }) {
  const { t } = useTranslation();
  const { refresh } = useApp();
  const { busy, error, run } = useCommand();
  const [pass, setPass] = useState("");
  if (!ca) return null;
  const submit = async () => {
    const r = await run(() => api.caUnlock(ca.id, pass));
    if (r) {
      setPass("");
      await refresh();
      onUnlocked?.();
      onClose();
    }
  };
  return (
    <Modal open={open} onClose={onClose} title={t("session.unlockTitle", { name: ca.name })} footer={<><Button variant="outline" onClick={onClose}>{t("common.cancel")}</Button><Button onClick={submit} loading={busy} disabled={!pass}>{t("session.unlock")}</Button></>}>
      <form className="space-y-4" onSubmit={(e) => { e.preventDefault(); void submit(); }}>
        <p className="text-sm text-muted-foreground">{t("session.unlockHint")}</p>
        <Field label={t("session.passphrase")}>
          <Input type="password" value={pass} autoFocus onChange={(e) => setPass(e.target.value)} autoComplete="current-password" />
        </Field>
        <ErrorBanner error={error} />
      </form>
    </Modal>
  );
}

/** Returns a helper that makes sure `caId` is unlocked before running `fn`. */
export function useEnsureUnlocked() {
  const { session, cas } = useApp();
  const [pending, setPending] = useState<{ ca: CaRecord; fn: () => void } | null>(null);
  const ensure = (caId: number, fn: () => void) => {
    if (session.unlocked_ca_id === caId) return fn();
    const ca = cas.find((c) => c.id === caId);
    if (ca) setPending({ ca, fn });
  };
  const dialog = <UnlockDialog ca={pending?.ca ?? null} open={!!pending} onClose={() => setPending(null)} onUnlocked={() => { const f = pending?.fn; setPending(null); f?.(); }} />;
  return { ensure, dialog };
}
