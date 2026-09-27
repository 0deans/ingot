// Builds Ingot's companion (the live map feed) for both server families; the app embeds
// both jars:
//   src-tauri/companion/ingot-companion-paper.jar   Paper, Purpur, Folia 1.20.1+
//   src-tauri/companion/ingot-companion-fabric.jar  Fabric on Minecraft 26.1+
//
// Usage: node scripts/build-companion.mjs   (needs JDK 25+: JAVA_HOME or javac on PATH)
//
// Both share companion/common (the live chunk file format). The Paper plugin compiles
// against the Paper 1.20.1 API for Java 17. The Fabric mod compiles against Minecraft
// 26.1's own server jar: from 26.1 on Minecraft isn't obfuscated, so the mod uses
// Mojang's names directly and needs no remapping (hence no Gradle/Loom).
import { execFileSync } from "node:child_process"
import fs from "node:fs"
import path from "node:path"

const VERSION = "1.2.0"
/** Oldest Minecraft the Fabric mod supports; compiling against it proves compatibility */
const FABRIC_MINECRAFT = "26.1"

const root = path.resolve(import.meta.dirname, "..")
const companion = path.join(root, "src-tauri", "companion")
const work = path.join(root, "src-tauri", "target", "companion")
const deps = path.join(work, "deps")

const PAPER = "https://repo.papermc.io/repository/maven-public"
const CENTRAL = "https://repo1.maven.org/maven2"
const FABRIC = "https://maven.fabricmc.net"

function jdkTool(name) {
	const exe = process.platform === "win32" ? `${name}.exe` : name
	const home = process.env.JAVA_HOME
	return home && fs.existsSync(path.join(home, "bin", exe)) ? path.join(home, "bin", exe) : name
}

function javacMajor() {
	const out = execFileSync(jdkTool("javac"), ["-version"], { encoding: "utf8", stdio: "pipe" })
	return Number.parseInt(out.trim().split(/\s+/)[1], 10)
}

async function download(url, name = path.basename(new URL(url).pathname)) {
	const dest = path.join(deps, name)
	if (fs.existsSync(dest)) return dest
	const res = await fetch(url, { headers: { "User-Agent": "Ingot-Build-Script" } })
	if (!res.ok) throw new Error(`Download failed (${res.status}): ${url}`)
	fs.writeFileSync(dest, Buffer.from(await res.arrayBuffer()))
	return dest
}

async function fetchJson(url) {
	const res = await fetch(url)
	if (!res.ok) throw new Error(`HTTP ${res.status}: ${url}`)
	return res.json()
}

function sources(...dirs) {
	const out = []
	const walk = (dir) => {
		for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
			const full = path.join(dir, entry.name)
			if (entry.isDirectory()) walk(full)
			else if (entry.name.endsWith(".java")) out.push(full)
		}
	}
	for (const dir of dirs) walk(dir)
	return out
}

/** Compiles sources into a fresh folder, adds resources (with ${version} filled in), jars it */
function buildJar({ name, release, classpath, srcDirs, resources, output }) {
	const classes = path.join(work, name)
	fs.rmSync(classes, { recursive: true, force: true })
	fs.mkdirSync(classes, { recursive: true })
	execFileSync(
		jdkTool("javac"),
		[
			"--release",
			String(release),
			"-Xlint:-deprecation",
			"-Xlint:-options",
			"-classpath",
			classpath.join(path.delimiter),
			"-d",
			classes,
			...sources(...srcDirs),
		],
		{ stdio: "inherit" },
	)
	for (const file of resources) {
		const text = fs
			.readFileSync(file, "utf8")
			// biome-ignore lint/suspicious/noTemplateCurlyInString: a literal placeholder in the resource files
			.replace("${version}", VERSION)
		fs.writeFileSync(path.join(classes, path.basename(file)), text)
	}
	fs.rmSync(output, { force: true })
	execFileSync(jdkTool("jar"), ["--create", "--file", output, "-C", classes, "."], {
		stdio: "inherit",
	})
	console.log(
		`Built ${path.relative(root, output)} (${fs.statSync(output).size} bytes, v${VERSION})`,
	)
}

