import i18n from "i18next"
import type { TranslationKey } from "@/i18n"
import { formatPercent } from "@/lib/format"

/**
 * Progress texts come from the Rust side in English. The known ones are matched here
 * and shown in the user's language; anything else (a new message, an error with
 * details) is shown as it came.
 */
type Rule = [RegExp, TranslationKey, (m: RegExpMatchArray) => Record<string, string>]

const SERVER_STEPS: Rule[] = [
	[/^Checking server files\.\.\.$/, "backend.steps.checkingFiles", () => ({})],
	[/^Preparing Pumpkin server\.\.\.$/, "backend.steps.preparingPumpkin", () => ({})],
	[
		/^Downloading Pumpkin \(first start only\)\.\.\.$/,
		"backend.steps.downloadingPumpkin",
		() => ({}),
	],
	[/^Preparing Linux sandbox\.\.\.$/, "backend.steps.preparingSandbox", () => ({})],
	[/^Downloading Linux sandbox\.\.\.$/, "backend.steps.downloadingSandbox", () => ({})],
	[/^Extracting sandbox filesystem\.\.\.$/, "backend.steps.extractingSandbox", () => ({})],
	[
		/^Installing Java (\d+) in sandbox\.\.\.$/,
		"backend.steps.installingJava",
		(m) => ({ version: m[1] }),
	],
	[/^Sandbox initialization complete!$/, "backend.steps.sandboxReady", () => ({})],
	[
		/^Installing (.+) \(first start only, this can take a few minutes\)\.\.\.$/,
		"backend.steps.installingCore",
		(m) => ({ name: m[1] }),
	],
	[/^Launching server process\.\.\.$/, "backend.steps.launching", () => ({})],
	[/^Failed to start: ([\s\S]*)$/, "backend.steps.failed", (m) => ({ error: m[1] })],
]

/** i18n.t with a key picked at runtime (the typed overloads want literal keys) */
const translate = (key: TranslationKey, options?: Record<string, string>): string =>
	(i18n.t as unknown as (...args: unknown[]) => string).call(i18n, key, options)

function match(rules: Rule[], text: string): string | null {
	for (const [re, key, params] of rules) {
		const m = text.match(re)
		if (m) return translate(key, params(m))
	}
	return null
}

