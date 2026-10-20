import { test } from "node:test";
import assert from "node:assert/strict";
import { build } from "vite";
test("production includes companion and excludes studio modules and remote assets", async () => {
  const ids: string[] = [];
  const resources: string[] = [];
  const chunks=new Map<string,{ids:string[];imports:string[]}>();
  const htmlFiles:string[]=[];
  const built = await build({
    mode:"companion",
    logLevel: "silent",
    build: { write: false },
    plugins: [
      {
        name: "inspect-companion-output",
        generateBundle(_options, bundle) {
          for (const output of Object.values(bundle)) {
            if (output.type === "chunk") {
              chunks.set(output.fileName,{ids:output.moduleIds.map(id=>id.replaceAll('\\','/')),imports:[...output.imports,...output.dynamicImports]});
              ids.push(
                ...output.moduleIds.map((id) => id.replaceAll("\\", "/")),
              );
            } else if (/\.(html|css)$/.test(output.fileName)) {
              resources.push(String(output.source));
              if(output.fileName.endsWith('.html'))htmlFiles.push(output.fileName);
            }
          }
        },
      },
    ],
  });
  // Vite emits HTML after user generateBundle hooks. Inspect the completed result.
  for(const result of Array.isArray(built)?built:[built]) {
    if(!('output' in result))continue;
    for(const output of result.output)if(output.type==='asset'&&output.fileName.endsWith('.html'))htmlFiles.push(output.fileName);
  }
  assert.ok(
    ids.some((id) => id.endsWith("/companion/DesktopCompanion.tsx")),
    "production companion must be bundled",
  );
  assert.ok(htmlFiles.includes('companion.html')&&!htmlFiles.includes('index.html'),'companion build is a separate entry');
  const companionChunk=[...chunks.entries()].find(([,chunk])=>chunk.ids.some(id=>id.endsWith('/app/companion.tsx')))?.[0];
  assert.ok(companionChunk);
  const visited=new Set<string>();const graph:string[]=[];
  function walk(name:string){if(visited.has(name))return;visited.add(name);const chunk=chunks.get(name);if(!chunk)return;graph.push(...chunk.ids);chunk.imports.forEach(walk)}
  walk(companionChunk);
  assert.ok(!graph.some(id=>id.endsWith('/platform/auth.ts')||id.endsWith('/app/main.tsx')||id.includes('/companion/studio/')),'companion must not load account or studio code');
  assert.ok(!graph.some(id=>id.endsWith('/platform/aylul.ts')||id.endsWith('/data/aylul-state.ts')||id.includes('/features/approvals/')||id.includes('/features/sessions/')),'companion must not load local agent state or approval controls');
  assert.ok(!graph.some(id=>id.endsWith('/platform/assistant.ts')||id.includes('/features/assistant/')),'companion must not load assistant operations');
  assert.ok(!graph.some(id=>/\/app\/(SettingsPanel|WorkspaceShell|AccountPanel)\.tsx$/.test(id)||id.includes('/features/assistant/BotList')||id.includes('/features/assistant/AssistantDetails')),'companion must not load workspace, account or bot administration');
  assert.ok(
    !ids.some((id) => id.includes("/companion/studio/")),
    "development studio must not be bundled",
  );
  assert.ok(
    !ids.some((id) => id.includes("/tests/")),
    "test fixture must not be bundled",
  );
  assert.ok(
    resources.every(
      (value) =>
        !value.includes("cdn.tailwindcss.com") &&
        !value.includes("fonts.googleapis.com"),
    ),
  );
});
