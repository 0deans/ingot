import { act, fireEvent, render, screen } from "@testing-library/react"
import { toast } from "sonner"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import NotificationBellButton from "@/features/shell/components/NotificationBellButton"
import NotificationsPopover from "@/features/shell/components/NotificationsPopover"
import { notifyBackgroundEvent, registerNotificationNavigator } from "@/shared/lib/notify"
import {
	MAX_NOTIFICATIONS,
	countUnreadNotifications,
	hasUnreadByLevel,
	useNotificationStore,
} from "@/shared/stores/useNotificationStore"

beforeEach(() => {
	useNotificationStore.setState({ notifications: [] })
})

afterEach(() => {
	registerNotificationNavigator(null)
	act(() => {
		toast.dismiss()
	})
})

describe("Persistent notification store & bell center", () => {
	it("records capped notifications and tracks unread status cleanly", () => {
		const { pushNotification, markRead, markAllRead, clearAll } = useNotificationStore.getState()

		for (let i = 0; i < MAX_NOTIFICATIONS + 5; i += 1) {
			pushNotification({
				title: `Alert ${i}`,
				level: i === MAX_NOTIFICATIONS + 4 ? "error" : "info",
				category: "instance",
			})
		}

		const items = useNotificationStore.getState().notifications
		expect(items).toHaveLength(MAX_NOTIFICATIONS)
		expect(countUnreadNotifications(items)).toBe(MAX_NOTIFICATIONS)
		expect(hasUnreadByLevel(items, "error")).toBe(true)

		const newestId = items[0]?.id
		expect(newestId).toBeTruthy()
		if (newestId) {
			markRead(newestId)
		}
		expect(countUnreadNotifications(useNotificationStore.getState().notifications)).toBe(
			MAX_NOTIFICATIONS - 1,
		)

		markAllRead()
		expect(countUnreadNotifications(useNotificationStore.getState().notifications)).toBe(0)

		clearAll()
		expect(useNotificationStore.getState().notifications).toHaveLength(0)
	})

	it("pushes persistent background events and skips transient ones", () => {
		const navigateSpy = vi.fn()
		registerNotificationNavigator(navigateSpy)

		const skippedId = notifyBackgroundEvent({
			title: "Copied theme JSON",
			level: "success",
			persist: false,
		})
		expect(skippedId).toBeUndefined()
		expect(useNotificationStore.getState().notifications).toHaveLength(0)

		const persistedId = notifyBackgroundEvent({
			title: "1.21.4 Fabric crashed (exit code 1)",
			message: "Inspect logs for details.",
			level: "error",
			category: "instance",
			action: { label: "View Logs", route: "/instances/inst-1/logs" },
		})

		expect(persistedId).toBeTruthy()
		const stored = useNotificationStore.getState().notifications
		expect(stored).toHaveLength(1)
		expect(stored[0]?.action?.route).toBe("/instances/inst-1/logs")
	})

	it("renders bell badge counts and popover filtering/actions", () => {
		const onToggle = vi.fn()
		render(
			<NotificationBellButton
				open={false}
				unreadCount={12}
				hasUnreadError={true}
				hasUnreadWarning={false}
				onClick={onToggle}
			/>,
		)

		expect(screen.getByRole("button", { name: "Notifications (12 unread)" })).toBeTruthy()
		expect(screen.getByText("9+")).toBeTruthy()

		const onNavigateAction = vi.fn()
		const onMarkAllRead = vi.fn()
		render(
			<NotificationsPopover
				open={true}
				anchorRect={{ top: 0, bottom: 36, left: 900, right: 928 }}
				notifications={[
					{
						id: "n-1",
						title: "Paper Server failed on :25565",
						message: "Port already in use",
						level: "error",
						category: "server",
						createdAt: Date.now(),
						read: false,
						action: { label: "Open Servers", route: "/servers" },
					},
					{
						id: "n-2",
						title: "Pack installed",
						level: "success",
						category: "download",
						createdAt: Date.now() - 120_000,
						read: true,
					},
				]}
				onClose={vi.fn()}
				onMarkRead={vi.fn()}
				onMarkAllRead={onMarkAllRead}
				onDismiss={vi.fn()}
				onClearAll={vi.fn()}
				onNavigateAction={onNavigateAction}
			/>,
		)

		fireEvent.click(screen.getByRole("button", { name: "Unread (1)" }))
		expect(screen.queryByText("Pack installed")).toBeNull()
		expect(screen.getByText("Paper Server failed on :25565")).toBeTruthy()

		fireEvent.click(screen.getByRole("button", { name: "Open Servers" }))
		expect(onNavigateAction).toHaveBeenCalledWith("n-1", "/servers")

		fireEvent.click(screen.getByRole("button", { name: "Read all" }))
		expect(onMarkAllRead).toHaveBeenCalledTimes(1)
	})
})
