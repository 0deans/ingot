import {
	Camera,
	FlaskConical,
	Gamepad2,
	Globe2,
	Newspaper,
	Package,
	Server,
	Settings,
	Shirt,
	type LucideIcon,
} from "lucide-react"

export type AppRoutePath =
	| "/"
	| "/modpacks"
	| "/news"
	| "/worlds"
	| "/servers"
	| "/skins"
	| "/screenshots"
	| "/settings"
	| "/ui-test"

export type NavGroupId = "library" | "community" | "personal" | "dev"

export type NavLabelKey =
	| "nav.items.play.label"
	| "nav.items.modpacks.label"
	| "nav.items.news.label"
	| "nav.items.worlds.label"
	| "nav.items.servers.label"
	| "nav.items.skins.label"
	| "nav.items.screenshots.label"
	| "nav.items.settings.label"
	| "nav.items.uiTest.label"

export type NavShortLabelKey =
	| "nav.items.play.shortLabel"
	| "nav.items.modpacks.shortLabel"
	| "nav.items.news.shortLabel"
	| "nav.items.worlds.shortLabel"
	| "nav.items.servers.shortLabel"
	| "nav.items.skins.shortLabel"
	| "nav.items.screenshots.shortLabel"
	| "nav.items.settings.shortLabel"
	| "nav.items.uiTest.shortLabel"

export type NavGroupLabelKey =
	| "nav.groups.library"
	| "nav.groups.community"
	| "nav.groups.personal"
	| "nav.groups.dev"

export type NavBadgeKey = "nav.badges.dev"

export interface NavItem {
	path: AppRoutePath
	label: string
	labelKey: NavLabelKey
	shortLabel: string
	shortLabelKey: NavShortLabelKey
	group: NavGroupId
	icon: LucideIcon
	badge?: string
	badgeKey?: NavBadgeKey
	devOnly?: boolean
}

export const NAV_GROUP_LABELS: Record<NavGroupId, string> = {
	library: "Library",
	community: "Community",
	personal: "Personal",
	dev: "Developer",
}

export const NAV_GROUP_LABEL_KEYS: Record<NavGroupId, NavGroupLabelKey> = {
	library: "nav.groups.library",
	community: "nav.groups.community",
	personal: "nav.groups.personal",
	dev: "nav.groups.dev",
}

export const NAV_GROUP_ORDER: readonly NavGroupId[] = ["library", "community", "personal", "dev"]

export const PRIMARY_MOBILE_PATHS: readonly AppRoutePath[] = [
	"/",
	"/modpacks",
	"/worlds",
	"/servers",
]

export const NAV_ITEMS: readonly NavItem[] = [
	{
		path: "/",
		label: "Play",
		labelKey: "nav.items.play.label",
		shortLabel: "Play",
		shortLabelKey: "nav.items.play.shortLabel",
		group: "library",
		icon: Gamepad2,
	},
	{
		path: "/modpacks",
		label: "Modpacks",
		labelKey: "nav.items.modpacks.label",
		shortLabel: "Packs",
		shortLabelKey: "nav.items.modpacks.shortLabel",
		group: "library",
		icon: Package,
	},
	{
		path: "/news",
		label: "News & Events",
		labelKey: "nav.items.news.label",
		shortLabel: "News",
		shortLabelKey: "nav.items.news.shortLabel",
		group: "community",
		icon: Newspaper,
		badge: "3",
	},
	{
		path: "/worlds",
		label: "Worlds",
		labelKey: "nav.items.worlds.label",
		shortLabel: "Worlds",
		shortLabelKey: "nav.items.worlds.shortLabel",
		group: "personal",
		icon: Globe2,
	},
	{
		path: "/servers",
		label: "Servers",
		labelKey: "nav.items.servers.label",
		shortLabel: "Servers",
		shortLabelKey: "nav.items.servers.shortLabel",
		group: "personal",
		icon: Server,
	},
	{
		path: "/skins",
		label: "Skins",
		labelKey: "nav.items.skins.label",
		shortLabel: "Skins",
		shortLabelKey: "nav.items.skins.shortLabel",
		group: "personal",
		icon: Shirt,
	},
	{
		path: "/screenshots",
		label: "Screenshots",
		labelKey: "nav.items.screenshots.label",
		shortLabel: "Gallery",
		shortLabelKey: "nav.items.screenshots.shortLabel",
		group: "personal",
		icon: Camera,
	},
	{
		path: "/settings",
		label: "Settings",
		labelKey: "nav.items.settings.label",
		shortLabel: "Settings",
		shortLabelKey: "nav.items.settings.shortLabel",
		group: "personal",
		icon: Settings,
	},
	{
		path: "/ui-test",
		label: "UI Lab",
		labelKey: "nav.items.uiTest.label",
		shortLabel: "UI Lab",
		shortLabelKey: "nav.items.uiTest.shortLabel",
		group: "dev",
		icon: FlaskConical,
		badge: "DEV",
		badgeKey: "nav.badges.dev",
		devOnly: true,
	},
]

export const getVisibleNavItems = (isDev: boolean): NavItem[] => {
	return NAV_ITEMS.filter((item) => {
		return isDev || !item.devOnly
	})
}

export const getSidebarNavItems = (isDev: boolean): NavItem[] => {
	return getVisibleNavItems(isDev).filter((item) => {
		return item.path !== "/settings"
	})
}

export const isNavItemActive = (currentPath: string, itemPath: AppRoutePath): boolean => {
	if (itemPath === "/") {
		return currentPath === "/" || currentPath.startsWith("/instances")
	}
	return currentPath === itemPath || currentPath.startsWith(`${itemPath}/`)
}

export const findNavItem = (pathname: string): NavItem => {
	const exact = NAV_ITEMS.find((item) => {
		return isNavItemActive(pathname, item.path)
	})
	return exact ?? NAV_ITEMS[0]
}

export const getPageTransitionKey = (pathname: string): string => {
	const normalized =
		pathname.length > 1 && pathname.endsWith("/") ? pathname.slice(0, -1) : pathname

	if (normalized === "/settings" || normalized.startsWith("/settings/")) {
		return "/settings"
	}

	if (normalized.startsWith("/instances/")) {
		const segments = normalized.split("/")
		const instanceId = segments[2]
		if (instanceId) {
			return `/instances/${instanceId}`
		}
	}

	return normalized || "/"
}
