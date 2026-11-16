// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen } from "@testing-library/react"
import { createElement } from "react"
import { afterEach, describe, expect, test, vi } from "vitest"
import ColorPicker from "@/shared/components/ColorPicker"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"
import {
	applyCustomThemeVariables,
	clearCustomThemeVariables,
} from "@/shared/theme/applyCustomTheme"
import {
	hsvToRgb,
	parseCssColorToHsv,
	parseCssColorToRgb,
	rgbToHex,
	rgbToHsv,
} from "@/shared/theme/colorConversion"
import {
	DEFAULT_CUSTOM_THEME,
	THEME_PRESETS,
	parseCustomThemeJson,
	stringifyCustomThemeJson,
	toColorPickerHex,
	validateCssColor,
} from "@/shared/theme/customTheme"

afterEach(() => {
	cleanup()
	clearCustomThemeVariables(document.documentElement)
	useLauncherPrefsStore.getState().resetCustomTheme()
})

describe("customTheme JSON serialization and validation", () => {
	test("round-trips valid preset JSON cleanly", () => {
		const serialized = stringifyCustomThemeJson(THEME_PRESETS.purple)
		const parsed = parseCustomThemeJson(serialized)

		expect(parsed.ok).toBe(true)
		if (parsed.ok) {
			expect(parsed.value).toEqual(THEME_PRESETS.purple)
		}
	})

	test("accepts valid oklch(), 3-digit hex color formats, and 0 radiusRem", () => {
		const oklchConfig = {
			...THEME_PRESETS.light,
			radiusRem: 0,
			colors: {
				...THEME_PRESETS.light.colors,
				primary: "oklch(0.62 0.21 292)",
				background: "#fff",
			},
		}
		const parsed = parseCustomThemeJson(JSON.stringify(oklchConfig))

		expect(parsed.ok).toBe(true)
		if (parsed.ok) {
			expect(parsed.value.radiusRem).toBe(0)
			expect(parsed.value.colors.primary).toBe("oklch(0.62 0.21 292)")
			expect(parsed.value.colors.background).toBe("#fff")
		}
	})

	test("rejects empty and malformed JSON inputs", () => {
		const emptyResult = parseCustomThemeJson("   ")
		expect(emptyResult.ok).toBe(false)

		const malformedResult = parseCustomThemeJson("{ broken json")
		expect(malformedResult.ok).toBe(false)
	})

	test("rejects out-of-range radiusRem and unsafe CSS expressions", () => {
		const unsafePayload = {
			...THEME_PRESETS.dark,
			radiusRem: 4.5,
			colors: {
				...THEME_PRESETS.dark.colors,
				background: "url(https://evil.example/bg.png)",
				primary: "expression(alert(1))",
			},
		}
		const parsed = parseCustomThemeJson(JSON.stringify(unsafePayload))

		expect(parsed.ok).toBe(false)
		if (!parsed.ok) {
			expect(
				parsed.errors.some((msg) => {
					return msg.includes("radiusRem")
				}),
			).toBe(true)
			expect(
				parsed.errors.some((msg) => {
					return msg.includes("colors.background")
				}),
			).toBe(true)
			expect(
				parsed.errors.some((msg) => {
					return msg.includes("colors.primary")
				}),
			).toBe(true)
		}
	})
})

describe("validateCssColor", () => {
	test("accepts valid hex and CSS color functions", () => {
		expect(validateCssColor("#fff").ok).toBe(true)
		expect(validateCssColor("#ffff").ok).toBe(true)
		expect(validateCssColor("#10b981").ok).toBe(true)
		expect(validateCssColor("#10b981cc").ok).toBe(true)
		expect(validateCssColor("rgb(16, 185, 129)").ok).toBe(true)
		expect(validateCssColor("rgb(16 185 129 / 80%)").ok).toBe(true)
		expect(validateCssColor("hsl(160deg 84% 39%)").ok).toBe(true)
		expect(validateCssColor("oklch(0.62 0.21 292 / 0.9)").ok).toBe(true)
		expect(validateCssColor("oklab(0.65 -0.12 0.08)").ok).toBe(true)
	})

	test("rejects overlong hex, invalid functions, and out-of-range channels", () => {
		expect(validateCssColor("").ok).toBe(false)
		expect(validateCssColor("#f4f4f56758493").ok).toBe(false)
		expect(validateCssColor("#12").ok).toBe(false)
		expect(validateCssColor("#12345").ok).toBe(false)
		expect(validateCssColor("#gggggg").ok).toBe(false)
		expect(validateCssColor("rgb(300, 10, 10)").ok).toBe(false)
		expect(validateCssColor("hsl(200 120% 50%)").ok).toBe(false)
		expect(validateCssColor("oklch(1.5 0.2 180)").ok).toBe(false)
		expect(validateCssColor("oklch(0.5 -0.1 180)").ok).toBe(false)
		expect(validateCssColor("oklch(invalid)").ok).toBe(false)
	})
})

describe("toColorPickerHex", () => {
	test("normalizes 3-digit and 6-digit hex values and falls back for CSS functions", () => {
		expect(toColorPickerHex("#ABC")).toBe("#aabbcc")
		expect(toColorPickerHex(" #10B981 ")).toBe("#10b981")
		expect(toColorPickerHex("oklch(0.62 0.21 292)", "#a855f7")).toBe("#a855f7")
	})
})

