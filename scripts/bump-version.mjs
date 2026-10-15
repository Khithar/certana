// SPDX-License-Identifier: GPL-3.0-or-later
// Zählt die Projektversion hoch und schreibt sie in alle drei Quellen:
// package.json, src-tauri/tauri.conf.json und src-tauri/Cargo.toml ([workspace.package]).
// Danach wird Cargo.lock für die Workspace-Crates nachgezogen.
// Aufruf: node scripts/bump-version.mjs [patch|minor|major]   (Default: patch)
import { readFileSync, writeFileSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { resolve } from "node:path";

const root = resolve(new URL("..", import.meta.url).pathname);
const part = process.argv[2] ?? "patch";
if (!["patch", "minor", "major"].includes(part)) {
  console.error(`Unbekannter Versionsteil «${part}», erlaubt: patch | minor | major`);
  process.exit(1);
}

const pkgPath = resolve(root, "package.json");
const confPath = resolve(root, "src-tauri/tauri.conf.json");
const cargoPath = resolve(root, "src-tauri/Cargo.toml");

const pkg = JSON.parse(readFileSync(pkgPath, "utf8"));
const current = pkg.version;
const m = /^(\d+)\.(\d+)\.(\d+)$/.exec(current);
if (!m) {
  console.error(`Version «${current}» in package.json ist nicht im Format MAJOR.MINOR.PATCH`);
  process.exit(1);
}
let [major, minor, patch] = m.slice(1).map(Number);
if (part === "major") { major++; minor = 0; patch = 0; }
else if (part === "minor") { minor++; patch = 0; }
else patch++;
const next = `${major}.${minor}.${patch}`;

// package.json: nur das Versionsfeld ersetzen, Formatierung beibehalten.
const replaceOnce = (text, re, replacement, what) => {
  if (!re.test(text)) {
    console.error(`Versionsfeld in ${what} nicht gefunden`);
    process.exit(1);
  }
  return text.replace(re, replacement);
};
writeFileSync(pkgPath, replaceOnce(readFileSync(pkgPath, "utf8"), /^(\s*"version":\s*")[^"]+(")/m, `$1${next}$2`, "package.json"));
writeFileSync(confPath, replaceOnce(readFileSync(confPath, "utf8"), /^(\s*"version":\s*")[^"]+(")/m, `$1${next}$2`, "tauri.conf.json"));
writeFileSync(cargoPath, replaceOnce(readFileSync(cargoPath, "utf8"), /^(\[workspace\.package\][\s\S]*?^version\s*=\s*")[^"]+(")/m, `$1${next}$2`, "Cargo.toml"));

// Cargo.lock für die Workspace-Mitglieder nachziehen (keine Abhängigkeiten anfassen).
execFileSync("cargo", ["update", "--workspace", "--offline", "--manifest-path", cargoPath], { stdio: "inherit" });

console.log(`Version ${current} → ${next}`);
