// Salin hasil `tauri build` ke folder release/ di root proyek.
import { cpSync, existsSync, mkdirSync, readdirSync } from "node:fs";
import { join } from "node:path";

const target = join("src-tauri", "target", "release");
const out = "release";
mkdirSync(out, { recursive: true });

cpSync(join(target, "cliptray.exe"), join(out, "ClipTray.exe"));
console.log(`  ${join(out, "ClipTray.exe")}`);

for (const kind of ["nsis", "msi"]) {
  const dir = join(target, "bundle", kind);
  if (!existsSync(dir)) continue;
  for (const file of readdirSync(dir).filter((f) => /\.(exe|msi)$/.test(f))) {
    cpSync(join(dir, file), join(out, file));
    console.log(`  ${join(out, file)}`);
  }
}
