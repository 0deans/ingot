import { fireEvent, render, screen } from "@testing-library/react"
import { describe, expect, it, vi } from "vitest"
import type { InstalledContent } from "@/bindings"
import ContentRow from "@/components/instance-content/content-row"

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
