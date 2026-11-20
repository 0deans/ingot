import { act, fireEvent, render, screen } from "@testing-library/react"
import { afterEach, describe, expect, it, vi } from "vitest"
import ErrorBoundary from "@/shared/components/errors/ErrorBoundary"
import ErrorFallbackCard from "@/shared/components/errors/ErrorFallbackCard"
import RootErrorFallback from "@/shared/components/errors/RootErrorFallback"

interface BombProps {
	shouldThrow: () => boolean
}

const Bomb = ({ shouldThrow }: BombProps) => {
	if (shouldThrow()) {
		throw new ReferenceError("onToggleAccount is not defined")
	}
	return <div>Recovered view</div>
}

afterEach(() => {
	vi.restoreAllMocks()
})

describe("ErrorBoundary & recovery views", () => {
	it("catches render errors and recovers when reset or resetKey changes", () => {
		vi.spyOn(console, "error").mockImplementation(() => {})

		let shouldThrow = true
		const checkShouldThrow = () => {
			return shouldThrow
		}

		const { rerender } = render(
			<ErrorBoundary
				resetKey="/instances"
				fallback={({ error, reset }) => {
					return <ErrorFallbackCard error={error} scope="route" onRetry={reset} />
				}}
			>
				<Bomb shouldThrow={checkShouldThrow} />
			</ErrorBoundary>,
		)

		expect(screen.getByText("Something went wrong in this view")).toBeTruthy()
		expect(screen.getByText("ReferenceError: onToggleAccount is not defined")).toBeTruthy()

		shouldThrow = false
		fireEvent.click(screen.getByRole("button", { name: "Try Again" }))
		expect(screen.getByText("Recovered view")).toBeTruthy()

		shouldThrow = true
		rerender(
			<ErrorBoundary
				resetKey="/instances"
				fallback={({ error, reset }) => {
					return <ErrorFallbackCard error={error} scope="route" onRetry={reset} />
				}}
			>
				<Bomb shouldThrow={checkShouldThrow} />
			</ErrorBoundary>,
		)
		expect(screen.getByText("Something went wrong in this view")).toBeTruthy()

		shouldThrow = false
		rerender(
			<ErrorBoundary
				resetKey="/settings"
				fallback={({ error, reset }) => {
					return <ErrorFallbackCard error={error} scope="route" onRetry={reset} />
				}}
			>
				<Bomb shouldThrow={checkShouldThrow} />
			</ErrorBoundary>,
		)
		expect(screen.getByText("Recovered view")).toBeTruthy()
	})

	it("renders SafeTitlebar with window controls inside RootErrorFallback", () => {
		const error = new Error("onToggleAccount is not defined")
		const onRetry = vi.fn()
		const onGoHome = vi.fn()

		render(
			<RootErrorFallback error={error} reset={onRetry} routePath="/mods" onGoHome={onGoHome} />,
		)

		expect(screen.getByText("Ingot")).toBeTruthy()
		expect(screen.getByRole("button", { name: "Minimize window" })).toBeTruthy()
		expect(screen.getByRole("button", { name: "Maximize window" })).toBeTruthy()
		expect(screen.getByRole("button", { name: "Close window" })).toBeTruthy()
		expect(screen.getByText("The launcher interface encountered an error")).toBeTruthy()
		expect(screen.getByText("/mods")).toBeTruthy()

		fireEvent.click(screen.getByRole("button", { name: "Go to Play" }))
		expect(onGoHome).toHaveBeenCalledTimes(1)

		fireEvent.click(screen.getByRole("button", { name: "Try Again" }))
		expect(onRetry).toHaveBeenCalledTimes(1)
	})

	it("collapses technical details by default and copies diagnostics", async () => {
		const writeTextSpy = vi.fn().mockResolvedValue(undefined)
		Object.defineProperty(navigator, "clipboard", {
			configurable: true,
			value: { writeText: writeTextSpy },
		})

		const error = new Error("TauRPC invoke timeout")
		error.stack = "Error: TauRPC invoke timeout\n    at ModList.tsx:42:10"

		render(<ErrorFallbackCard error={error} scope="route" routePath="/mods" />)

		expect(
			screen.getByText(
				"An unexpected error occurred while communicating with the launcher backend.",
			),
		).toBeTruthy()
		expect(screen.queryByText(/Stack Trace:/)).toBeNull()

		fireEvent.click(screen.getByRole("button", { name: "Technical details" }))
		expect(screen.getByText(/at ModList.tsx:42:10/)).toBeTruthy()

		await act(async () => {
			fireEvent.click(screen.getByRole("button", { name: "Copy diagnostics" }))
			await Promise.resolve()
		})
		expect(writeTextSpy).toHaveBeenCalledTimes(1)
		const copiedPayload = String(writeTextSpy.mock.calls[0]?.[0] ?? "")
		expect(copiedPayload).toContain("Route: /mods")
		expect(copiedPayload).toContain("TauRPC invoke timeout")
	})
})
