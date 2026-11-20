import { create } from "zustand"
import { type PopoverAnchorRect } from "@/features/shell/components/popovers/AccountSwitcherPopover"

export type AnchoredOverlayKind = "processes" | "notifications" | "account" | "update"

interface ClosedOverlayState {
	kind: "none"
}

interface AnchoredOverlayState {
	kind: AnchoredOverlayKind
	layoutKey: string
}

interface MobileMoreOverlayState {
	kind: "mobileMore"
}

export type ActiveShellOverlay = ClosedOverlayState | AnchoredOverlayState | MobileMoreOverlayState

const CLOSED_OVERLAY: ClosedOverlayState = { kind: "none" }

const extractAnchorRect = (element: HTMLElement): PopoverAnchorRect => {
	const rect = element.getBoundingClientRect()
	return {
		top: rect.top,
		bottom: rect.bottom,
		left: rect.left,
		right: rect.right,
	}
}

export const resolveNextOverlayKind = (
	current: ActiveShellOverlay,
	kind: AnchoredOverlayKind,
	layoutKey: string,
): ActiveShellOverlay => {
	if (current.kind === kind && current.layoutKey === layoutKey) {
		return CLOSED_OVERLAY
	}
	return { kind, layoutKey }
}

export const isAnchoredOverlayOpen = (
	active: ActiveShellOverlay,
	kind: AnchoredOverlayKind,
	layoutKey: string,
): boolean => {
	return active.kind === kind && active.layoutKey === layoutKey
}

interface ShellOverlayStoreState {
	activeOverlay: ActiveShellOverlay
	anchorsByKind: Partial<Record<AnchoredOverlayKind, PopoverAnchorRect>>
	toggleAnchored: (kind: AnchoredOverlayKind, layoutKey: string, trigger: HTMLElement) => void
	toggleMore: () => void
	closeAll: () => void
}

export const useShellOverlayStore = create<ShellOverlayStoreState>((set) => {
	return {
		activeOverlay: CLOSED_OVERLAY,
		anchorsByKind: {},

		toggleAnchored: (kind, layoutKey, trigger) => {
			const rect = extractAnchorRect(trigger)
			set((state) => {
				return {
					anchorsByKind: { ...state.anchorsByKind, [kind]: rect },
					activeOverlay: resolveNextOverlayKind(state.activeOverlay, kind, layoutKey),
				}
			})
		},

		toggleMore: () => {
			set((state) => {
				if (state.activeOverlay.kind === "mobileMore") {
					return { activeOverlay: CLOSED_OVERLAY }
				}
				return { activeOverlay: { kind: "mobileMore" } }
			})
		},

		closeAll: () => {
			set({ activeOverlay: CLOSED_OVERLAY })
		},
	}
})
