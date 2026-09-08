import {
  copyFileSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { verifyUpdate } from "./verify-update.mjs";

const requiredEnvironment = [
  "RELEASE_VERSION",
  "RELEASE_TAG",
  "RELEASE_PUBLISHED_AT",
  "RELEASE_NOTES_FILE",
  "APPIMAGE_SIGNATURE_FILE",
  "APPIMAGE_FILE",
  "OUTPUT_DIR",
];

function sourceIdentity(path) {
  const stat = lstatSync(path, { bigint: true });
  if (!stat.isFile() || stat.isSymbolicLink()) {
    throw new Error(`Update snapshot source is not a regular non-symlink file: ${basename(path)}`);
  }
  return [stat.dev, stat.ino, stat.size, stat.mtimeNs, stat.ctimeNs].join(":");
}

export function snapshotUpdatePair(imageInput, signatureInput, version) {
  const image = resolve(imageInput);
  const signature = resolve(signatureInput);
  const imageName = `Cassette_${version}_amd64.AppImage`;
  if (basename(image) !== imageName || basename(signature) !== `${imageName}.sig`
    || dirname(image) !== dirname(signature)) {
    throw new Error("AppImage/signature snapshot inputs are not an exact adjacent versioned pair.");
  }

  const beforeImage = sourceIdentity(image);
  const beforeSignature = sourceIdentity(signature);
  const directory = mkdtempSync(join(tmpdir(), "cassette-update-feed-snapshot-"));
  const snapshotImage = join(directory, imageName);
  const snapshotSignature = join(directory, `${imageName}.sig`);
  try {
    copyFileSync(image, snapshotImage);
    copyFileSync(signature, snapshotSignature);
    if (sourceIdentity(image) !== beforeImage || sourceIdentity(signature) !== beforeSignature) {
      throw new Error("Update source changed while its private verification snapshot was created.");
    }
    return {
      image: snapshotImage,
      signature: snapshotSignature,
      dispose() { rmSync(directory, { recursive: true, force: true }); },
    };
  } catch (error) {
    rmSync(directory, { recursive: true, force: true });
    throw error;
  }
}

export function generateUpdateFeed(environment = process.env) {
  for (const name of requiredEnvironment) {
    if (!environment[name]?.trim()) {
      throw new Error(`Missing required environment variable: ${name}`);
    }
  }

  const repository = environment.GITHUB_REPOSITORY?.trim() || "atilla-m/cassette";
  if (repository !== "atilla-m/cassette") {
    throw new Error(`Refusing to generate a Cassette feed for unexpected repository: ${repository}`);
  }

  const version = environment.RELEASE_VERSION.trim();
  const tag = environment.RELEASE_TAG.trim();
  if (tag !== `v${version}` || version !== "0.1.0-beta.1") {
    throw new Error(`Release tag/version mismatch: ${tag} / ${version}`);
  }

  const publishedAt = environment.RELEASE_PUBLISHED_AT.trim();
  if (Number.isNaN(Date.parse(publishedAt))) {
    throw new Error(`Release publication time is not a valid timestamp: ${publishedAt}`);
  }

  const appImageName = `Cassette_${version}_amd64.AppImage`;
  const snapshot = snapshotUpdatePair(
    environment.APPIMAGE_FILE,
    environment.APPIMAGE_SIGNATURE_FILE,
    version,
  );
  try {
    // Both verification and feed serialization consume the same private,
    // stable snapshot. Later changes to the downloaded originals cannot enter.
    verifyUpdate(snapshot.image, snapshot.signature, version);
    const signature = readFileSync(snapshot.signature, "utf8").trim();
    if (!signature) {
      throw new Error("The verified AppImage signature snapshot is empty.");
    }

    const notes = readFileSync(resolve(environment.RELEASE_NOTES_FILE), "utf8").trim();
    if (!notes) {
      throw new Error("Published prerelease notes are empty.");
    }
    const encodedTag = encodeURIComponent(tag);
    const encodedName = encodeURIComponent(appImageName);
    const feed = {
      version,
      notes,
      pub_date: publishedAt,
      platforms: {
        "linux-x86_64": {
          signature,
          url: `https://github.com/${repository}/releases/download/${encodedTag}/${encodedName}`,
        },
      },
    };

    const outputRoot = resolve(environment.OUTPUT_DIR);
    const feedDirectory = join(outputRoot, "updates", "beta");
    mkdirSync(feedDirectory, { recursive: true });
    writeFileSync(join(outputRoot, ".nojekyll"), "", { mode: 0o644 });
    writeFileSync(join(feedDirectory, "latest.json"), `${JSON.stringify(feed, null, 2)}\n`, {
      mode: 0o644,
    });

    console.log(`Generated immutable-release beta feed for Cassette ${version}.`);
  } finally {
    snapshot.dispose();
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  generateUpdateFeed();
}
