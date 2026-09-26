const fs = require("node:fs")
const path = require("node:path")

const ROOT_DIR = path.resolve(__dirname, "..")
const ANDROID_GEN_DIR = path.join(ROOT_DIR, "src-tauri", "gen", "android")
const ANDROID_APP_DIR = path.join(ANDROID_GEN_DIR, "app", "src", "main")
const JAVA_PKG_DIR = path.join(ANDROID_APP_DIR, "java", "org", "ingot", "server")
const MANIFEST_PATH = path.join(ANDROID_APP_DIR, "AndroidManifest.xml")
const JNI_ARM64_DIR = path.join(ANDROID_APP_DIR, "jniLibs", "arm64-v8a")

function main() {
	console.log("Preparing Android project configuration...")

	if (!fs.existsSync(ANDROID_GEN_DIR)) {
		console.error("Android project not found at:", ANDROID_GEN_DIR)
		console.error("Please run `pnpm tauri android init` first.")
		process.exit(1)
	}

	// 1. Copy Kotlin Foreground Service and MainActivity
	fs.mkdirSync(JAVA_PKG_DIR, { recursive: true })
	const serviceSrc = path.join(ROOT_DIR, "src-tauri", "android", "ServerHostService.kt")
	const serviceDest = path.join(JAVA_PKG_DIR, "ServerHostService.kt")
	if (fs.existsSync(serviceSrc)) {
		fs.copyFileSync(serviceSrc, serviceDest)
		console.log("Copied ServerHostService.kt ->", serviceDest)
	}

	const activitySrc = path.join(ROOT_DIR, "src-tauri", "android", "MainActivity.kt")
	const activityDest = path.join(JAVA_PKG_DIR, "MainActivity.kt")
	if (fs.existsSync(activitySrc)) {
		fs.copyFileSync(activitySrc, activityDest)
		console.log("Copied MainActivity.kt ->", activityDest)
	}

	// 2. Patch AndroidManifest.xml
	if (fs.existsSync(MANIFEST_PATH)) {
		let manifest = fs.readFileSync(MANIFEST_PATH, "utf8")

		const permissions = [
			'<uses-permission android:name="android.permission.INTERNET" />',
			'<uses-permission android:name="android.permission.ACCESS_NETWORK_STATE" />',
			'<uses-permission android:name="android.permission.ACCESS_WIFI_STATE" />',
			'<uses-permission android:name="android.permission.WAKE_LOCK" />',
			'<uses-permission android:name="android.permission.FOREGROUND_SERVICE" />',
			'<uses-permission android:name="android.permission.FOREGROUND_SERVICE_SPECIAL_USE" />',
			'<uses-permission android:name="android.permission.POST_NOTIFICATIONS" />',
			'<uses-permission android:name="android.permission.REQUEST_IGNORE_BATTERY_OPTIMIZATIONS" />',
		]

		const missingPerms = permissions.filter((p) => !manifest.includes(p))
		if (missingPerms.length > 0) {
			manifest = manifest.replace(
				"<application",
				`    ${missingPerms.join("\n    ")}\n\n    <application`,
			)
			console.log(`Injected ${missingPerms.length} permissions into AndroidManifest.xml`)
		}

		const serviceTag = `
        <service
            android:name=".ServerHostService"
            android:foregroundServiceType="specialUse"
            android:exported="false">
            <property
                android:name="android.app.PROPERTY_SPECIAL_USE_FGS_SUBTYPE"
                android:value="Hosting dedicated local and tunneled Minecraft game server on device"/>
        </service>`

		if (!manifest.includes("ServerHostService")) {
			manifest = manifest.replace("</application>", `${serviceTag}\n    </application>`)
			console.log("Injected ServerHostService into AndroidManifest.xml")
		}

		if (!manifest.includes("windowSoftInputMode")) {
			manifest = manifest.replace(
				'android:name=".MainActivity"',
				'android:windowSoftInputMode="adjustResize"\n            android:name=".MainActivity"',
			)
			console.log("Injected windowSoftInputMode into AndroidManifest.xml")
		}

		fs.writeFileSync(MANIFEST_PATH, manifest, "utf8")
	}

	// 3. Ensure arm64 jniLibs directory exists
	fs.mkdirSync(JNI_ARM64_DIR, { recursive: true })
	const prootDest = path.join(JNI_ARM64_DIR, "libproot.so")
	const localProot = path.join(ROOT_DIR, "src-tauri", "android", "libproot.so")

	if (fs.existsSync(localProot)) {
		fs.copyFileSync(localProot, prootDest)
		console.log("Copied local libproot.so ->", prootDest)
	}

	const resDir = path.join(ANDROID_APP_DIR, "res")

	// 4. App icon (ingot) and name. `tauri android init` generates the default Tauri
	// icon, so copy our adaptive icon set over it and drop Tauri's vector drawables.
	const iconSrcDir = path.join(ROOT_DIR, "src-tauri", "icons", "android")
	if (fs.existsSync(iconSrcDir)) {
		fs.cpSync(iconSrcDir, resDir, { recursive: true })
		for (const stale of [
			path.join("drawable-v24", "ic_launcher_foreground.xml"),
			path.join("drawable", "ic_launcher_background.xml"),
		]) {
			fs.rmSync(path.join(resDir, stale), { force: true })
		}
		console.log("Copied ingot launcher icons")
	}
	const stringsPath = path.join(resDir, "values", "strings.xml")
	if (fs.existsSync(stringsPath)) {
		const strings = fs
			.readFileSync(stringsPath, "utf8")
			.replace(/(<string name="(?:app_name|main_activity_title)">)[^<]*(<\/string>)/g, '$1"Ingot"$2')
		fs.writeFileSync(stringsPath, strings, "utf8")
	}

	// 5. Ensure dark theme background for status bar & navigation bar
	const colorsPath = path.join(resDir, "values", "colors.xml")
	if (fs.existsSync(colorsPath)) {
		let colors = fs.readFileSync(colorsPath, "utf8")
		if (!colors.includes('name="background"')) {
			colors = colors.replace('</resources>', '    <color name="background">#09090b</color>\n</resources>')
			fs.writeFileSync(colorsPath, colors, "utf8")
		}
	}
	for (const valuesFolder of ["values", "values-night"]) {
		const themesPath = path.join(resDir, valuesFolder, "themes.xml")
		if (fs.existsSync(themesPath)) {
			let themes = fs.readFileSync(themesPath, "utf8")
			if (!themes.includes("android:windowBackground")) {
				themes = themes.replace(
					'<style name="Theme.ingot" parent="Theme.MaterialComponents.DayNight.NoActionBar">',
					'<style name="Theme.ingot" parent="Theme.MaterialComponents.DayNight.NoActionBar">\n        <item name="android:windowBackground">@color/background</item>',
				)
				fs.writeFileSync(themesPath, themes, "utf8")
			}
		}
	}

	console.log("Android project prepared successfully!")
}

try {
	main()
} catch (err) {
	console.error("Error preparing Android:", err)
	process.exit(1)
}
