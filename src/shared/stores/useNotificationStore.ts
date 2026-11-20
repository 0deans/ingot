import { z } from "zod"
import { create } from "zustand"
import { persist } from "zustand/middleware"
import { i18n } from "@/shared/i18n/i18n"

export type NotificationLevel = "info" | "success" | "warning" | "error"
export type NotificationCategory = "instance" | "server" | "download" | "auth" | "system"

export interface NotificationActionTarget {
	label: string
	route: string
}

export interface NotificationItem {
	id: string
	title: string
	message?: string
	level: NotificationLevel
	category: NotificationCategory
	createdAt: number
	read: boolean
	action?: NotificationActionTarget
}

export interface CreateNotificationInput {
	title: string
	message?: string
	level?: NotificationLevel
	category?: NotificationCategory
	action?: NotificationActionTarget
}

export const MAX_NOTIFICATIONS = 40

const notificationLevelSchema = z.enum(["info", "success", "warning", "error"])
const notificationCategorySchema = z.enum(["instance", "server", "download", "auth", "system"])
const notificationActionSchema = z.object({
	label: z.string().min(1),
	route: z.string().min(1),
})
const notificationItemSchema = z.object({
	id: z.string().min(1),
	title: z.string().min(1),
	message: z.string().optional(),
	level: notificationLevelSchema,
	category: notificationCategorySchema,
	createdAt: z.number().finite(),
	read: z.boolean(),
	action: notificationActionSchema.optional(),
})
const persistedNotificationsSchema = z.object({
	notifications: z.array(notificationItemSchema),
})

let notificationCounter = 0

const createNotificationId = (): string => {
	notificationCounter += 1
	return `notif-${Date.now()}-${notificationCounter}`
}

export const countUnreadNotifications = (items: readonly NotificationItem[]): number => {
	let unread = 0
	for (const item of items) {
		if (!item.read) {
			unread += 1
		}
	}
	return unread
}

export const hasUnreadByLevel = (
	items: readonly NotificationItem[],
	level: NotificationLevel,
): boolean => {
	return items.some((item) => {
		return !item.read && item.level === level
	})
}

export const formatNotificationTimeAgo = (
	timestampMs: number,
	nowMs: number = Date.now(),
): string => {
	const diffSeconds = Math.max(0, Math.floor((nowMs - timestampMs) / 1000))
	if (diffSeconds < 45) {
		return i18n.t("shell.notifications.timeAgo.justNow")
	}
	const diffMinutes = Math.floor(diffSeconds / 60)
	if (diffMinutes < 60) {
		return i18n.t("shell.notifications.timeAgo.minutesAgo", {
			count: diffMinutes,
		})
	}
	const diffHours = Math.floor(diffMinutes / 60)
	if (diffHours < 24) {
		return i18n.t("shell.notifications.timeAgo.hoursAgo", {
			count: diffHours,
		})
	}
	const diffDays = Math.floor(diffHours / 24)
	return i18n.t("shell.notifications.timeAgo.daysAgo", {
		count: diffDays,
	})
}

interface PersistedNotificationState {
	notifications: NotificationItem[]
}

interface NotificationStoreState extends PersistedNotificationState {
	pushNotification: (input: CreateNotificationInput) => string
	markRead: (id: string) => void
	markAllRead: () => void
	dismissNotification: (id: string) => void
	clearAll: () => void
}

export const selectNotifications = (state: NotificationStoreState): NotificationItem[] => {
	return state.notifications
}

export const useNotificationStore = create<NotificationStoreState>()(
	persist(
		(set) => {
			return {
				notifications: [],

				pushNotification: (input) => {
					const id = createNotificationId()
					const item: NotificationItem = {
						id,
						title: input.title,
						message: input.message,
						level: input.level ?? "info",
						category: input.category ?? "system",
						createdAt: Date.now(),
						read: false,
						action: input.action,
					}
					set((state) => {
						const next = [item, ...state.notifications].slice(0, MAX_NOTIFICATIONS)
						return { notifications: next }
					})
					return id
				},

				markRead: (id) => {
					set((state) => {
						const next = state.notifications.map((item) => {
							if (item.id !== id || item.read) {
								return item
							}
							return { ...item, read: true }
						})
						return { notifications: next }
					})
				},

				markAllRead: () => {
					set((state) => {
						const next = state.notifications.map((item) => {
							if (item.read) {
								return item
							}
							return { ...item, read: true }
						})
						return { notifications: next }
					})
				},

				dismissNotification: (id) => {
					set((state) => {
						const next = state.notifications.filter((item) => {
							return item.id !== id
						})
						return { notifications: next }
					})
				},

				clearAll: () => {
					set({ notifications: [] })
				},
			}
		},
		{
			name: "ingot-notifications",
			partialize: (state): PersistedNotificationState => {
				return { notifications: state.notifications }
			},
			merge: (persistedState, currentState) => {
				const parsed = persistedNotificationsSchema.safeParse(persistedState)
				if (!parsed.success) {
					return currentState
				}
				return {
					...currentState,
					notifications: parsed.data.notifications.slice(0, MAX_NOTIFICATIONS),
				}
			},
		},
	),
)
