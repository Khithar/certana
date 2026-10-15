// SPDX-License-Identifier: GPL-3.0-or-later
import { invoke } from "@tauri-apps/api/core";
import type * as T from "./types";

export * from "./types";

const call = <R>(cmd: string, args?: Record<string, unknown>) => invoke<R>(cmd, args);

export const api = {
  appInfo: () => call<T.AppInfo>("app_info"),

  dbDefaultPath: () => call<string>("db_default_path"),
  dbOpen: (path: string | null, forceShared = false) => call<T.OpenInfo>("db_open", { path, forceShared }),
  dbInfo: () => call<T.OpenInfo | null>("db_info"),
  dbClose: () => call<void>("db_close"),
  dbBackup: (dest: string) => call<string>("db_backup", { dest }),

  settingsAll: () => call<Record<string, string>>("settings_all"),
  settingsSet: (key: string, value: string) => call<void>("settings_set", { key, value }),

  caList: () => call<T.CaRecord[]>("ca_list"),
  caInfo: (caId: number) => call<T.CaDetail>("ca_info", { caId }),
  caCreateRoot: (req: T.RootCreateRequest) => call<T.RootCreateResponse>("ca_create_root", { req }),
  caImportRoot: (req: T.RootImportRequest) => call<T.RootCreateResponse>("ca_import_root", { req }),
  caOpenRootBundle: (bundlePath: string, passphrase: string) => call<T.BundleCheck>("ca_open_root_bundle", { bundlePath, passphrase }),
  caCreateIntermediate: (req: T.IntermediateCreateRequest) => call<T.CaRecord>("ca_create_intermediate", { req }),
  caCreateIntermediateCsr: (req: T.IntermediateCsrRequest) => call<{ csr_path: string; csr_pem: string }>("ca_create_intermediate_csr", { req }),
  caImportIntermediateCert: (req: T.IntermediateImportRequest) => call<T.CaRecord>("ca_import_intermediate_cert", { req }),
  caUnlock: (caId: number, passphrase: string) => call<T.SessionStatus>("ca_unlock", { caId, passphrase }),
  caLock: () => call<T.SessionStatus>("ca_lock"),
  caSession: () => call<T.SessionStatus>("ca_session"),
  caExportCert: (caId: number, dest: string, format: "pem" | "der") => call<string>("ca_export_cert", { caId, dest, format }),
  caUpdateUrls: (caId: number, crlUrl: string | null, aiaUrl: string | null, crlDays: number) => call<T.CaRecord>("ca_update_urls", { caId, crlUrl, aiaUrl, crlDays }),

  crlPublish: (caId: number, exportDir: string | null) => call<T.CrlPublished>("crl_publish", { caId, exportDir }),
  crlPublishRoot: (caId: number, bundlePath: string, passphrase: string, exportDir: string | null) => call<T.CrlPublished>("crl_publish_root", { caId, bundlePath, passphrase, exportDir }),
  crlList: (caId: number, limit = 20) => call<T.CrlRecord[]>("crl_list", { caId, limit }),
  crlExport: (caId: number, dest: string) => call<string>("crl_export", { caId, dest }),

  certProfiles: () => call<T.ProfileInfo[]>("cert_profiles"),
  certList: (filter?: T.CertificateFilter) => call<T.CertificateRecord[]>("cert_list", { filter: filter ?? null }),
  certGet: (id: number) => call<T.CertDetail>("cert_get", { id }),
  certParseCsr: (csrPem: string) => call<T.CsrInfo>("cert_parse_csr", { csrPem }),
  certIssue: (req: T.IssueRequest) => call<T.CertificateRecord>("cert_issue", { req }),
  certExport: (req: T.ExportRequest) => call<T.ExportResult>("cert_export", { req }),
  certRevoke: (id: number, reason: T.Reason, comment: string | null) => call<T.RevocationRecord>("cert_revoke", { id, reason, comment }),
  certRenew: (req: T.RenewRequest) => call<T.CertificateRecord>("cert_renew", { req }),
  certUpdateNotes: (id: number, notes: string) => call<void>("cert_update_notes", { id, notes }),

  dashboardSummary: () => call<T.DashboardSummary>("dashboard_summary"),
  auditRecent: (limit = 50) => call<T.AuditRecord[]>("audit_recent", { limit }),
  auditExportCsv: (dest: string) => call<string>("audit_export_csv", { dest }),
};

/** Normalise anything thrown by invoke() into a CommandError. */
export function asCommandError(e: unknown): T.CommandError {
  if (e && typeof e === "object" && "code" in e && "message" in e) return e as T.CommandError;
  return { code: "unknown", message: String(e) };
}
