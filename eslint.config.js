// SPDX-License-Identifier: GPL-3.0-or-later
import js from "@eslint/js";
import tseslint from "typescript-eslint";
import reactHooks from "eslint-plugin-react-hooks";

export default tseslint.config(
  { ignores: ["dist", "src-tauri", "node_modules", "docs"] },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  reactHooks.configs.flat.recommended,
  {
    // Node-Skripte (Build-Helfer), kein Browser-Kontext.
    files: ["scripts/**/*.mjs"],
    languageOptions: { globals: { process: "readonly", console: "readonly", URL: "readonly" } },
  },
  {
    rules: {
      // Syncing props/context into local draft state via effects is deliberate in a few forms.
      "react-hooks/set-state-in-effect": "off",
    },
  },
);
