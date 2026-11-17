import { act, fireEvent, render, screen } from "@testing-library/react"
import { type ReactNode } from "react"
import { beforeEach, describe, expect, it, vi } from "vitest"
import AppearanceSettingsPage from "@/features/settings/AppearanceSettingsPage"
import NotificationSettingsCard from "@/features/settings/components/NotificationSettingsCard"
import { getSidebarNavItems, getVisibleNavItems } from "@/features/shell/navigation"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"

interface MockLinkProps {
	to: string
	children?: ReactNode
	className?: string
}

vi.mock("@tanstack/react-router", () => {
	return {
		Link: ({ to, children, className }: MockLinkProps) => {
			return (
				<a href={to} className={className}>
					{children}
				</a>
			)
		},
	}
})

beforeEach(() => {
	useLauncherPrefsStore.setState({
		navVariant: "sonoma",
		toastPosition: "bottom-right",
		showDeveloperPages: true,
		enableDevContextMenu: true,
	})
})

describe("Launcher toast & developer preferences", () => {
	it("updates and validates preferred toastPosition in the store", () => {
		const { setToastPosition } = useLauncherPrefsStore.getState()

		setToastPosition("top-center")
		expect(useLauncherPrefsStore.getState().toastPosition).toBe("top-center")

		setToastPosition("top-left")
		expect(useLauncherPrefsStore.getState().toastPosition).toBe("top-left")
	})

	it("toggles showDeveloperPages and enableDevContextMenu cleanly", () => {
		const { setShowDeveloperPages, setEnableDevContextMenu } = useLauncherPrefsStore.getState()

		setShowDeveloperPages(false)
		expect(useLauncherPrefsStore.getState().showDeveloperPages).toBe(false)
		expect(
			getVisibleNavItems(useLauncherPrefsStore.getState().showDeveloperPages).some((item) => {
				return item.path === "/ui-test"
			}),
		).toBe(false)
		expect(
			getSidebarNavItems(useLauncherPrefsStore.getState().showDeveloperPages).some((item) => {
				return item.path === "/ui-test"
			}),
		).toBe(false)

		setShowDeveloperPages(true)
		expect(
			getVisibleNavItems(useLauncherPrefsStore.getState().showDeveloperPages).some((item) => {
				return item.path === "/ui-test"
			}),
		).toBe(true)

		setEnableDevContextMenu(false)
		expect(useLauncherPrefsStore.getState().enableDevContextMenu).toBe(false)
	})

	it("updates persisted toastPosition when clicking a corner in NotificationSettingsCard", () => {
		render(<NotificationSettingsCard />)

		fireEvent.click(screen.getByRole("button", { name: "Top Right" }))

		expect(useLauncherPrefsStore.getState().toastPosition).toBe("top-right")
		expect(screen.getByRole("button", { name: "Top Right" }).getAttribute("aria-pressed")).toBe(
			"true",
		)
	})

	it("shows a tablet/desktop-only notice on Navigation Layout on mobile while keeping preference selection working", () => {
		Object.defineProperty(window, "innerWidth", {
			configurable: true,
			writable: true,
			value: 420,
		})

		render(<AppearanceSettingsPage />)

		expect(screen.getByText(/Tablet & Desktop only/i)).toBeTruthy()

		fireEvent.click(screen.getByRole("button", { name: /Top Capsule/i }))
		expect(useLauncherPrefsStore.getState().navVariant).toBe("capsule")

		act(() => {
			window.innerWidth = 1024
			window.dispatchEvent(new Event("resize"))
		})

		expect(screen.queryByText(/Tablet & Desktop only/i)).toBeNull()
	})
})
