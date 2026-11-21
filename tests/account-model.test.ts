import { describe, expect, test } from "vitest"
import { type AccountProfile as BackendAccountProfile } from "@/bindings"
import {
	findActiveAccount,
	getAccountInitials,
	mapBackendAccount,
	normalizeAccountType,
	type UiAccountProfile,
} from "@/features/accounts/accountModel"

const createBackendAccount = (
	overrides: Partial<BackendAccountProfile> = {},
): BackendAccountProfile => {
	return {
		id: "acc-1",
		username: "Steve",
		accountType: "microsoft",
		uuid: "00000000-0000-0000-0000-000000000001",
		skinUrl: null,
		isActive: false,
		createdAt: 1700000000,
		...overrides,
	}
}

const createUiAccount = (overrides: Partial<UiAccountProfile> = {}): UiAccountProfile => {
	return {
		...mapBackendAccount(createBackendAccount()),
		...overrides,
	}
}

describe("normalizeAccountType", () => {
	test("normalizes known provider variants case-insensitively with whitespace trimmed", () => {
		expect(normalizeAccountType("microsoft")).toBe("Microsoft")
		expect(normalizeAccountType("  MSA  ")).toBe("Microsoft")
		expect(normalizeAccountType("ely")).toBe("Ely.by")
		expect(normalizeAccountType("Ely.by")).toBe("Ely.by")
		expect(normalizeAccountType("ELYBY")).toBe("Ely.by")
	})

	test("falls back to Offline for offline and unrecognized account types", () => {
		expect(normalizeAccountType("offline")).toBe("Offline")
		expect(normalizeAccountType("")).toBe("Offline")
		expect(normalizeAccountType("unknown-provider")).toBe("Offline")
	})
})

describe("mapBackendAccount", () => {
	test("maps backend fields and derives normalized type, badgeColor, and deterministic avatarBg", () => {
		const backend = createBackendAccount({
			id: "msa-42",
			username: "Alex",
			accountType: "msa",
			uuid: "11111111-1111-1111-1111-111111111111",
			skinUrl: "https://example.com/skin.png",
			isActive: true,
			createdAt: 1712345678,
		})

		const mapped = mapBackendAccount(backend)
		const mappedAgain = mapBackendAccount(backend)

		expect(mapped.id).toBe("msa-42")
		expect(mapped.username).toBe("Alex")
		expect(mapped.type).toBe("Microsoft")
		expect(mapped.uuid).toBe("11111111-1111-1111-1111-111111111111")
		expect(mapped.skinUrl).toBe("https://example.com/skin.png")
		expect(mapped.isActive).toBe(true)
		expect(mapped.createdAt).toBe(1712345678)
		expect(mapped.badgeColor).toContain("emerald")
		expect(mapped.avatarBg).toMatch(/^from-.+ to-.+$/)
		expect(mapped.avatarBg).toBe(mappedAgain.avatarBg)
	})

	test("falls back to uuid or username when id is empty for avatar gradient derivation", () => {
		const withoutId = mapBackendAccount(
			createBackendAccount({
				id: "",
				uuid: "uuid-seed",
				accountType: "ely.by",
			}),
		)

		expect(withoutId.type).toBe("Ely.by")
		expect(withoutId.badgeColor).toContain("sky")
		expect(withoutId.avatarBg).toMatch(/^from-.+ to-.+$/)
	})
})

describe("findActiveAccount", () => {
	test("returns null for an empty list", () => {
		expect(findActiveAccount([])).toBeNull()
	})

	test("returns the account marked as active", () => {
		const first = createUiAccount({ id: "acc-1", username: "First", isActive: false })
		const second = createUiAccount({ id: "acc-2", username: "Second", isActive: true })
		const third = createUiAccount({ id: "acc-3", username: "Third", isActive: false })

		expect(findActiveAccount([first, second, third])).toBe(second)
	})

	test("falls back to the first account when none is marked active", () => {
		const first = createUiAccount({ id: "acc-1", username: "First", isActive: false })
		const second = createUiAccount({ id: "acc-2", username: "Second", isActive: false })

		expect(findActiveAccount([first, second])).toBe(first)
	})
})

describe("getAccountInitials", () => {
	test("returns MC for empty or whitespace-only usernames", () => {
		expect(getAccountInitials("")).toBe("MC")
		expect(getAccountInitials("   ")).toBe("MC")
	})

	test("uses the first letter of the first two segments separated by underscore, hyphen, or space", () => {
		expect(getAccountInitials("alex_smith")).toBe("AS")
		expect(getAccountInitials("red-stone")).toBe("RS")
		expect(getAccountInitials("  iron   golem  ")).toBe("IG")
	})

	test("uses the first two characters for single-token usernames", () => {
		expect(getAccountInitials("notch")).toBe("NO")
		expect(getAccountInitials("a")).toBe("A")
	})
})
