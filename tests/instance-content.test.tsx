import { QueryClient, QueryClientProvider } from "@tanstack/react-query"
import { fireEvent, render, screen } from "@testing-library/react"
import { describe, expect, it, vi } from "vitest"
import type { ContentUpdateCandidate, InstanceConfig, InstalledContent } from "@/bindings"
import { isVersionCompatibleWithInstance, supportsMods } from "@/components/content/install-dialog"
import ContentRow from "@/components/instance-content/content-row"
import ContentUpdatesDialog from "@/components/instance-content/content-updates-dialog"
import ModsPanel from "@/components/instance-content/mods-panel"
import type { UnifiedContentVersion } from "@/services/content-service"

// Mock react-i18next
vi.mock("react-i18next", () => {
	return {
		useTranslation: () => {
			return {
				t: (key: string, options?: Record<string, unknown>) => {
					if (options && "name" in options) {
						return `${key}:${String(options.name)}`
					}
					if (options && "count" in options) {
						return `${key}:${String(options.count)}`
					}
					return key
				},
			}
		},
	}
})

describe("ContentRow component", () => {
	const sampleItem: InstalledContent = {
		fileName: "sodium-fabric-0.5.8.jar",
		category: "mod",
		state: "enabled",
		sizeBytes: 1024 * 500,
		modifiedAt: 1700000000,
		metadata: {
			name: "Sodium",
			modId: "sodium",
			version: "0.5.8",
			description: "Modern rendering engine for Minecraft",
			authors: ["JellySquid"],
			iconDataUrl: null,
			dependencies: [],
		},
		origin: {
			kind: "unknown",
		},
	}

	it("renders mod name, version, and filename", () => {
		render(
			<ContentRow
				item={sampleItem}
				isSelected={false}
				isBusy={false}
				isInstanceRunning={false}
				onToggleSelect={vi.fn()}
				onToggleEnabled={vi.fn()}
				onOpenDetails={vi.fn()}
				onDelete={vi.fn()}
			/>,
		)

		expect(screen.getByText("Sodium")).toBeDefined()
		expect(screen.getByText("0.5.8")).toBeDefined()
		expect(screen.getByText("sodium-fabric-0.5.8.jar")).toBeDefined()
		expect(screen.getByText("JellySquid")).toBeDefined()
	})

	it("disables the toggle switch when the instance is running", () => {
		render(
			<ContentRow
				item={sampleItem}
				isSelected={false}
				isBusy={false}
				isInstanceRunning={true}
				onToggleSelect={vi.fn()}
				onToggleEnabled={vi.fn()}
				onOpenDetails={vi.fn()}
				onDelete={vi.fn()}
			/>,
		)

		const toggle = screen.getByRole("switch")
		const isDisabled =
			toggle.hasAttribute("disabled") ||
			toggle.hasAttribute("data-disabled") ||
			toggle.getAttribute("aria-disabled") === "true"
		expect(isDisabled).toBe(true)
	})

	it("calls onOpenDetails when info button is clicked", () => {
		const handleOpenDetails = vi.fn()
		render(
			<ContentRow
				item={sampleItem}
				isSelected={false}
				isBusy={false}
				isInstanceRunning={false}
				onToggleSelect={vi.fn()}
				onToggleEnabled={vi.fn()}
				onOpenDetails={handleOpenDetails}
				onDelete={vi.fn()}
			/>,
		)

		const detailsButton = screen.getByLabelText("instanceContent.details")
		fireEvent.click(detailsButton)
		expect(handleOpenDetails).toHaveBeenCalledWith(sampleItem)
	})

	it("calls onDelete when delete button is clicked", () => {
		const handleDelete = vi.fn()
		render(
			<ContentRow
				item={sampleItem}
				isSelected={false}
				isBusy={false}
				isInstanceRunning={false}
				onToggleSelect={vi.fn()}
				onToggleEnabled={vi.fn()}
				onOpenDetails={vi.fn()}
				onDelete={handleDelete}
			/>,
		)

		const deleteButton = screen.getByLabelText("common.delete")
		fireEvent.click(deleteButton)
		expect(handleDelete).toHaveBeenCalledWith(sampleItem.fileName)
	})
})

describe("ContentUpdatesDialog component", () => {
	const sampleCandidate: ContentUpdateCandidate = {
		fileName: "sodium-fabric-0.5.8.jar",
		currentVersion: "0.5.8",
		latestVersion: "0.5.9",
		latestVersionId: "abc12345",
		title: "Sodium",
		downloadUrl: "https://example.com/sodium-0.5.9.jar",
		newFileName: "sodium-fabric-0.5.9.jar",
		channel: "release",
		source: "modrinth",
		sha1: "da39a3ee5e6b4b0d3255bfef95601890afd80709",
	}

	it("renders update candidates with versions and channel badge", () => {
		const queryClient = new QueryClient({
			defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
		})
		render(
			<QueryClientProvider client={queryClient}>
				<ContentUpdatesDialog
					instanceId="test-inst"
					isOpen={true}
					onClose={vi.fn()}
					candidates={[sampleCandidate]}
				/>
			</QueryClientProvider>,
		)

		expect(screen.getByText("Sodium")).toBeDefined()
		expect(screen.getByText("0.5.9")).toBeDefined()
		expect(screen.getByText("instanceContent.channelRelease")).toBeDefined()
		expect(screen.getByText("Modrinth")).toBeDefined()
	})

	it("renders beta and alpha badges accurately", () => {
		const betaCandidate: ContentUpdateCandidate = {
			...sampleCandidate,
			channel: "beta",
			source: "curseforge",
		}
		const queryClient = new QueryClient({
			defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
		})
		render(
			<QueryClientProvider client={queryClient}>
				<ContentUpdatesDialog
					instanceId="test-inst"
					isOpen={true}
					onClose={vi.fn()}
					candidates={[betaCandidate]}
				/>
			</QueryClientProvider>,
		)

		expect(screen.getByText("instanceContent.channelBeta")).toBeDefined()
		expect(screen.getByText("CurseForge")).toBeDefined()
	})
})