/** Minecraft's server jar and its libraries, unpacked from Mojang's bundler jar */
async function minecraftServer(version) {
	const dir = path.join(deps, `minecraft-${version}`)
	const marker = path.join(dir, ".done")
	if (!fs.existsSync(marker)) {
		const manifest = await fetchJson(
			"https://piston-meta.mojang.com/mc/game/version_manifest_v2.json",
		)
		const entry = manifest.versions.find((v) => v.id === version)
		if (!entry) throw new Error(`Minecraft ${version} not found in Mojang's version manifest`)
		const meta = await fetchJson(entry.url)
		const bundler = await download(meta.downloads.server.url, `minecraft-server-${version}.jar`)
		fs.rmSync(dir, { recursive: true, force: true })
		fs.mkdirSync(dir, { recursive: true })
		execFileSync(
			jdkTool("jar"),
			["--extract", "--file", bundler, "META-INF/versions", "META-INF/libraries"],
			{
				cwd: dir,
				stdio: "inherit",
			},
		)
		fs.writeFileSync(marker, "")
	}
	const jars = []
	const walk = (d) => {
		for (const entry of fs.readdirSync(d, { withFileTypes: true })) {
			const full = path.join(d, entry.name)
			if (entry.isDirectory()) walk(full)
			else if (entry.name.endsWith(".jar")) jars.push(full)
		}
	}
	walk(path.join(dir, "META-INF"))
	return jars
}

if (javacMajor() < 25) {
	throw new Error(
		"JDK 25 or newer is needed (Minecraft 26 is built for Java 25). Point JAVA_HOME at one, e.g. the Java 25 Ingot downloads into its runtimes folder.",
	)
}
fs.mkdirSync(deps, { recursive: true })
const common = path.join(companion, "common", "src")

// ─── Paper, Purpur, Folia ─────────────────────────────────────────────────────

buildJar({
	name: "paper",
	release: 17,
	classpath: [
		await download(
			`${PAPER}/io/papermc/paper/paper-api/1.20.1-R0.1-SNAPSHOT/paper-api-1.20.1-R0.1-20230921.165944-178.jar`,
		),
		await download(`${CENTRAL}/net/kyori/adventure-api/4.14.0/adventure-api-4.14.0.jar`),
		await download(`${CENTRAL}/net/kyori/adventure-key/4.14.0/adventure-key-4.14.0.jar`),
		await download(`${CENTRAL}/net/kyori/examination-api/1.3.0/examination-api-1.3.0.jar`),
		await download(`${CENTRAL}/org/jetbrains/annotations/24.0.1/annotations-24.0.1.jar`),
		await download(`${CENTRAL}/com/google/guava/guava/32.1.2-jre/guava-32.1.2-jre.jar`),
	],
	srcDirs: [common, path.join(companion, "paper", "src")],
	resources: [path.join(companion, "paper", "plugin.yml")],
	output: path.join(companion, "ingot-companion-paper.jar"),
})

// ─── Fabric ───────────────────────────────────────────────────────────────────

buildJar({
	name: "fabric",
	release: 25,
	classpath: [
		...(await minecraftServer(FABRIC_MINECRAFT)),
		// Only for the @Mixin annotations; Fabric Loader ships Mixin at runtime
		await download(
			`${FABRIC}/net/fabricmc/sponge-mixin/0.17.4+mixin.0.8.7/sponge-mixin-0.17.4+mixin.0.8.7.jar`,
		),
	],
	srcDirs: [common, path.join(companion, "fabric", "src")],
	resources: [
		path.join(companion, "fabric", "fabric.mod.json"),
		path.join(companion, "fabric", "ingot.mixins.json"),
	],
	output: path.join(companion, "ingot-companion-fabric.jar"),
})
