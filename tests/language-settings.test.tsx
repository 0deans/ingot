import { fireEvent, render, screen } from "@testing-library/react"
import { beforeEach, describe, expect, it } from "vitest"
import LanguageSettingsPage from "@/features/settings/LanguageSettingsPage"
import {
	APP_LANGUAGES,
	detectSystemLocale,
	filterLanguages,
	formatLocalePreview,
	resolveEffectiveLocale,
} from "@/shared/i18n/languages"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"

beforeEach(() => {
	useLauncherPrefsStore.setState({
		language: "system",
	})
})

describe("Language settings & catalog", () => {
	it("detects supported locales from system languages and resolves preference cleanly", () => {
		expect(APP_LANGUAGES).toHaveLength(22)
		expect(detectSystemLocale(["uk-UA", "en-US"])).toBe("uk")
		expect(detectSystemLocale(["xx-YY"], "de-AT")).toBe("de")
		expect(detectSystemLocale(["xx-YY"], "zz")).toBe("en")

		expect(resolveEffectiveLocale("system", "ja")).toBe("ja")
		expect(resolveEffectiveLocale("fr", "ja")).toBe("fr")

		expect(filterLanguages("укра")).toHaveLength(1)
		expect(filterLanguages("uk")[0]?.code).toBe("uk")

		const enPreview = formatLocalePreview("en")
		expect(enPreview.sizeSample).toBe("2.45 GB")
		expect(enPreview.numberSample).toBe("12,480")
		expect(enPreview.timeSample.length).toBeGreaterThan(0)
	})

	it("filters languages, selects a language, and restores System Default", () => {
		render(<LanguageSettingsPage />)

		const systemBtn = screen.getByRole("button", { name: /Follow System Language/i })
		expect(systemBtn.getAttribute("aria-pressed")).toBe("true")

		const searchInput = screen.getByRole("textbox", { name: "Search languages" })
		fireEvent.change(searchInput, { target: { value: "Укра" } })

		const ukrainianBtn = screen.getByRole("button", { name: /Українська/i })
		fireEvent.click(ukrainianBtn)

		expect(useLauncherPrefsStore.getState().language).toBe("uk")
		expect(ukrainianBtn.getAttribute("aria-pressed")).toBe("true")
		expect(systemBtn.getAttribute("aria-pressed")).toBe("false")

		fireEvent.change(searchInput, { target: { value: "nonexistent-lang" } })
		expect(screen.getByText(/No languages matched/i)).toBeTruthy()
		fireEvent.click(screen.getByRole("button", { name: "Clear search" }))
		expect(screen.getByRole("button", { name: /Deutsch/i })).toBeTruthy()

		fireEvent.click(systemBtn)
		expect(useLauncherPrefsStore.getState().language).toBe("system")
		expect(systemBtn.getAttribute("aria-pressed")).toBe("true")
	})
})
