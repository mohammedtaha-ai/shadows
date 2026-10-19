import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync, existsSync } from "node:fs";
import { resolve } from "node:path";
import YAML from "yaml";
import { createScanner, SyntaxKind } from "typescript/unstable/ast";
const root = resolve(import.meta.dirname, "../../../..");
test("companion contracts name real source exports and inspected tests", () => {
  const contracts = ["apps/desktop/src/features/companion/contract.yaml"];
  const studio = "apps/desktop/src/features/companion/studio/contract.yaml";
  if (existsSync(resolve(root, studio))) contracts.push(studio);
  for (const path of contracts) {
    const contract = YAML.parse(readFileSync(resolve(root, path), "utf8"));
    assert.ok(existsSync(resolve(root, contract.source)));
    for (const operation of contract.exports) {
      const file = resolve(root, operation.file);
      const text = readFileSync(file, "utf8");
      const declarations = [
        ...text.matchAll(/^export (?:async )?function .+/gm),
      ];
      assert.ok(
        declarations.some((match) => {
          const scanner = createScanner(true);
          scanner.setText(match[0]);
          const tokens: string[] = [];
          while (scanner.scan() !== SyntaxKind.EndOfFile)
            tokens.push(scanner.getTokenText());
          return tokens[tokens.indexOf("function") + 1] === operation.name;
        }),
        operation.name,
      );
    }
    for (const obligation of contract.obligations) {
      if (obligation.tested_by === "none") continue;
      const body = readFileSync(resolve(root, obligation.tested_by), "utf8");
      assert.ok(body.includes(obligation.test), obligation.test);
    }
  }
});
