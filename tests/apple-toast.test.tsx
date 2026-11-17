import { act, fireEvent, render, screen, waitFor } from "@testing-library/react"
import { toast } from "sonner"
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest"
import ToastPositionPicker from "@/features/ui-test/components/ToastPositionPicker"
import { appleToast, Toaster } from "@/shared/ui/sonner"

beforeAll(() => {
	if (typeof globalThis.ResizeObserver === "undefined") {
		class MockResizeObserver {
			observe() {}
			unobserve() {}
			disconnect() {}
		}
		globalThis.ResizeObserver = MockResizeObserver
	}
})

afterEach(() => {
	act(() => {
		toast.dismiss()
	})
	vi.useRealTimers()
})

describe("Apple-style Toaster", () => {
	it("renders success and error notifications with descriptions via sonner toast API", async () => {
		render(<Toaster position="bottom-right" duration={5000} />)

		act(() => {
			toast.success("Theme copied", {
				description: "JSON written to clipboard.",
			})
			toast.error("Sync failed", {
				description: "Unable to verify remote checksum.",
			})
		})

		expect(await screen.findByText("Theme copied")).toBeTruthy()
		expect(screen.getByText("JSON written to clipboard.")).toBeTruthy()
		expect(screen.getByText("Sync failed")).toBeTruthy()
		expect(screen.getByRole("status")).toBeTruthy()
		expect(screen.getByRole("alert")).toBeTruthy()
	})

	it("invokes action callback and dismisses when action button is clicked", async () => {
		const onAction = vi.fn()
		render(<Toaster position="bottom-right" duration={5000} />)

		act(() => {
			toast("Screenshot saved", {
				action: {
					label: "Reveal",
					onClick: onAction,
				},
			})
		})

		const actionButton = await screen.findByRole("button", { name: "Reveal" })
		fireEvent.click(actionButton)

		expect(onAction).toHaveBeenCalledTimes(1)
		await waitFor(() => {
			expect(screen.queryByText("Screenshot saved")).toBeNull()
		})
	})

	it("morphs a promise notification from loading to success", async () => {
		render(<Toaster position="bottom-right" duration={5000} />)

		let resolvePromise: ((value: string) => void) | undefined
		const pending = new Promise<string>((resolve) => {
			resolvePromise = resolve
		})

		act(() => {
			toast.promise(pending, {
				loading: "Verifying manifest...",
				success: (pack) => {
					return `${pack} ready`
				},
				error: "Verification failed",
			})
		})

		expect(await screen.findByText("Verifying manifest...")).toBeTruthy()

		await act(async () => {
			if (resolvePromise) {
				resolvePromise("Horizon")
			}
			await pending
		})

		expect(await screen.findByText("Horizon ready")).toBeTruthy()
	})

	it("auto-closes finite toasts and pauses countdown while stack is hovered", async () => {
		render(<Toaster position="bottom-right" duration={250} />)

		act(() => {
			toast.info("Hover pause test", { duration: 150 })
		})

		const titleEl = await screen.findByText("Hover pause test")
		const list = titleEl.closest("ol")
		expect(list).toBeTruthy()

		if (list) {
			fireEvent.mouseEnter(list)
		}

		await act(async () => {
			await new Promise((resolve) => {
				window.setTimeout(resolve, 220)
			})
		})

		expect(screen.queryByText("Hover pause test")).toBeTruthy()

		if (list) {
			fireEvent.mouseLeave(list)
		}

		await waitFor(
			() => {
				expect(screen.queryByText("Hover pause test")).toBeNull()
			},
			{ timeout: 1000 },
		)
	})

	it("renders cancel + action buttons, custom JSX cards, and no-icon/warning/loading tones", async () => {
		const onCancel = vi.fn()
		render(<Toaster position="bottom-right" duration={5000} />)

		act(() => {
			toast.warning("Overwrite keybinds?", {
				icon: null,
				cancel: {
					label: "Keep",
					onClick: onCancel,
				},
				action: {
					label: "Apply",
					onClick: vi.fn(),
				},
			})
			toast.loading("Downloading artifacts...")
			toast.custom(() => {
				return <div>Custom download progress card</div>
			})
		})

		expect(await screen.findByText("Overwrite keybinds?")).toBeTruthy()
		expect(screen.getByText("Downloading artifacts...")).toBeTruthy()
		expect(screen.getByText("Custom download progress card")).toBeTruthy()

		const keepButton = screen.getByRole("button", { name: "Keep" })
		fireEvent.click(keepButton)
		expect(onCancel).toHaveBeenCalledTimes(1)
	})

	it("renders determinate progress updates and morphs cleanly on completion", async () => {
		render(<Toaster position="bottom-right" duration={5000} />)

		let id: string | number = ""
		act(() => {
			id = appleToast.progress("Downloading Iris Shaders", {
				value: 25,
				detail: "16.0 / 64.0 MB",
			})
		})

		expect(await screen.findByText("Downloading Iris Shaders")).toBeTruthy()
		expect(screen.getByText("25%")).toBeTruthy()
		expect(screen.getByText("16.0 / 64.0 MB")).toBeTruthy()

		act(() => {
			appleToast.updateProgress(id, {
				value: 75,
				detail: "48.0 / 64.0 MB",
			})
		})

		expect(await screen.findByText("75%")).toBeTruthy()
		expect(screen.getByText("48.0 / 64.0 MB")).toBeTruthy()

		act(() => {
			appleToast.completeProgress(id, "Iris Shaders installed", {
				description: "Ready to launch.",
			})
		})

		expect(await screen.findByText("Iris Shaders installed")).toBeTruthy()
		expect(screen.queryByText("75%")).toBeNull()
	})

	it("updates screen position visually in ToastPositionPicker", () => {
		const onChange = vi.fn()
		render(<ToastPositionPicker value="bottom-right" onChange={onChange} />)

		expect(screen.getByRole("button", { name: "Bottom Right" }).getAttribute("aria-pressed")).toBe(
			"true",
		)

		const topCenterBtn = screen.getByRole("button", { name: "Top Center" })
		fireEvent.click(topCenterBtn)
		expect(onChange).toHaveBeenCalledWith("top-center")
	})

	it("renders Top and Bottom phone-frame slots on mobile while preserving horizontal alignment", () => {
		const previousWidth = window.innerWidth
		Object.defineProperty(window, "innerWidth", {
			configurable: true,
			writable: true,
			value: 400,
		})

		try {
			const onChange = vi.fn()
			render(<ToastPositionPicker value="bottom-right" onChange={onChange} />)

			expect(screen.getByRole("button", { name: "Bottom" }).getAttribute("aria-pressed")).toBe(
				"true",
			)
			expect(screen.queryByRole("button", { name: "Top Left" })).toBeNull()

			fireEvent.click(screen.getByRole("button", { name: "Top" }))
			expect(onChange).toHaveBeenCalledWith("top-right")
		} finally {
			Object.defineProperty(window, "innerWidth", {
				configurable: true,
				writable: true,
				value: previousWidth,
			})
		}
	})
})
