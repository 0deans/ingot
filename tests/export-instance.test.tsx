import { fireEvent, render, screen, waitFor } from "@testing-library/react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import type { ExportPreview, ExportReport, InstanceConfig } from "@/bindings"
import ExportInstanceView from "@/components/instances/export-instance-view"
import { instanceService } from "@/services/instance-service"

vi.mock("react-i18next", () => {
	return {
		useTranslation: () => {
			return {
				t: (key: string, options?: Record<string, unknown>) => {
					if (options && "path" in options) {
						return `${key}:${String(options.path)}`
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

vi.mock("@tauri-apps/plugin-dialog", () => {
	return {
		save: vi.fn(),
	}
})

vi.mock("@/lib/share", () => {
	return {
		canShareFiles: () => {
			return false
		},
		shareFile: vi.fn(),
	}
})

vi.mock("@/services/instance-service", () => {
	return {
		instanceService: {
			previewExport: vi.fn(),
			exportInstance: vi.fn(),
		},
	}
})

describe("ExportInstanceView component", () => {
	const sampleInstance: InstanceConfig = {
		id: "inst-1",
		name: "My Fabric Pack",
		gameVersion: "1.20.1",
		loader: "fabric",
		loaderVersion: "0.15.11",
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

	const samplePreview: ExportPreview = {
		defaultName: "My Fabric Pack",
		defaultVersion: "1.0.0",
		entries: [
			{
				path: "mods/fabric-api.jar",
				sizeBytes: 1024 * 100,
				defaultIncluded: true,
			},
			{
				path: "config/fabric.json",
				sizeBytes: 512,
				defaultIncluded: true,
			},
			{
				path: "options.txt",
				sizeBytes: 256,
				defaultIncluded: false,
			},
		],
		totalSizeBytes: 1024 * 100 + 512,
	}

	beforeEach(() => {
		vi.clearAllMocks()
	})

	it("renders preview with default included files selected", async () => {
		vi.mocked(instanceService.previewExport).mockResolvedValue(samplePreview)

		render(
			<ExportInstanceView instance={sampleInstance} isInstanceRunning={false} onBack={vi.fn()} />,
		)

		await waitFor(() => {
			expect(screen.getByDisplayValue("My Fabric Pack")).toBeDefined()
			expect(screen.getByDisplayValue("1.0.0")).toBeDefined()
		})

		expect(screen.getByText("fabric-api.jar")).toBeDefined()
		expect(screen.getByText("fabric.json")).toBeDefined()
	})

	it("renders running instance warning alert when isInstanceRunning is true", async () => {
		vi.mocked(instanceService.previewExport).mockResolvedValue(samplePreview)

		render(
			<ExportInstanceView instance={sampleInstance} isInstanceRunning={true} onBack={vi.fn()} />,
		)

		await waitFor(() => {
			expect(screen.getByText("exportInstance.runningWarningTitle")).toBeDefined()
		})
	})

	it("calls exportInstance with selected options on submit", async () => {
		vi.mocked(instanceService.previewExport).mockResolvedValue(samplePreview)
		const { save } = await import("@tauri-apps/plugin-dialog")
		vi.mocked(save).mockResolvedValue("C:/fake/export/pack.mrpack")

		const mockReport: ExportReport = {
			outputPath: "C:/fake/export/pack.mrpack",
			totalFiles: 2,
			resolvedFiles: 1,
			embeddedFiles: 1,
			embeddedWarnings: ["mods/unknown.jar: Not found on Modrinth"],
			warnings: [],
		}
		vi.mocked(instanceService.exportInstance).mockResolvedValue(mockReport)

		render(
			<ExportInstanceView instance={sampleInstance} isInstanceRunning={false} onBack={vi.fn()} />,
		)

		await waitFor(() => {
			expect(screen.getByDisplayValue("My Fabric Pack")).toBeDefined()
		})

		const exportBtn = screen.getByRole("button", { name: /exportButton/i })
		fireEvent.click(exportBtn)

		await waitFor(() => {
			expect(instanceService.exportInstance).toHaveBeenCalledTimes(1)
		})

		const calledOptions = vi.mocked(instanceService.exportInstance).mock.calls[0][1]
		expect(calledOptions.format).toBe("modrinth")
		expect(calledOptions.name).toBe("My Fabric Pack")
		expect(calledOptions.version).toBe("1.0.0")
		expect(calledOptions.includedPaths).toContain("mods/fabric-api.jar")
		expect(calledOptions.includedPaths).toContain("config/fabric.json")
		expect(calledOptions.includedPaths).not.toContain("options.txt")

		// Success report rendered
		await waitFor(() => {
			expect(screen.getByText("exportInstance.exportSuccessTitle")).toBeDefined()
			expect(screen.getByText("• mods/unknown.jar: Not found on Modrinth")).toBeDefined()
		})
	})

	it("shows error state when export fails", async () => {
		vi.mocked(instanceService.previewExport).mockResolvedValue(samplePreview)
		const { save } = await import("@tauri-apps/plugin-dialog")
		vi.mocked(save).mockResolvedValue("C:/fake/export/pack.mrpack")
		vi.mocked(instanceService.exportInstance).mockRejectedValue(
			new Error("Modrinth provider request timed out"),
		)

		render(
			<ExportInstanceView instance={sampleInstance} isInstanceRunning={false} onBack={vi.fn()} />,
		)

		await waitFor(() => {
			expect(screen.getByDisplayValue("My Fabric Pack")).toBeDefined()
		})

		const exportBtn = screen.getByRole("button", { name: /exportButton/i })
		fireEvent.click(exportBtn)

		await waitFor(() => {
			expect(screen.getByText("Modrinth provider request timed out")).toBeDefined()
		})
	})

	it("switches back to files tab when changing export format away from modrinth", async () => {
		vi.mocked(instanceService.previewExport).mockResolvedValue(samplePreview)

		render(
			<ExportInstanceView instance={sampleInstance} isInstanceRunning={false} onBack={vi.fn()} />,
		)

		await waitFor(() => {
			expect(screen.getByDisplayValue("My Fabric Pack")).toBeDefined()
		})

		const sidesTab = screen.getByRole("tab", { name: /exportInstance.sidesTab/i })
		fireEvent.click(sidesTab)

		await waitFor(() => {
			expect(screen.getByText("exportInstance.fileColumn")).toBeDefined()
		})

		const cfRadio = screen.getByRole("radio", { name: /exportInstance.formatCurseForge/i })
		fireEvent.click(cfRadio)

		await waitFor(() => {
			expect(screen.queryByText("exportInstance.fileColumn")).toBeNull()
			expect(screen.getByText("fabric-api.jar")).toBeDefined()
			expect(
				sidesTab.hasAttribute("disabled") || sidesTab.getAttribute("aria-disabled") === "true",
			).toBe(true)
		})
	})
})
