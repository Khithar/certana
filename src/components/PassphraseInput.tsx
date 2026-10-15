// SPDX-License-Identifier: GPL-3.0-or-later
import { useState } from "react";
import { Eye, EyeOff } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button, Field, Input } from "@/components/ui";

export const MIN_PASSPHRASE = 16;

export function strength(p: string): 0 | 1 | 2 | 3 {
  if (p.length < MIN_PASSPHRASE) return 0;
  const classes = [/[a-z]/, /[A-Z]/, /\d/, /[^A-Za-z0-9]/].filter((r) => r.test(p)).length;
  if (p.length >= 24 && classes >= 3) return 3;
  if (p.length >= 20 || classes >= 3) return 2;
  return 1;
}

export function PassphraseInput({ label, value, onChange, confirm, hint, autoFocus }: { label: string; value: string; onChange: (v: string) => void; confirm?: boolean; hint?: string; autoFocus?: boolean }) {
  const { t } = useTranslation();
  const [show, setShow] = useState(false);
  const [second, setSecond] = useState("");
  const s = strength(value);
  const colors = ["bg-red-500", "bg-amber-500", "bg-emerald-500", "bg-emerald-600"];
  const mismatch = confirm && second.length > 0 && second !== value;
  return (
    <div className="flex flex-col gap-3">
      <Field label={label} hint={hint} error={mismatch ? t("passphrase.mismatch") : undefined}>
        <div className="flex gap-2">
          <Input type={show ? "text" : "password"} value={value} autoFocus={autoFocus} onChange={(e) => onChange(e.target.value)} autoComplete="new-password" />
          <Button type="button" variant="outline" size="icon" onClick={() => setShow(!show)} aria-label="toggle">
            {show ? <EyeOff className="size-4" /> : <Eye className="size-4" />}
          </Button>
        </div>
      </Field>
      {confirm && (
        <>
          <div className="flex items-center gap-2">
            <div className="h-1.5 flex-1 overflow-hidden rounded bg-muted">
              <div className={`h-full transition-all ${colors[s]}`} style={{ width: `${((s + (value ? 1 : 0)) / 4) * 100}%` }} />
            </div>
            <span className="w-24 text-right text-xs text-muted-foreground">{t(`passphrase.strength.${s}`)}</span>
          </div>
          <Field label={t("passphrase.confirm")}>
            <Input type={show ? "text" : "password"} value={second} onChange={(e) => setSecond(e.target.value)} autoComplete="new-password" />
          </Field>
        </>
      )}
    </div>
  );
}

export const passphraseOk = (p: string, confirmValue?: string) => p.length >= MIN_PASSPHRASE && (confirmValue === undefined || confirmValue === p);
