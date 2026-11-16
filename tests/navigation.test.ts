import { describe, expect, test } from "vitest"
import { getPageTransitionKey } from "@/features/shell/navigation"

describe("getPageTransitionKey", () => {
	test("keeps a shared transition key across all settings tabs", () => {
		expect(getPageTransitionKey("/settings")).toBe("/settings")
		expect(getPageTransitionKey("/settings/")).toBe("/settings")
		expect(getPageTransitionKey("/settings/java")).toBe("/settings")
		expect(getPageTransitionKey("/settings/accounts")).toBe("/settings")
		expect(getPageTransitionKey("/settings/network")).toBe("/settings")
	})

	test("keeps a shared transition key across tabs of the same instance", () => {
		expect(getPageTransitionKey("/instances/inst-1/mods")).toBe("/instances/inst-1")
		expect(getPageTransitionKey("/instances/inst-1/worlds")).toBe("/instances/inst-1")
		expect(getPageTransitionKey("/instances/inst-1/logs")).toBe("/instances/inst-1")
		expect(getPageTransitionKey("/instances/inst-1/settings")).toBe("/instances/inst-1")
		expect(getPageTransitionKey("/instances/inst-2/mods")).toBe("/instances/inst-2")
	})

	test("preserves distinct keys for top-level routes and modpack details", () => {
		expect(getPageTransitionKey("/")).toBe("/")
		expect(getPageTransitionKey("/modpacks")).toBe("/modpacks")
		expect(getPageTransitionKey("/modpacks/mp-fabulously")).toBe("/modpacks/mp-fabulously")
	})
})
