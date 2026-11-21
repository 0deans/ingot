import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import {
	CLOSE_DELAY_MS,
	computePopupAnchor,
	resolveSlideDirection,
	SINGLE_COLUMN_WIDTH,
	type NavGroupSection,
	type PopupAnchor,
} from "@/features/shell/components/desktop/capsule-nav/capsuleNavModel"
import { type NavGroupId } from "@/features/shell/navigation"

export interface CapsuleNavMenuState {
	openGroup: NavGroupId | null
	activeGroupSection: NavGroupSection | null
	slideDirection: number
	anchor: PopupAnchor | null
	panelHeight: number | null
}

export interface CapsuleNavMenuActions {
	openGroupMenu: (nextGroupId: NavGroupId) => void
	toggleGroupMenu: (groupId: NavGroupId) => void
	closeImmediately: () => void
	scheduleClose: () => void
	cancelCloseTimer: () => void
}

export interface CapsuleNavMenuRegistry {
	nav: (node: HTMLElement | null) => void
	popup: (node: HTMLDivElement | null) => void
	trigger: (groupId: NavGroupId, node: HTMLButtonElement | null) => void
	content: (groupId: NavGroupId, node: HTMLDivElement | null) => void
}

export interface UseCapsuleNavMenuResult {
	state: CapsuleNavMenuState
	actions: CapsuleNavMenuActions
	registry: CapsuleNavMenuRegistry
}

export const useCapsuleNavMenu = (groups: readonly NavGroupSection[]): UseCapsuleNavMenuResult => {
	const navRef = useRef<HTMLElement | null>(null)
	const popupRef = useRef<HTMLDivElement | null>(null)
	const triggerNodesRef = useRef<Partial<Record<NavGroupId, HTMLButtonElement | null>>>({})
	const contentNodesRef = useRef<Partial<Record<NavGroupId, HTMLDivElement | null>>>({})
	const openGroupRef = useRef<NavGroupId | null>(null)
	const closeTimerRef = useRef<number | null>(null)

	const [openGroup, setOpenGroup] = useState<NavGroupId | null>(null)
	const [slideDirection, setSlideDirection] = useState<number>(1)
	const [anchor, setAnchor] = useState<PopupAnchor | null>(null)
	const [panelHeight, setPanelHeight] = useState<number | null>(null)

	const activeGroupSection = useMemo<NavGroupSection | null>(() => {
		if (openGroup === null) {
			return null
		}
		const found = groups.find((group) => {
			return group.id === openGroup
		})
		return found ?? null
	}, [groups, openGroup])

	const cancelCloseTimer = useCallback(() => {
		if (closeTimerRef.current !== null) {
			window.clearTimeout(closeTimerRef.current)
			closeTimerRef.current = null
		}
	}, [])

	const closeImmediately = useCallback(() => {
		cancelCloseTimer()
		openGroupRef.current = null
		setOpenGroup(null)
		setPanelHeight(null)
	}, [cancelCloseTimer])

	const scheduleClose = useCallback(() => {
		cancelCloseTimer()
		closeTimerRef.current = window.setTimeout(() => {
			openGroupRef.current = null
			setOpenGroup(null)
			setPanelHeight(null)
			closeTimerRef.current = null
		}, CLOSE_DELAY_MS)
	}, [cancelCloseTimer])

	const registerNavNode = useCallback((node: HTMLElement | null) => {
		navRef.current = node
	}, [])

	const registerPopupNode = useCallback((node: HTMLDivElement | null) => {
		popupRef.current = node
	}, [])

	const registerTriggerNode = useCallback((groupId: NavGroupId, node: HTMLButtonElement | null) => {
		triggerNodesRef.current[groupId] = node
	}, [])

	const registerContentNode = useCallback((groupId: NavGroupId, node: HTMLDivElement | null) => {
		contentNodesRef.current[groupId] = node
		if (node && openGroupRef.current === groupId) {
			setPanelHeight(node.offsetHeight)
		}
	}, [])

	const measureAnchorForGroup = useCallback(
		(groupId: NavGroupId): PopupAnchor | null => {
			const navEl = navRef.current
			const triggerEl = triggerNodesRef.current[groupId]
			if (!navEl || !triggerEl) {
				return null
			}

			const foundGroup = groups.find((group) => {
				return group.id === groupId
			})
			const panelWidth = foundGroup ? foundGroup.panelWidth : SINGLE_COLUMN_WIDTH

			return computePopupAnchor(
				navEl.getBoundingClientRect(),
				triggerEl.getBoundingClientRect(),
				panelWidth,
				window.innerWidth,
			)
		},
		[groups],
	)

	const openGroupMenu = useCallback(
		(nextGroupId: NavGroupId) => {
			cancelCloseTimer()
			openGroupRef.current = nextGroupId

			const nextAnchor = measureAnchorForGroup(nextGroupId)
			if (nextAnchor) {
				setAnchor(nextAnchor)
			}

			const existingContentNode = contentNodesRef.current[nextGroupId]
			if (existingContentNode) {
				setPanelHeight(existingContentNode.offsetHeight)
			}

			setOpenGroup((prevGroupId) => {
				const nextDirection = resolveSlideDirection(groups, prevGroupId, nextGroupId)
				if (nextDirection !== null) {
					setSlideDirection(nextDirection)
				}
				return nextGroupId
			})
		},
		[cancelCloseTimer, groups, measureAnchorForGroup],
	)

	const toggleGroupMenu = useCallback(
		(groupId: NavGroupId) => {
			if (openGroupRef.current === groupId) {
				closeImmediately()
				return
			}
			openGroupMenu(groupId)
		},
		[closeImmediately, openGroupMenu],
	)

	const actions = useMemo<CapsuleNavMenuActions>(() => {
		return {
			openGroupMenu,
			toggleGroupMenu,
			closeImmediately,
			scheduleClose,
			cancelCloseTimer,
		}
	}, [cancelCloseTimer, closeImmediately, openGroupMenu, scheduleClose, toggleGroupMenu])

	const registry = useMemo<CapsuleNavMenuRegistry>(() => {
		return {
			nav: registerNavNode,
			popup: registerPopupNode,
			trigger: registerTriggerNode,
			content: registerContentNode,
		}
	}, [registerContentNode, registerNavNode, registerPopupNode, registerTriggerNode])

	useEffect(() => {
		return () => {
			cancelCloseTimer()
		}
	}, [cancelCloseTimer])

	useEffect(() => {
		if (openGroup === null) {
			return
		}

		const handleKeyDown = (event: KeyboardEvent) => {
			if (event.key === "Escape") {
				closeImmediately()
			}
		}

		const handlePointerDown = (event: PointerEvent) => {
			const target = event.target
			if (!(target instanceof Node)) {
				return
			}
			if (navRef.current?.contains(target) || popupRef.current?.contains(target)) {
				return
			}
			closeImmediately()
		}

		const handleResize = () => {
			const nextAnchor = measureAnchorForGroup(openGroup)
			if (nextAnchor) {
				setAnchor(nextAnchor)
			}
		}

		window.addEventListener("keydown", handleKeyDown)
		window.addEventListener("pointerdown", handlePointerDown)
		window.addEventListener("resize", handleResize)

		return () => {
			window.removeEventListener("keydown", handleKeyDown)
			window.removeEventListener("pointerdown", handlePointerDown)
			window.removeEventListener("resize", handleResize)
		}
	}, [closeImmediately, measureAnchorForGroup, openGroup])

	return {
		state: {
			openGroup,
			activeGroupSection,
			slideDirection,
			anchor,
			panelHeight,
		},
		actions,
		registry,
	}
}
