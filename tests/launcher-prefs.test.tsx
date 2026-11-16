import { fireEvent, render, screen } from "@testing-library/react"
import { beforeEach, describe, expect, it } from "vitest"
import NotificationSettingsCard from "@/features/settings/components/NotificationSettingsCard"
import { getSidebarNavItems, getVisibleNavItems } from "@/features/shell/navigation"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"

beforeEach(() => {
	useLauncherPrefsStore.setState({
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
})
