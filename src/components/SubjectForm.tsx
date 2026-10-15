// SPDX-License-Identifier: GPL-3.0-or-later
import { useTranslation } from "react-i18next";
import { Field, Input } from "@/components/ui";
import type { Subject } from "@/api/types";

export function SubjectForm({ value, onChange, cnLabel, cnHint }: { value: Subject; onChange: (s: Subject) => void; cnLabel?: string; cnHint?: string }) {
  const { t } = useTranslation();
  const set = (k: keyof Subject) => (e: React.ChangeEvent<HTMLInputElement>) => onChange({ ...value, [k]: e.target.value });
  return (
    <div className="grid grid-cols-2 gap-3">
      <Field label={cnLabel ?? t("subject.cn")} hint={cnHint} className="col-span-2">
        <Input value={value.common_name} onChange={set("common_name")} />
      </Field>
      <Field label={t("subject.o")}>
        <Input value={value.organization} onChange={set("organization")} />
      </Field>
      <Field label={t("subject.ou")}>
        <Input value={value.organizational_unit} onChange={set("organizational_unit")} />
      </Field>
      <Field label={t("subject.c")}>
        <Input value={value.country} maxLength={2} onChange={set("country")} placeholder="CH" />
      </Field>
      <Field label={t("subject.st")}>
        <Input value={value.state} onChange={set("state")} />
      </Field>
      <Field label={t("subject.l")} className="col-span-2">
        <Input value={value.locality} onChange={set("locality")} />
      </Field>
    </div>
  );
}
