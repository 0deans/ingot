import { create } from "zustand"

export interface AppUpdateInfo {
	version: string
	notes: string
	changelogUrl?: string
}

export const getCurrentAppVersion = (): string => {
	if (typeof APP_VERSION === "string" && APP_VERSION.length > 0) {
		return APP_VERSION
	}
	return "0.5.17"
}

interface AppUpdateStoreState {
	availableUpdate: AppUpdateInfo | null
	dismissed: boolean
	isInstalling: boolean
	setAvailableUpdate: (update: AppUpdateInfo | null) => void
	dismissUpdate: () => void
	simulateMockUpdate: () => void
	clearUpdate: () => void
	startInstall: () => void
}

export const useAppUpdateStore = create<AppUpdateStoreState>((set) => {
	return {
		availableUpdate: null,
		dismissed: false,
		isInstalling: false,

		setAvailableUpdate: (update) => {
			set({
				availableUpdate: update,
				dismissed: false,
				isInstalling: false,
			})
		},

		dismissUpdate: () => {
			set({ dismissed: true })
		},

		simulateMockUpdate: () => {
			set({
				availableUpdate: {
					version: "0.5.18",
					notes: "Improved shell animations, unified UI controls, and general stability fixes.",
					changelogUrl: "https://github.com/0deans/ingot/releases",
				},
				dismissed: false,
				isInstalling: false,
			})
		},

		clearUpdate: () => {
			set({
				availableUpdate: null,
				dismissed: false,
				isInstalling: false,
			})
		},

		startInstall: () => {
			set({ isInstalling: true })
		},
	}
})
