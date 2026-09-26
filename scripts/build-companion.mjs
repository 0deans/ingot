// Builds the Ingot companion plugin (Paper/Purpur/Folia) into
// src-tauri/companion/ingot-companion.jar, which the app embeds.
//
// Usage: node scripts/build-companion.mjs   (needs a JDK 17+: JAVA_HOME or javac on PATH)
//
// Compiles against the Paper 1.20.1 API for Java 17, so one jar runs on every server
// version the plugin supports (1.20.1 and newer).
import { execFileSync } from "node:child_process"
import fs from "node:fs"
import path from "node:path"

const VERSION = "1.0.0"

const root = path.resolve(import.meta.dirname, "..")
const source = path.join(root, "src-tauri", "companion", "paper")
const output = path.join(root, "src-tauri", "companion", "ingot-companion.jar")
const work = path.join(root, "src-tauri", "target", "companion")
const deps = path.join(work, "deps")

const PAPER = "https://repo.papermc.io/repository/maven-public"
const CENTRAL = "https://repo1.maven.org/maven2"
// Only what javac needs to resolve the types the plugin touches
const DEPENDENCIES = [
	[PAPER, "io/papermc/paper/paper-api/1.20.1-R0.1-SNAPSHOT/paper-api-1.20.1-R0.1-20230921.165944-178.jar"],
	[CENTRAL, "net/kyori/adventure-api/4.14.0/adventure-api-4.14.0.jar"],
	[CENTRAL, "net/kyori/adventure-key/4.14.0/adventure-key-4.14.0.jar"],
	[CENTRAL, "net/kyori/examination-api/1.3.0/examination-api-1.3.0.jar"],
	[CENTRAL, "org/jetbrains/annotations/24.0.1/annotations-24.0.1.jar"],
	[CENTRAL, "com/google/guava/guava/32.1.2-jre/guava-32.1.2-jre.jar"],
]

function jdkTool(name) {
	const exe = process.platform === "win32" ? `${name}.exe` : name
	const home = process.env.JAVA_HOME
	return home && fs.existsSync(path.join(home, "bin", exe)) ? path.join(home, "bin", exe) : name
}

async function download(base, file) {
	const dest = path.join(deps, path.basename(file))
	if (fs.existsSync(dest)) return dest
	const res = await fetch(`${base}/${file}`)
	if (!res.ok) throw new Error(`Download failed (${res.status}): ${file}`)
	fs.writeFileSync(dest, Buffer.from(await res.arrayBuffer()))
	return dest
}

fs.rmSync(path.join(work, "classes"), { recursive: true, force: true })
fs.mkdirSync(deps, { recursive: true })
fs.mkdirSync(path.join(work, "classes"), { recursive: true })

const classpath = []
for (const [base, file] of DEPENDENCIES) classpath.push(await download(base, file))

const sources = []
const walk = (dir) => {
	for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
		const full = path.join(dir, entry.name)
		if (entry.isDirectory()) walk(full)
		else if (entry.name.endsWith(".java")) sources.push(full)
	}
}
walk(path.join(source, "src"))

execFileSync(
	jdkTool("javac"),
	[
		"--release",
		"17",
		"-Xlint:-deprecation",
		"-classpath",
		classpath.join(path.delimiter),
		"-d",
		path.join(work, "classes"),
		...sources,
	],
	{ stdio: "inherit" },
)

const pluginYml = fs.readFileSync(path.join(source, "plugin.yml"), "utf8").replace("${version}", VERSION)
fs.writeFileSync(path.join(work, "classes", "plugin.yml"), pluginYml)

fs.rmSync(output, { force: true })
execFileSync(jdkTool("jar"), ["--create", "--file", output, "-C", path.join(work, "classes"), "."], {
	stdio: "inherit",
})
console.log(`Built ${path.relative(root, output)} (${fs.statSync(output).size} bytes, v${VERSION})`)
