import { spawnSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");

// This list mirrors the commands reported by the repository-pinned Tauri v2
// CLI. Keeping root parsing closed prevents an option from hiding build/bundle.
const COMMANDS = new Set([
  "init", "dev", "build", "bundle", "android", "migrate", "info", "add",
  "remove", "plugin", "icon", "signer", "completions", "permission",
  "capability", "inspect", "help",
]);

function isVerbosityOption(argument) {
  return argument === "--verbose" || /^-v+$/.test(argument);
}

export function parseTauriInvocation(args) {
  if (args.length === 0) return { command: null, commandIndex: -1 };

  let index = 0;
  while (index < args.length && isVerbosityOption(args[index])) index += 1;

  if (["-h", "--help", "-V", "--version"].includes(args[index])) {
    if (index !== args.length - 1) {
      throw new Error("Tauri help/version options must be standalone.");
    }
    return { command: null, commandIndex: -1 };
  }

  const command = args[index];
  if (!command || command === "--" || command.startsWith("-") || !COMMANDS.has(command)) {
    throw new Error(
      "Unsupported or ambiguous Tauri root arguments. Only -v/--verbose may precede a known command.",
    );
  }
  return { command, commandIndex: index };
}

function isProtectedBundleValue(value) {
  return value.toLowerCase().split(",").some((part) => ["appimage", "all"].includes(part));
}

export function validateTauriInvocation(args, environment = process.env) {
  const parsed = parseTauriInvocation(args);
  if (!["build", "bundle"].includes(parsed.command)) return parsed;

  if (environment.TAURI_CONFIG) {
    throw new Error("AppImage/config-overlay builds must use npm run release:linux:signed.");
  }

  const commandArguments = args.slice(parsed.commandIndex + 1);
  if (commandArguments.includes("--")) {
    throw new Error("Argument separators are unsupported for ordinary Tauri build/bundle commands.");
  }

  for (let index = 0; index < commandArguments.length; index += 1) {
    const argument = commandArguments[index];
    const lower = argument.toLowerCase();

    if (COMMANDS.has(argument)) {
      throw new Error("A second Tauri command after build/bundle is ambiguous and unsupported.");
    }
    if (isProtectedBundleValue(argument)
      || lower.split(",").includes("signed-updater")) {
      throw new Error("AppImage/all-bundle/updater-feature builds must use npm run release:linux:signed.");
    }

    if (lower === "--config" || lower === "-c" || lower.startsWith("--config=")
      || (/^-c.+/i.test(argument) && argument !== "--ci")) {
      throw new Error("AppImage/config-overlay builds must use npm run release:linux:signed.");
    }

    if (lower === "--bundles" || lower === "-b") {
      const value = commandArguments[index + 1];
      if (!value || value.startsWith("-") || isProtectedBundleValue(value)) {
        throw new Error("AppImage/all-bundle builds must use npm run release:linux:signed.");
      }
      index += 1;
      continue;
    }
    if (lower.startsWith("--bundles=")) {
      if (isProtectedBundleValue(argument.slice(argument.indexOf("=") + 1))) {
        throw new Error("AppImage/all-bundle builds must use npm run release:linux:signed.");
      }
      continue;
    }
    if (/^-b.+/i.test(argument)) {
      if (isProtectedBundleValue(argument.slice(2).replace(/^=/, ""))) {
        throw new Error("AppImage/all-bundle builds must use npm run release:linux:signed.");
      }
      continue;
    }

    if (lower === "--features" || lower === "-f") {
      const value = commandArguments[index + 1];
      if (!value || value.startsWith("-") || value.toLowerCase().split(",").includes("signed-updater")) {
        throw new Error("The signed-updater feature requires npm run release:linux:signed.");
      }
      index += 1;
      continue;
    }
    if (lower.startsWith("--features=")
      && argument.slice(argument.indexOf("=") + 1).toLowerCase().split(",").includes("signed-updater")) {
      throw new Error("The signed-updater feature requires npm run release:linux:signed.");
    }
    if (/^-f.+/i.test(argument)
      && argument.slice(2).replace(/^=/, "").toLowerCase().split(",").includes("signed-updater")) {
      throw new Error("The signed-updater feature requires npm run release:linux:signed.");
    }
  }

  return parsed;
}

export function runTauri(args = process.argv.slice(2)) {
  validateTauriInvocation(args);
  const result = spawnSync(process.execPath, [resolve(root, "node_modules/@tauri-apps/cli/tauri.js"), ...args], {
    cwd: root, stdio: "inherit", shell: false,
  });
  if (result.error) throw result.error;
  return result.status ?? 1;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    process.exit(runTauri());
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  }
}