describe("applyCustomThemeVariables & clearCustomThemeVariables", () => {
	test("applies custom CSS variables, contrast class, and clears them cleanly", () => {
		const target = document.createElement("div")

		applyCustomThemeVariables(THEME_PRESETS.light, target)
		expect(target.classList.contains("custom-light")).toBe(true)
		expect(target.classList.contains("custom-dark")).toBe(false)
		expect(target.style.colorScheme).toBe("light")
		expect(target.style.getPropertyValue("--background")).toBe(
			THEME_PRESETS.light.colors.background,
		)
		expect(target.style.getPropertyValue("--primary")).toBe(THEME_PRESETS.light.colors.primary)
		expect(target.style.getPropertyValue("--radius")).toBe(`${THEME_PRESETS.light.radiusRem}rem`)

		applyCustomThemeVariables(THEME_PRESETS.dark, target)
		expect(target.classList.contains("custom-dark")).toBe(true)
		expect(target.classList.contains("custom-light")).toBe(false)
		expect(target.style.colorScheme).toBe("dark")

		clearCustomThemeVariables(target)
		expect(target.classList.contains("custom-dark")).toBe(false)
		expect(target.classList.contains("custom-light")).toBe(false)
		expect(target.style.colorScheme).toBe("")
		expect(target.style.getPropertyValue("--background")).toBe("")
		expect(target.style.getPropertyValue("--primary")).toBe("")
	})
})

describe("useLauncherPrefsStore customTheme actions", () => {
	test("updates color, baseMode, clamped radius, and resets to presets", () => {
		const store = useLauncherPrefsStore.getState()

		store.updateCustomThemeColor("primary", "#38bdf8")
		expect(useLauncherPrefsStore.getState().customTheme.colors.primary).toBe("#38bdf8")

		store.updateCustomThemeColor("popoverForeground", "#f4f4f56758493")
		expect(useLauncherPrefsStore.getState().customTheme.colors.popoverForeground).toBe(
			DEFAULT_CUSTOM_THEME.colors.popoverForeground,
		)

		store.setCustomThemeBaseMode("light")
		expect(useLauncherPrefsStore.getState().customTheme.baseMode).toBe("light")

		store.setCustomThemeRadius(5)
		expect(useLauncherPrefsStore.getState().customTheme.radiusRem).toBe(1.5)

		store.setCustomThemeRadius(0)
		expect(useLauncherPrefsStore.getState().customTheme.radiusRem).toBe(0)

		store.setCustomThemeRadius(-1)
		expect(useLauncherPrefsStore.getState().customTheme.radiusRem).toBe(0)

		store.resetCustomTheme("purple")
		expect(useLauncherPrefsStore.getState().customTheme).toEqual(THEME_PRESETS.purple)

		store.resetCustomTheme()
		expect(useLauncherPrefsStore.getState().customTheme).toEqual(DEFAULT_CUSTOM_THEME)
	})
})

describe("colorConversion utilities", () => {
	test("parses hex, rgb(), hsl(), oklch(), and oklab() into RGB and round-trips HSV", () => {
		expect(parseCssColorToRgb("#10b981")).toEqual({ r: 16, g: 185, b: 129 })
		expect(parseCssColorToRgb("#fff")).toEqual({ r: 255, g: 255, b: 255 })
		expect(parseCssColorToRgb("rgb(16 185 129 / 80%)")).toEqual({ r: 16, g: 185, b: 129 })
		expect(parseCssColorToRgb("hsl(0 100% 50%)")).toEqual({ r: 255, g: 0, b: 0 })

		const oklchRgb = parseCssColorToRgb("oklch(0.62 0.21 292)")
		expect(oklchRgb).not.toBeNull()
		if (oklchRgb) {
			expect(oklchRgb.b).toBeGreaterThan(oklchRgb.g)
		}

		const oklabRgb = parseCssColorToRgb("oklab(1 0 0)")
		expect(oklabRgb).toEqual({ r: 255, g: 255, b: 255 })

		expect(parseCssColorToRgb("not-a-color")).toBeNull()

		const hsv = rgbToHsv({ r: 16, g: 185, b: 129 })
		expect(rgbToHex(hsvToRgb(hsv))).toBe("#10b981")
		expect(parseCssColorToHsv("invalid", "#38bdf8")).toEqual(rgbToHsv({ r: 56, g: 189, b: 248 }))
	})
})

describe("ColorPicker", () => {
	test("opens a custom popover without native inputs or eyedropper and updates color", () => {
		const handleChange = vi.fn()
		const { container } = render(
			createElement(ColorPicker, {
				label: "Primary",
				value: "rgb(16, 185, 129)",
				onChange: handleChange,
			}),
		)

		expect(container.querySelector('input[type="color"]')).toBeNull()

		const trigger = screen.getByRole("button", { name: "Primary color picker" })
		fireEvent.click(trigger)

		expect(screen.queryByRole("button", { name: "Pick color from screen" })).toBeNull()

		const blueInput = screen.getByLabelText("Primary B channel")
		expect(blueInput.getAttribute("type")).toBe("text")
		fireEvent.change(blueInput, { target: { value: "255" } })
		expect(handleChange).toHaveBeenCalledWith("#10b9ff")

		const swatch = screen.getByRole("button", { name: "Select swatch #a855f7" })
		fireEvent.click(swatch)

		expect(handleChange).toHaveBeenCalledWith("#a855f7")
	})
})
