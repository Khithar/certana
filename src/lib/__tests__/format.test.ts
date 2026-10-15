// SPDX-License-Identifier: GPL-3.0-or-later
import { describe, expect, it } from "vitest";
import { colonHex, parseSans, sanLabel } from "@/lib/format";
import { strength, passphraseOk } from "@/components/PassphraseInput";

describe("parseSans", () => {
  it("classifies names, IPs, e-mails and URIs", () => {
    expect(parseSans("WWW.Example.internal, 10.0.0.5; alice@example.internal https://x.internal/a fe80::1")).toEqual([
      { kind: "dns", value: "www.example.internal" },
      { kind: "ip", value: "10.0.0.5" },
      { kind: "email", value: "alice@example.internal" },
      { kind: "uri", value: "https://x.internal/a" },
      { kind: "ip", value: "fe80::1" },
    ]);
  });
  it("ignores empty input", () => {
    expect(parseSans("  ,, ")).toEqual([]);
  });
  it("labels like openssl", () => {
    expect(sanLabel({ kind: "dns", value: "a" })).toBe("DNS:a");
    expect(sanLabel({ kind: "email", value: "b" })).toBe("email:b");
  });
});

describe("colonHex", () => {
  it("groups pairs", () => {
    expect(colonHex("0a1b2c")).toBe("0A:1B:2C");
  });
});

describe("passphrase strength", () => {
  it("enforces the minimum length", () => {
    expect(strength("short")).toBe(0);
    expect(passphraseOk("fifteen chars!!")).toBe(false);
    expect(passphraseOk("sixteen chars!!!")).toBe(true);
    expect(passphraseOk("sixteen chars!!!", "different........")).toBe(false);
  });
  it("rates mixed long passphrases higher", () => {
    expect(strength("correct horse battery staple")).toBeGreaterThanOrEqual(2);
    expect(strength("Tr0ub4dor&3-really-long-phrase!")).toBe(3);
  });
});
