import { afterEach, describe, expect, it } from "vitest"
import {
	AVAILABLE_TRANSLATION_LOCALES,
	i18n,
	resolveTranslationLocale,
	syncI18nLanguage,
} from "@/shared/i18n/i18n"

afterEach(async () => {
	await syncI18nLanguage("en")
})

describe("i18n runtime & fallback configuration", () => {
	it("loads English translations from public/locales/en.json", () => {
		expect(AVAILABLE_TRANSLATION_LOCALES).toContain("en")
		expect(i18n.t("settings.language.title")).toBe("Display Language")
		expect(i18n.t("settings.java.title", { ramGb: 8 })).toBe("Default Memory Allocation (8 GB)")
	})

	it("provides English keys across shell, play, and feature namespaces", () => {
		expect(i18n.t("nav.items.play.label")).toBe("Play")
		expect(i18n.t("shell.statusActive", { count: 2 })).toBe("2 Active")
		expect(i18n.t("shell.notifications.unreadAria", { count: 3 })).toBe("Notifications (3 unread)")
		expect(i18n.t("errors.notFound.badge")).toBe("404 • ROUTE NOT FOUND")
		expect(i18n.t("play.allInstancesHeading", { count: 5 })).toBe("All Instances (5)")
		expect(i18n.t("instanceDetail.jvm.title", { ram: 6 })).toBe(
			"Per-Instance Memory Override (6 GB)",
		)
		expect(i18n.t("modpacks.installTitle", { name: "Fabulously Optimized" })).toBe(
			"Install Fabulously Optimized",
		)
		expect(i18n.t("news.title")).toBe("News & Events")
		expect(i18n.t("worlds.playWorld")).toBe("Play World")
		expect(i18n.t("servers.addServer")).toBe("Add Server")
		expect(i18n.t("skins.capePreset", { cape: "Migrator" })).toBe("Cape preset: Migrator")
		expect(i18n.t("screenshots.copyName")).toBe("Copy Name")
		expect(i18n.t("toastPicker.screenPosition")).toBe("Screen Position")
		expect(i18n.t("settings.customTheme.loadedPresetToast", { preset: "Dark" })).toBe(
			"Loaded Dark preset into Custom theme.",
		)
	})

	it("falls back untranslated locales to en while syncing document lang attribute", async () => {
		expect(resolveTranslationLocale("en")).toBe("en")
		expect(resolveTranslationLocale("uk")).toBe("en")

		await syncI18nLanguage("uk")

		expect(document.documentElement.lang).toBe("uk")
		expect(i18n.language).toBe("en")
		expect(i18n.t("settings.tabs.language")).toBe("Language")
	})
})
