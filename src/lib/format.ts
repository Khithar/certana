// SPDX-License-Identifier: GPL-3.0-or-later
import type { San } from "@/api/types";

export const fmtDate = (iso: string, locale?: string) =>
  new Date(iso).toLocaleDateString(locale, { year: "numeric", month: "2-digit", day: "2-digit" });

export const fmtDateTime = (iso: string, locale?: string) =>
  new Date(iso).toLocaleString(locale, { year: "numeric", month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" });

export const daysUntil = (iso: string) => Math.ceil((new Date(iso).getTime() - Date.now()) / 86_400_000);

export const colonHex = (hex: string) => hex.toUpperCase().match(/.{1,2}/g)?.join(":") ?? hex;

export const sanLabel = (s: San) => `${{ dns: "DNS", ip: "IP", email: "email", uri: "URI" }[s.kind]}:${s.value}`;

/** Parse user input like `www.example.com, 10.0.0.1, alice@example.com` into SANs. */
export function parseSans(input: string): San[] {
  return input
    .split(/[\s,;]+/)
    .map((s) => s.trim())
    .filter(Boolean)
    .map((v): San => {
      if (/^\d{1,3}(\.\d{1,3}){3}$/.test(v) || /^[0-9a-f:]+:[0-9a-f:]*$/i.test(v)) return { kind: "ip", value: v };
      if (v.includes("@")) return { kind: "email", value: v };
      if (/^[a-z][a-z0-9+.-]*:\/\//i.test(v)) return { kind: "uri", value: v };
      return { kind: "dns", value: v.toLowerCase() };
    });
}
