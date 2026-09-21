import { execSync } from "node:child_process"
import fs from "node:fs"

// 1. Read the bumped version from package.json
const pkg = JSON.parse(fs.readFileSync("package.json", "utf-8"))
const version = pkg.version

console.log(`[sync-version] Synchronizing version ${version} across Tauri files...`)

// 2. Update src-tauri/tauri.conf.json (preserves exact formatting)
const tauriConfPath = "src-tauri/tauri.conf.json"
let tauriConf = fs.readFileSync(tauriConfPath, "utf-8")
tauriConf = tauriConf.replace(/"version":\s*"[^"]+"/, `"version": "${version}"`)
fs.writeFileSync(tauriConfPath, tauriConf)

// 3. Update src-tauri/Cargo.toml
const cargoTomlPath = "src-tauri/Cargo.toml"
let cargoToml = fs.readFileSync(cargoTomlPath, "utf-8")
cargoToml = cargoToml.replace(/^version\s*=\s*"[^"]+"/m, `version = "${version}"`)
fs.writeFileSync(cargoTomlPath, cargoToml)

// 4. Update src-tauri/Cargo.lock
console.log(`[sync-version] Updating Cargo.lock...`)
execSync("cargo check --manifest-path src-tauri/Cargo.toml", { stdio: "inherit" })

console.log(`[sync-version] Successfully synchronized all files to v${version}`)
