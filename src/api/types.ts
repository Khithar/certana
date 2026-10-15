// SPDX-License-Identifier: GPL-3.0-or-later
// Mirrors the Rust DTOs in src-tauri (certana-core::db::models, pki::*, commands::*).

export type KeyAlgorithm = "ecdsa-p256" | "ecdsa-p384" | "ed25519" | "rsa2048" | "rsa3072" | "rsa4096";
export type Profile = "server" | "client" | "smime" | "code-signing";
export type CaKind = "root" | "intermediate";
export type CertStatus = "valid" | "revoked" | "expired";
export type Reason =
  | "unspecified"
  | "keyCompromise"
  | "caCompromise"
  | "affiliationChanged"
  | "superseded"
  | "cessationOfOperation"
  | "privilegeWithdrawn";

export type San =
  | { kind: "dns"; value: string }
  | { kind: "ip"; value: string }
  | { kind: "email"; value: string }
  | { kind: "uri"; value: string };

export interface Subject {
  common_name: string;
  organization: string;
  organizational_unit: string;
  country: string;
  state: string;
  locality: string;
}

export const emptySubject = (): Subject => ({ common_name: "", organization: "", organizational_unit: "", country: "", state: "", locality: "" });

export interface AppInfo { name: string; version: string; license: string; repository: string }

export interface OpenInfo {
  path: string;
  mode: "local" | "shared";
  read_only: boolean;
  held_by: { host: string; user: string; pid: number; acquired_at: string } | null;
  schema_version: number;
}

export interface CaRecord {
  id: number;
  kind: CaKind;
  name: string;
  cert_pem: string;
  cert_sha256: string;
  subject: string;
  not_before: string;
  not_after: string;
  key_algo: KeyAlgorithm;
  has_key: boolean;
  parent_ca_id: number | null;
  crl_url: string | null;
  aia_url: string | null;
  crl_number: number;
  crl_days: number;
  created_at: string;
}

export interface CertificateRecord {
  id: number;
  ca_id: number;
  serial_hex: string;
  profile: Profile;
  subject: string;
  sans: San[];
  not_before: string;
  not_after: string;
  cert_pem: string;
  has_key: boolean;
  csr_pem: string | null;
  status: CertStatus;
  notes: string;
  created_at: string;
  created_by: string;
}

export interface RevocationRecord {
  cert_id: number;
  revoked_at: string;
  reason: Reason;
  invalidity_date: string | null;
  comment: string;
  first_crl_id: number | null;
}

export interface CrlRecord {
  id: number;
  ca_id: number;
  crl_number: number;
  this_update: string;
  next_update: string;
  exported_path: string | null;
}

export interface AuditRecord {
  id: number;
  ts: string;
  actor: string;
  host: string;
  action: string;
  target_type: string;
  target_id: string;
  details: unknown;
}

export interface CertificateInfo {
  subject: string;
  subject_common_name: string | null;
  issuer: string;
  issuer_common_name: string | null;
  serial_hex: string;
  not_before: string;
  not_after: string;
  is_ca: boolean;
  path_len: number | null;
  self_signed: boolean;
  key_usages: string[];
  extended_key_usages: string[];
  sans: string[];
  crl_urls: string[];
  ca_issuers_urls: string[];
  ocsp_urls: string[];
  signature_algorithm: string;
  public_key_algorithm: string;
  sha256_fingerprint: string;
  sha1_fingerprint: string;
  text: string;
}

export interface CsrInfo { subject: Subject; sans: San[]; key_algorithm: string }

export interface CertificateFilter {
  ca_id?: number | null;
  profile?: Profile | null;
  status?: CertStatus | null;
  search?: string | null;
  expiring_within_days?: number | null;
}

export interface ProfileInfo {
  profile: Profile;
  default_validity_days: number;
  max_validity_days: number;
  default_key_algorithm: KeyAlgorithm;
  key_algorithms: KeyAlgorithm[];
}

export interface SessionStatus { unlocked_ca_id: number | null; idle_seconds_left: number | null }

export interface CaDetail { ca: CaRecord; info: CertificateInfo; chain_pem: string }
export interface CertDetail { record: CertificateRecord; info: CertificateInfo; revocation: RevocationRecord | null; ca_name: string }
export interface BundleCheck { name: string; fingerprint_sha256: string; subject: string; matches_ca_id: number | null }

export interface RootCreateRequest {
  name: string;
  subject: Subject;
  algorithm: KeyAlgorithm;
  validity_days: number;
  crl_url: string | null;
  aia_url: string | null;
  passphrase: string;
  bundle_path: string;
}
export interface RootCreateResponse { ca: CaRecord; bundle_path: string; fingerprint_sha256: string }

export interface RootImportRequest {
  name: string;
  cert_path: string;
  key_path: string | null;
  key_passphrase: string | null;
  p12_path: string | null;
  p12_password: string | null;
  bundle_path: string | null;
  bundle_passphrase: string | null;
  crl_url: string | null;
  aia_url: string | null;
}

export interface ChangePassphraseRequest { ca_id: number; old_passphrase: string; new_passphrase: string }
export interface ChangePassphraseResponse { rewrapped_leaf_keys: number }

export interface IntermediateCreateRequest {
  name: string;
  subject: Subject;
  algorithm: KeyAlgorithm;
  validity_days: number;
  crl_url: string | null;
  aia_url: string | null;
  crl_days: number;
  passphrase: string;
  root_ca_id: number;
  root_bundle_path: string;
  root_passphrase: string;
}

export interface IntermediateCsrRequest { name: string; subject: Subject; algorithm: KeyAlgorithm; passphrase: string; csr_path: string }
export interface IntermediateImportRequest { name: string; cert_path: string; root_ca_id: number; crl_url: string | null; aia_url: string | null; crl_days: number }

export type KeySource = { kind: "generate"; algorithm: KeyAlgorithm } | { kind: "csr"; csr_pem: string };

export interface IssueRequest {
  ca_id: number;
  profile: Profile;
  subject: Subject;
  sans: San[];
  validity_days: number;
  key: KeySource;
  notes: string | null;
}

export interface ExportRequest {
  cert_id: number;
  dest_dir: string;
  base_name: string | null;
  include_key: boolean;
  include_der: boolean;
  include_p12: boolean;
  p12_password: string | null;
  p12_legacy: boolean;
}
export interface ExportResult { files: string[]; snippets: { nginx: string; apache: string }; key_written: boolean }

export interface RenewRequest { cert_id: number; validity_days: number; new_key_algorithm: KeyAlgorithm | null; revoke_old: boolean }

export interface CrlPublished { crl: CrlRecord; der_path: string | null; pem_path: string | null; revoked_count: number }

export interface CaStatus { ca: CaRecord; latest_crl: CrlRecord | null; crl_expires_soon: boolean; unlocked: boolean }
export interface DashboardSummary {
  cas: CaStatus[];
  valid: number;
  revoked: number;
  expired: number;
  expiring_30: CertificateRecord[];
  expiring_14: number;
  expiring_7: number;
  recent_audit: AuditRecord[];
}

export interface CommandError { code: string; message: string }

export const SETTING_KEYS = {
  organization: "subject.organization",
  organizationalUnit: "subject.organizational_unit",
  country: "subject.country",
  state: "subject.state",
  locality: "subject.locality",
  exportDir: "export.dir",
  crlExportDir: "crl.export_dir",
  autoLockMinutes: "security.auto_lock_minutes",
  expiryWarnDays: "notify.expiry_warn_days",
  language: "ui.language",
  lastDbPath: "ui.last_db_path",
} as const;
