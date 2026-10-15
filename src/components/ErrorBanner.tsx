// SPDX-License-Identifier: GPL-3.0-or-later
import { useTranslation } from "react-i18next";
import { Alert } from "@/components/ui";
import type { CommandError } from "@/api/types";

/** Shows a command error with an i18n'd headline for well-known codes. */
export function ErrorBanner({ error }: { error: CommandError | null }) {
  const { t } = useTranslation();
  if (!error) return null;
  const known = ["no-database", "locked", "wrong-passphrase", "vault", "invalid", "pki", "db", "io"];
  const title = known.includes(error.code) ? t(`errors.${error.code}`) : t("errors.unknown");
  return (
    <Alert tone="error" title={title}>
      <span className="font-mono text-xs">{error.message}</span>
    </Alert>
  );
}