describe("Mod support and instance compatibility helpers", () => {
	it("correctly identifies which loaders support mods", () => {
		expect(supportsMods("vanilla")).toBe(false)
		expect(supportsMods("horizon")).toBe(false)
		expect(supportsMods("fabric")).toBe(true)
		expect(supportsMods("quilt")).toBe(true)
		expect(supportsMods("neoforge")).toBe(true)
		expect(supportsMods("forge")).toBe(true)
	})

	const baseInstance: InstanceConfig = {
		id: "inst-1",
		name: "Test Instance",
		gameVersion: "1.21.1",
		loader: "vanilla",
		loaderVersion: null,
		javaPath: null,
		memoryMinMb: null,
		memoryMaxMb: null,
		icon: null,
		createdAt: 0,
		lastPlayed: null,
		totalPlayTimeSeconds: 0,
		jvmArgs: null,
		fullscreen: null,
		windowWidth: null,
		windowHeight: null,
		syncOptions: null,
		syncServers: null,
		syncResourcePacks: null,
		syncCommandHistory: null,
		syncCreativeHotbars: null,
		lastSyncedAt: null,
		group: null,
		colorTag: null,
	}

	const fabricModVersion: UnifiedContentVersion = {
		id: "v-1",
		name: "Sodium 0.6.0",
		versionNumber: "0.6.0",
		gameVersions: ["1.21.1"],
		loaders: ["fabric"],
		versionType: "release",
		featured: false,
		downloads: 100,
		datePublished: "2024-01-01",
		filename: "sodium-fabric-0.6.0.jar",
		downloadUrl: "https://example.com/sodium.jar",
	}

	it("rejects mod installation on vanilla or horizon instances", () => {
		expect(isVersionCompatibleWithInstance(baseInstance, fabricModVersion, "mod")).toBe(false)

		const horizonInstance: InstanceConfig = {
			...baseInstance,
			loader: "horizon",
		}
		expect(isVersionCompatibleWithInstance(horizonInstance, fabricModVersion, "mod")).toBe(false)
	})

	it("accepts mod installation when loader and game version match", () => {
		const fabricInstance: InstanceConfig = {
			...baseInstance,
			loader: "fabric",
		}
		expect(isVersionCompatibleWithInstance(fabricInstance, fabricModVersion, "mod")).toBe(true)

		// Quilt can run Fabric mods
		const quiltInstance: InstanceConfig = {
			...baseInstance,
			loader: "quilt",
		}
		expect(isVersionCompatibleWithInstance(quiltInstance, fabricModVersion, "mod")).toBe(true)

		// Forge cannot run pure Fabric mods
		const forgeInstance: InstanceConfig = {
			...baseInstance,
			loader: "forge",
		}
		expect(isVersionCompatibleWithInstance(forgeInstance, fabricModVersion, "mod")).toBe(false)
	})

	it("allows resource packs on vanilla instances", () => {
		const resourcePackVersion: UnifiedContentVersion = {
			...fabricModVersion,
			loaders: ["minecraft"],
		}
		expect(isVersionCompatibleWithInstance(baseInstance, resourcePackVersion, "resourcepack")).toBe(
			true,
		)
	})
})

describe("ModsPanel component", () => {
	it("renders centered unsupported loader notice and hides toolbar when loader is horizon", () => {
		const queryClient = new QueryClient({
			defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
		})
		render(
			<QueryClientProvider client={queryClient}>
				<ModsPanel
					instanceId="test-inst"
					loader="horizon"
					isLoading={false}
					isInstanceRunning={false}
				/>
			</QueryClientProvider>,
		)

		expect(screen.getByText("instanceContent.unsupportedLoaderTitle")).toBeDefined()
		expect(screen.queryByPlaceholderText("instanceContent.searchMods")).toBeNull()
		expect(screen.queryByText("instanceContent.identify")).toBeNull()
	})

	it("renders toolbar with search input when loader supports mods", () => {
		const queryClient = new QueryClient({
			defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
		})
		render(
			<QueryClientProvider client={queryClient}>
				<ModsPanel
					instanceId="test-inst"
					loader="fabric"
					isLoading={false}
					isInstanceRunning={false}
				/>
			</QueryClientProvider>,
		)

		expect(screen.queryByText("instanceContent.unsupportedLoaderTitle")).toBeNull()
		expect(screen.getByPlaceholderText("instanceContent.searchMods")).toBeDefined()
	})
})
