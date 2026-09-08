import { spawnSync } from "node:child_process";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
export function verifyUpdate(image, signature, version) {
  const run = (program, args) => {
    const result = spawnSync(program, args, { cwd: root, stdio: "inherit", shell: false });
    if (result.error || result.status !== 0) throw new Error("AppImage safety/signature verification failed; publication refused.");
  };
  run("python3", ["scripts/audit-release-artifacts.py", "--workspace", root,
    "--version", version, "--artifact", resolve(signature)]);
  run("cargo", ["run", "--locked", "--quiet", "--manifest-path", "scripts/update-verifier/Cargo.toml", "--",
    resolve(image), resolve(signature), resolve(root, "src-tauri/tauri.conf.json"), version]);
}
if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  if (process.argv.length !== 5) throw new Error("Usage: node scripts/verify-update.mjs APPIMAGE SIGNATURE VERSION");
  verifyUpdate(...process.argv.slice(2));
}
