import { useSyncExternalStore } from "react"
import { toast, type ExternalToast } from "sonner"

export interface ToastProgressState {
	value: number
	detail?: string
	indeterminate?: boolean
}

export interface ProgressToastOptions extends Omit<ExternalToast, "duration"> {
	value: number
	detail?: string
	indeterminate?: boolean
	duration?: number
}

export interface ProgressUpdateOptions {
	value: number
	title?: string
	description?: string
	detail?: string
	indeterminate?: boolean
}

type StoreListener = () => void

const progressEntries = new Map<string, ToastProgressState>()
const listeners = new Set<StoreListener>()

const notifyListeners = (): void => {
	for (const listener of listeners) {
		listener()
	}
}

const subscribe = (listener: StoreListener): (() => void) => {
	listeners.add(listener)
	return () => {
		listeners.delete(listener)
	}
}

const getServerProgressSnapshot = (): undefined => {
	return undefined
}

export const clampProgressValue = (value: number): number => {
	if (!Number.isFinite(value)) {
		return 0
	}
	return Math.min(100, Math.max(0, value))
}

export const setToastProgress = (id: string | number, next: ToastProgressState): void => {
	const key = String(id)
	progressEntries.set(key, {
		value: clampProgressValue(next.value),
		detail: next.detail,
		indeterminate: next.indeterminate,
	})
	notifyListeners()
}

export const clearToastProgress = (id: string | number): void => {
	const key = String(id)
	if (!progressEntries.has(key)) {
		return
	}
	progressEntries.delete(key)
	notifyListeners()
}

export const useToastProgress = (id: string | number): ToastProgressState | undefined => {
	const key = String(id)
	const getSnapshot = () => {
		return progressEntries.get(key)
	}
	return useSyncExternalStore(subscribe, getSnapshot, getServerProgressSnapshot)
}

const dispatchDefaultToast: typeof toast = Object.assign(
	(message: Parameters<typeof toast>[0], data?: ExternalToast) => {
		return toast(message, data)
	},
	toast,
)

export const appleToast = Object.assign(dispatchDefaultToast, {
	progress: (title: string, options: ProgressToastOptions): string | number => {
		const { value, detail, indeterminate, duration, ...toastOptions } = options
		const id = toast.loading(title, {
			...toastOptions,
			duration: duration ?? Number.POSITIVE_INFINITY,
		})
		setToastProgress(id, { value, detail, indeterminate })
		return id
	},

	updateProgress: (id: string | number, next: ProgressUpdateOptions): void => {
		setToastProgress(id, {
			value: next.value,
			detail: next.detail,
			indeterminate: next.indeterminate,
		})
		if (next.title !== undefined || next.description !== undefined) {
			toast.loading(next.title ?? "Working...", {
				id,
				description: next.description,
				duration: Number.POSITIVE_INFINITY,
			})
		}
	},

	completeProgress: (
		id: string | number,
		title: string,
		options?: ExternalToast,
	): string | number => {
		clearToastProgress(id)
		return toast.success(title, {
			...options,
			id,
		})
	},

	failProgress: (id: string | number, title: string, options?: ExternalToast): string | number => {
		clearToastProgress(id)
		return toast.error(title, {
			...options,
			id,
		})
	},
})
