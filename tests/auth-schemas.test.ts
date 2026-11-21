import { describe, expect, it } from "vitest"
import {
	createOfflineAccountSchema,
	elyLoginSchema,
	offlineAccountSchema,
} from "@/features/accounts/authSchemas"

describe("elyLoginSchema", () => {
	it("accepts valid trimmed username and non-empty password", () => {
		const result = elyLoginSchema.safeParse({
			username: "  steve@example.com  ",
			password: "secret-password",
		})

		expect(result.success).toBe(true)
		if (result.success) {
			expect(result.data).toEqual({
				username: "steve@example.com",
				password: "secret-password",
			})
		}
	})

	it("rejects empty or whitespace-only username", () => {
		const result = elyLoginSchema.safeParse({
			username: "   ",
			password: "secret-password",
		})

		expect(result.success).toBe(false)
		if (!result.success) {
			expect(result.error.issues[0]?.message).toBe("accounts.auth.ely.usernameRequired")
		}
	})

	it("rejects empty password", () => {
		const result = elyLoginSchema.safeParse({
			username: "steve",
			password: "",
		})

		expect(result.success).toBe(false)
		if (!result.success) {
			expect(result.error.issues[0]?.message).toBe("accounts.auth.ely.passwordRequired")
		}
	})
})

describe("offlineAccountSchema", () => {
	it("accepts valid 3-16 character alphanumeric/underscore username", () => {
		const result = offlineAccountSchema.safeParse({
			username: "  Steve_123  ",
		})

		expect(result.success).toBe(true)
		if (result.success) {
			expect(result.data).toEqual({ username: "Steve_123" })
		}
	})

	it("rejects username shorter than 3 characters", () => {
		const result = offlineAccountSchema.safeParse({
			username: "ab",
		})

		expect(result.success).toBe(false)
		if (!result.success) {
			expect(result.error.issues[0]?.message).toBe("accounts.auth.offline.usernameTooShort")
		}
	})

	it("rejects username longer than 16 characters", () => {
		const result = offlineAccountSchema.safeParse({
			username: "a".repeat(17),
		})

		expect(result.success).toBe(false)
		if (!result.success) {
			expect(result.error.issues[0]?.message).toBe("accounts.auth.offline.usernameTooLong")
		}
	})

	it("rejects username with invalid characters", () => {
		const result = offlineAccountSchema.safeParse({
			username: "steve-123",
		})

		expect(result.success).toBe(false)
		if (!result.success) {
			expect(result.error.issues[0]?.message).toBe("accounts.auth.offline.usernameInvalidChars")
		}
	})
})

describe("createOfflineAccountSchema", () => {
	const schema = createOfflineAccountSchema(new Set(["steve_123"]))

	it("rejects exact duplicate username", () => {
		const result = schema.safeParse({
			username: "steve_123",
		})

		expect(result.success).toBe(false)
		if (!result.success) {
			expect(result.error.issues[0]?.message).toBe("accounts.auth.offline.usernameExists")
		}
	})

	it("rejects different-case duplicate username", () => {
		const result = schema.safeParse({
			username: "  Steve_123  ",
		})

		expect(result.success).toBe(false)
		if (!result.success) {
			expect(result.error.issues[0]?.message).toBe("accounts.auth.offline.usernameExists")
		}
	})

	it("accepts non-duplicate username", () => {
		const result = schema.safeParse({
			username: "Alex_456",
		})

		expect(result.success).toBe(true)
		if (result.success) {
			expect(result.data).toEqual({ username: "Alex_456" })
		}
	})
})
