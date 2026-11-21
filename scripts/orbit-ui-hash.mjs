import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const metadata =
  process.argv[2] ?? path.join(root, "app/dist/orbit-build.json");
const { ui } = JSON.parse(await readFile(metadata, "utf8"));
if (typeof ui !== "string" || !/^[a-f\d]{16}$/i.test(ui)) {
  throw new Error(`invalid Orbit UI fingerprint in ${metadata}`);
}
process.stdout.write(`${ui}\n`);