const PLAN_NOTES: Rule[] = [
	[
		/^Not on Modrinth, CurseForge or Hangar, so Ingot can't check it$/,
		"backend.notes.unknownSource",
		() => ({}),
	],
	[/^Beta version, may be unstable$/, "backend.notes.beta", () => ({})],
	[/^Alpha version, may be unstable$/, "backend.notes.alpha", () => ({})],
	[
		/^Its (.+) version can only be downloaded from (.+)$/,
		"backend.notes.siteOnly",
		(m) => ({ target: m[1], site: m[2] }),
	],
	[/^Only needed on Fabric and Quilt$/, "backend.notes.fabricOnly", () => ({})],
	[/^No version for (.+) yet$/, "backend.notes.noVersion", (m) => ({ target: m[1] })],
	[
		/^Not marked for (.+) yet; most plugins still work$/,
		"backend.notes.pluginNotMarked",
		(m) => ({ version: m[1] }),
	],
	[
		/^Not updated for (.+); it usually still works$/,
		"backend.notes.notUpdated",
		(m) => ({ version: m[1] }),
	],
	[/^Same as (.+)$/, "backend.notes.sameAs", (m) => ({ name: m[1] })],
	[/^Doesn't work together with (.+)$/, "backend.notes.incompatible", (m) => ({ name: m[1] })],
	[
		/^Needs (.+), which is being turned off$/,
		"backend.notes.needsTurnedOff",
		(m) => ({ name: m[1] }),
	],
	[
		/^Needs (.+), which isn't available for (.+)$/,
		"backend.notes.needsMissing",
		(m) => ({ name: m[1], version: m[2] }),
	],
	[/^Needed by (.+)$/, "backend.notes.neededBy", (m) => ({ name: m[1] })],
]

/** A note on a version change item; "A · B" notes are translated part by part */
export function translatePlanNote(note: string): string {
	return note
		.split(" · ")
		.map((part) => match(PLAN_NOTES, part) ?? part)
		.join(" · ")
}

const LAUNCH_PHASES: Record<string, TranslationKey> = {
	"Preparing launch": "backend.launch.preparing",
	"Resolving Version": "backend.launch.resolvingVersion",
	"Resolving Mod Loader": "backend.launch.resolvingLoader",
	"Checking Java Runtime": "backend.launch.checkingJava",
	"Java Setup": "backend.launch.javaSetup",
	"Downloading Client": "backend.launch.downloadingClient",
	"Downloading Libraries": "backend.launch.downloadingLibraries",
	"Downloading Assets": "backend.launch.downloadingAssets",
	"Verifying Files": "backend.launch.verifying",
	"Auth Setup": "backend.launch.auth",
	"Starting Game": "backend.launch.starting",
}

const LAUNCH_DETAILS: Rule[] = [
	[/^Fetching version metadata\.\.\.$/, "backend.launch.fetchingMetadata", () => ({})],
	[/^Fetching (\w+) profile\.\.\.$/, "backend.launch.fetchingProfile", (m) => ({ loader: m[1] })],
	[/^Detecting Java environment\.\.\.$/, "backend.launch.detectingJava", () => ({})],
	[
		/^Downloading Java (\d+) runtime \(([\d.]+)MB \/ ([\d.]+)MB\)\.\.\.$/,
		"backend.launch.downloadingJavaProgress",
		(m) => ({ version: m[1], done: m[2], total: m[3] }),
	],
	[
		/^Downloading Java (\d+) runtime\.\.\.$/,
		"backend.launch.downloadingJava",
		(m) => ({ version: m[1] }),
	],
	[
		/^Extracting Java (\d+) runtime\.\.\.$/,
		"backend.launch.extractingJava",
		(m) => ({ version: m[1] }),
	],
	[/^Downloading Minecraft client jar\.\.\.$/, "backend.launch.downloadingJar", () => ({})],
	[
		/^Downloading (\d+) libraries\.\.\.$/,
		"backend.launch.downloadingLibrariesCount",
		(m) => ({ count: m[1] }),
	],
	[
		/^Library (\d+)\/(\d+) \((.+)\)$/,
		"backend.launch.library",
		(m) => ({ current: m[1], total: m[2], file: m[3] }),
	],
	[
		/^Downloading (\d+) assets\.\.\.$/,
		"backend.launch.downloadingAssetsCount",
		(m) => ({ count: m[1] }),
	],
	[/^Assets (\d+)\/(\d+)$/, "backend.launch.assets", (m) => ({ current: m[1], total: m[2] })],
	[/^All game files verified$/, "backend.launch.verified", () => ({})],
	[
		/^Setting up Ely\.by skin & authentication agent\.\.\.$/,
		"backend.launch.authDetail",
		() => ({}),
	],
	[/^Launching Minecraft process\.\.\.$/, "backend.launch.launching", () => ({})],
]

/** The phase of a game launch ("Downloading Assets") */
export function translateLaunchPhase(phase: string): string {
	const key = LAUNCH_PHASES[phase]
	return key ? translate(key) : phase
}

/** The detail line of a game launch ("Library 12/80 (lwjgl.jar)") */
export function translateLaunchDetail(detail: string): string {
	return match(LAUNCH_DETAILS, detail) ?? detail
}

/** A server startup step ("Downloading Linux sandbox... (40%)") */
export function translateStep(step: string): string {
	const pct = step.match(/^(.*) \((\d+)%\)$/)
	const text = pct ? pct[1] : step
	const translated = match(SERVER_STEPS, text) ?? text
	return pct ? `${translated} (${formatPercent(Number(pct[2]))})` : translated
}
