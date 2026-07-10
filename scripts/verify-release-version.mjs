import fs from "node:fs";

const tag = process.argv[2];
if (!tag || !/^v\d+\.\d+\.\d+$/.test(tag)) {
  throw new Error(`Invalid release tag: ${tag ?? "<missing>"}`);
}

const expected = tag.slice(1);
const packageJson = JSON.parse(fs.readFileSync("package.json", "utf8"));
const tauriConfig = JSON.parse(fs.readFileSync("src-tauri/tauri.conf.json", "utf8"));
const cargoToml = fs.readFileSync("src-tauri/Cargo.toml", "utf8");
const cargoVersion = cargoToml.match(/^version\s*=\s*"([^"]+)"/m)?.[1];

const versions = {
  "package.json": packageJson.version,
  "src-tauri/tauri.conf.json": tauriConfig.version,
  "src-tauri/Cargo.toml": cargoVersion,
};

const mismatches = Object.entries(versions).filter(([, version]) => version !== expected);
if (mismatches.length > 0) {
  const details = mismatches.map(([file, version]) => `${file}=${version ?? "<missing>"}`).join(", ");
  throw new Error(`Release tag ${tag} requires version ${expected}; found ${details}`);
}

console.log(`Release version ${expected} is consistent across npm, Cargo, and Tauri.`);
