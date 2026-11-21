import i18next from "i18next"
import { describe, expect, it } from "vitest"
import { extractFirstFormError, i18nMessage, resolveFormError } from "@/shared/i18n/formI18n"

const createTestTranslator = async () => {
	const instance = i18next.createInstance()
	await instance.init({
		lng: "en",
		resources: {
			en: {
				translation: {
					accounts: {
						auth: {
							ely: {
								usernameRequired: "Username is required.",
							},
							offline: {
								usernameTooShort: "Username must be at least {{min}} characters.",
							},
						},
					},
				},
			},
		},
	})
	return instance.t
}

describe("i18nMessage", () => {
	it("returns the plain key when params are omitted", () => {
		expect(i18nMessage("accounts.auth.ely.usernameRequired")).toBe(
			"accounts.auth.ely.usernameRequired",
		)
	})

	it("encodes key and params with the i18n prefix when params are provided", () => {
		const encoded = i18nMessage("accounts.auth.offline.usernameTooShort", { min: 3 })
		expect(encoded.startsWith("\u0000i18n:")).toBe(true)
	})
})

describe("extractFirstFormError", () => {
	it("returns the first string error", () => {
		expect(extractFirstFormError(["first error", "second error"])).toBe("first error")
	})

	it("extracts message from Standard Schema / Zod error objects", () => {
		expect(extractFirstFormError([{ message: "object error" }])).toBe("object error")
	})

	it("returns null for empty arrays", () => {
		expect(extractFirstFormError([])).toBeNull()
	})

	it("returns null for non-string, non-message-object errors", () => {
		expect(extractFirstFormError([42])).toBeNull()
		expect(extractFirstFormError([null])).toBeNull()
		expect(extractFirstFormError([{ code: "invalid" }])).toBeNull()
	})
})

describe("resolveFormError", () => {
	it("returns null when there are no errors", async () => {
		const t = await createTestTranslator()
		expect(resolveFormError(t, [])).toBeNull()
	})

	it("translates a plain i18n key", async () => {
		const t = await createTestTranslator()
		const key = i18nMessage("accounts.auth.ely.usernameRequired")
		expect(resolveFormError(t, [key])).toBe("Username is required.")
	})

	it("decodes tagged i18nMessage with params and passes them to t", async () => {
		const t = await createTestTranslator()
		const encoded = i18nMessage("accounts.auth.offline.usernameTooShort", { min: 3 })
		expect(resolveFormError(t, [{ message: encoded }])).toBe(
			"Username must be at least 3 characters.",
		)
	})

	it("handles malformed tagged JSON gracefully without throwing", async () => {
		const t = await createTestTranslator()
		const malformed = "\u0000i18n:{not-valid-json"
		expect(resolveFormError(t, [malformed])).toBe(malformed)
	})

	it("handles tagged JSON with invalid payload shape gracefully", async () => {
		const t = await createTestTranslator()
		const invalidPayload = `\u0000i18n:${JSON.stringify({ key: 123, params: "bad" })}`
		expect(resolveFormError(t, [invalidPayload])).toBe(invalidPayload)
	})
})
