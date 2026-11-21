import { Compass, FlaskConical, Library, User, type LucideIcon } from "lucide-react"
import {
	getVisibleNavItems,
	isNavItemActive,
	NAV_GROUP_ORDER,
	type NavGroupId,
	type NavItem,
} from "@/features/shell/navigation"

export interface NavGroupSection {
	id: NavGroupId
	items: NavItem[]
	hasBadge: boolean
	isActive: boolean
	panelWidth: number
}

export interface PopupAnchor {
	top: number
	left: number
}

export const SINGLE_COLUMN_WIDTH = 224
export const MULTI_COLUMN_WIDTH = 352
export const VIEWPORT_EDGE_PADDING = 12
export const CLOSE_DELAY_MS = 180
export const PANEL_SPRING_EASE = [0.22, 1, 0.36, 1] as const

export const GROUP_ICONS: Record<NavGroupId, LucideIcon> = {
	library: Library,
	community: Compass,
	personal: User,
	dev: FlaskConical,
}

export const buildNavGroupSections = (
	currentPath: string,
	showDeveloperPages: boolean,
): NavGroupSection[] => {
	const visibleItems = getVisibleNavItems(showDeveloperPages)
	const sections: NavGroupSection[] = []

	for (const groupId of NAV_GROUP_ORDER) {
		const groupItems = visibleItems.filter((item) => {
			return item.group === groupId
		})
		if (groupItems.length === 0) {
			continue
		}

		const hasBadge = groupItems.some((item) => {
			return Boolean(item.badge || item.badgeKey)
		})
		const isActive = groupItems.some((item) => {
			return isNavItemActive(currentPath, item.path)
		})
		const panelWidth = groupItems.length > 2 ? MULTI_COLUMN_WIDTH : SINGLE_COLUMN_WIDTH

		sections.push({
			id: groupId,
			items: groupItems,
			hasBadge,
			isActive,
			panelWidth,
		})
	}

	return sections
}

export const computePopupAnchor = (
	navRect: DOMRect,
	triggerRect: DOMRect,
	panelWidth: number,
	viewportWidth: number,
): PopupAnchor => {
	const rawLeft = triggerRect.left + triggerRect.width / 2 - panelWidth / 2
	const maxLeft = Math.max(
		VIEWPORT_EDGE_PADDING,
		viewportWidth - panelWidth - VIEWPORT_EDGE_PADDING,
	)
	const clampedLeft = Math.min(Math.max(rawLeft, VIEWPORT_EDGE_PADDING), maxLeft)

	return {
		top: navRect.bottom,
		left: clampedLeft,
	}
}

export const resolveSlideDirection = (
	groups: readonly NavGroupSection[],
	prevGroupId: NavGroupId | null,
	nextGroupId: NavGroupId,
): number | null => {
	if (prevGroupId === null || prevGroupId === nextGroupId) {
		return null
	}

	const prevIndex = groups.findIndex((group) => {
		return group.id === prevGroupId
	})
	const nextIndex = groups.findIndex((group) => {
		return group.id === nextGroupId
	})
	if (prevIndex === -1 || nextIndex === -1) {
		return null
	}

	return nextIndex > prevIndex ? 1 : -1
}
