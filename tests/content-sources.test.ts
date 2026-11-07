import * as v from "valibot"
import { describe, expect, it } from "vitest"
import { getSourceBadgeInfo, sourceSchema } from "@/routes/modpacks"
import { isModpackOnlySource, stripContentSourcePrefix } from "@/services/content-service"

describe("content source helpers", () => {
	it("strips prefixes for all supported sources", () => {
		expect(stripContentSourcePrefix("mr:12345")).toBe("12345")
		expect(stripContentSourcePrefix("cf:67890")).toBe("67890")
		expect(stripContentSourcePrefix("ftb:101")).toBe("101")
		expect(stripContentSourcePrefix("technic:tekkit-2")).toBe("tekkit-2")
		expect(stripContentSourcePrefix("atl:skyfactory-4")).toBe("skyfactory-4")
		expect(stripContentSourcePrefix("modrinth:abc")).toBe("abc")
		expect(stripContentSourcePrefix("curseforge:xyz")).toBe("xyz")
		expect(stripContentSourcePrefix("atlauncher:pack1")).toBe("pack1")
		expect(stripContentSourcePrefix("plain-id")).toBe("plain-id")
	})

	it("identifies modpack-only sources accurately", () => {
		expect(isModpackOnlySource("ftb")).toBe(true)
		expect(isModpackOnlySource("technic")).toBe(true)
		expect(isModpackOnlySource("atlauncher")).toBe(true)

		expect(isModpackOnlySource("all")).toBe(false)
		expect(isModpackOnlySource("modrinth")).toBe(false)
		expect(isModpackOnlySource("curseforge")).toBe(false)
	})

	it("validates all supported sources in sourceSchema", () => {
		expect(v.is(sourceSchema, "all")).toBe(true)
		expect(v.is(sourceSchema, "modrinth")).toBe(true)
		expect(v.is(sourceSchema, "curseforge")).toBe(true)
		expect(v.is(sourceSchema, "ftb")).toBe(true)
		expect(v.is(sourceSchema, "technic")).toBe(true)
		expect(v.is(sourceSchema, "atlauncher")).toBe(true)
		expect(v.is(sourceSchema, "unknown")).toBe(false)
	})

	it("provides correct badge styling and icons for each source", () => {
		const mockTranslate = (key: string) => {
			return key
		}

		const ftbBadge = getSourceBadgeInfo("ftb", mockTranslate)
		expect(ftbBadge.className).toContain("text-red-400")
		expect(ftbBadge.icon).toBeDefined()

		const technicBadge = getSourceBadgeInfo("technic", mockTranslate)
		expect(technicBadge.className).toContain("text-blue-400")
		expect(technicBadge.icon).toBeDefined()

		const atlBadge = getSourceBadgeInfo("atlauncher", mockTranslate)
		expect(atlBadge.className).toContain("text-sky-400")
		expect(atlBadge.icon).toBeDefined()

		const cfBadge = getSourceBadgeInfo("curseforge", mockTranslate)
		expect(cfBadge.className).toContain("text-warning")
		expect(cfBadge.icon).toBeDefined()

		const mrBadge = getSourceBadgeInfo("modrinth", mockTranslate)
		expect(mrBadge.className).toContain("text-primary")
		expect(mrBadge.icon).toBeDefined()
	})
})

describe("modpack background installation and loader normalization", () => {
	it("normalizes loader strings safely without type assertions", async () => {
		const { normalizeLoader } = await import("@/components/content/install-dialog")
		expect(normalizeLoader("fabric")).toBe("fabric")
		expect(normalizeLoader("quilt")).toBe("quilt")
		expect(normalizeLoader("neoforge")).toBe("neoforge")
		expect(normalizeLoader("forge")).toBe("forge")
		expect(normalizeLoader("horizon")).toBe("horizon")
		expect(normalizeLoader("vanilla")).toBe("vanilla")
		expect(normalizeLoader(null)).toBe("forge")
		expect(normalizeLoader("custom-unknown")).toBe("forge")
	})

	it("registers and finishes pending modpack installs in instanceService", async () => {
		const { instanceService } = await import("@/services/instance-service")
		const trackingId = instanceService.registerPendingInstall({
			name: "All The Mods 9",
			gameVersion: "1.20.1",
			loader: "forge",
		})

		expect(trackingId).toBe("installing:All The Mods 9")
		expect(instanceService.isPendingInstall(trackingId)).toBe(true)

		const instances = await instanceService.getInstances()
		const pendingInst = instances.find((i) => {
			return i.id === trackingId
		})
		expect(pendingInst).toBeDefined()
		expect(pendingInst?.name).toBe("All The Mods 9")
		expect(pendingInst?.loader).toBe("forge")

		instanceService.finishPendingInstall(trackingId)
		expect(instanceService.isPendingInstall(trackingId)).toBe(false)
	})
})
