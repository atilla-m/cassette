import { existsSync, realpathSync, readFileSync } from "node:fs";
import { homedir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const UNIT_SEPARATOR = "\u001f";
const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const workspace = realpathSync(resolve(scriptDirectory, ".."));
const packageInfo = JSON.parse(readFileSync(join(workspace, "package.json"), "utf8"));
const expectedVersion = "0.1.0-beta.1";

if (packageInfo.version !== expectedVersion) {
  throw new Error(
    `Refusing release build: package.json is ${packageInfo.version}, expected ${expectedVersion}.`,
  );
}

const forwardedArguments = process.argv.slice(2);
if (!forwardedArguments.includes("--bundles")) {
  throw new Error("A release build must specify an exact --bundles list.");
}

const environment = { ...process.env };
// Release correctness must not depend on an ambient developer/runner ccache
// directory. This still allows Cargo's own safe dependency caching in CI.
environment.CCACHE_DISABLE = "1";
if (environment.RUSTFLAGS && !environment.CARGO_ENCODED_RUSTFLAGS) {
  throw new Error(
    "RUSTFLAGS is set. Move those flags to CARGO_ENCODED_RUSTFLAGS so release path remapping cannot discard or misquote them.",
  );
}

const encodedFlags = environment.CARGO_ENCODED_RUSTFLAGS
  ? environment.CARGO_ENCODED_RUSTFLAGS.split(UNIT_SEPARATOR).filter(Boolean)
  : [];
const mappings = [];
const seenSources = new Set();

function addPathMapping(source, destination, label) {
  if (!source || !existsSync(source)) return;

  const canonicalSource = realpathSync(source);
  const comparisonKey = process.platform === "win32"
    ? canonicalSource.toLowerCase()
    : canonicalSource;
  if (seenSources.has(comparisonKey)) return;

  seenSources.add(comparisonKey);
  mappings.push({ source: canonicalSource, destination, label });
}

addPathMapping(workspace, "/workspace", "workspace");

const developerHome = homedir();
const cargoHome = environment.CARGO_HOME || join(developerHome, ".cargo");
addPathMapping(cargoHome, "/cargo-home", "Cargo home");
addPathMapping(developerHome, "/developer-home", "developer home");

if (environment.GITHUB_WORKSPACE) {
  addPathMapping(environment.GITHUB_WORKSPACE, "/workspace", "GitHub workspace");
}

for (const mapping of mappings) {
  encodedFlags.push(`--remap-path-prefix=${mapping.source}=${mapping.destination}`);
}

environment.CARGO_ENCODED_RUSTFLAGS = encodedFlags.join(UNIT_SEPARATOR);

// Rust's remapping does not reach C/C++ sources compiled by dependency build
// scripts. In particular, MSVC can otherwise retain Cargo-home paths from
// __FILE__ in the final PE. Supply the equivalent native-compiler mappings for
// release builds without relying on a shell to quote them.
function shellQuote(value) {
  return `'${value.replaceAll("'", `'\\''`)}'`;
}

function appendCompilerFlags(variable, flags) {
  const existing = environment[variable]?.trim();
  const encoded = flags.map(shellQuote).join(" ");
  environment[variable] = existing ? `${existing} ${encoded}` : encoded;
}

environment.CC_SHELL_ESCAPED_FLAGS = "1";
const compilerMappingFlags = mappings.map((mapping) => (
  process.platform === "win32"
    ? `/pathmap:${mapping.source}=${mapping.destination}`
    : `-ffile-prefix-map=${mapping.source}=${mapping.destination}`
));
appendCompilerFlags("CFLAGS", compilerMappingFlags);
appendCompilerFlags("CXXFLAGS", compilerMappingFlags);

console.log(
  `Building Cassette ${expectedVersion} with release-only stripping and ${mappings.length} portable Rust/C/C++ path remapping rule(s).`,
);
console.log(`Remapped roots: ${mappings.map((mapping) => mapping.label).join(", ")}.`);

// Invoke the repository-pinned JavaScript CLI directly. Windows cannot spawn
// npm.cmd without a shell on current Node releases, while enabling a shell
// would make path-remapping flags vulnerable to platform-specific quoting.
const tauriCli = join(workspace, "node_modules", "@tauri-apps", "cli", "tauri.js");
if (!existsSync(tauriCli)) {
  throw new Error("The Tauri CLI is missing. Run npm ci before building a release.");
}

const result = spawnSync(
  process.execPath,
  [tauriCli, "build", ...forwardedArguments],
  {
    cwd: workspace,
    env: environment,
    stdio: "inherit",
    shell: false,
  },
);

if (result.error) throw result.error;
process.exit(result.status ?? 1);
