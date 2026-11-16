import { type ExternalToast } from "sonner"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"
import {
	useNotificationStore,
	type CreateNotificationInput,
	type NotificationActionTarget,
	type NotificationLevel,
} from "@/shared/stores/useNotificationStore"
import { appleToast } from "@/shared/ui/sonner"

export interface BackgroundNotificationInput extends CreateNotificationInput {
	persist?: boolean
}

type NotificationNavigator = (route: string) => void

let registeredNavigator: NotificationNavigator | null = null

export const registerNotificationNavigator = (navigator: NotificationNavigator | null): void => {
	registeredNavigator = navigator
}

export const navigateFromNotification = (route: string): void => {
	if (registeredNavigator) {
		registeredNavigator(route)
	}
}

const buildToastAction = (
	action: NotificationActionTarget | undefined,
	notificationId: string | undefined,
): ExternalToast["action"] => {
	if (!action) {
		return undefined
	}

	return {
		label: action.label,
		onClick: () => {
			if (notificationId) {
				useNotificationStore.getState().markRead(notificationId)
			}
			navigateFromNotification(action.route)
		},
	}
}

const dispatchToastByLevel = (
	level: NotificationLevel,
	title: string,
	description: string | undefined,
	position: ReturnType<typeof useLauncherPrefsStore.getState>["toastPosition"],
	action: CreateNotificationInput["action"],
	notificationId: string | undefined,
): void => {
	const toastAction = buildToastAction(action, notificationId)

	const options = {
		description,
		position,
		action: toastAction,
	}

	if (level === "success") {
		appleToast.success(title, options)
		return
	}
	if (level === "warning") {
		appleToast.warning(title, options)
		return
	}
	if (level === "error") {
		appleToast.error(title, options)
		return
	}
	appleToast.info(title, options)
}

const persistNotificationIfEnabled = (
	input: BackgroundNotificationInput,
	level: NotificationLevel,
): string | undefined => {
	if (input.persist === false) {
		return undefined
	}

	return useNotificationStore.getState().pushNotification({
		title: input.title,
		message: input.message,
		level,
		category: input.category,
		action: input.action,
	})
}

/**
 * Emits a transient floating toast and records persistent alerts into the
 * titlebar notification inbox when `persist !== false`.
 */
export const notifyBackgroundEvent = (input: BackgroundNotificationInput): string | undefined => {
	const level: NotificationLevel = input.level ?? "info"
	const toastPosition = useLauncherPrefsStore.getState().toastPosition
	const notificationId = persistNotificationIfEnabled(input, level)

	dispatchToastByLevel(
		level,
		input.title,
		input.message,
		toastPosition,
		input.action,
		notificationId,
	)
	return notificationId
}
