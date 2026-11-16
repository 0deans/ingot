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

export interface NavItem {
	path: AppRoutePath
	label: string
	shortLabel: string
	group: NavGroupId
	icon: LucideIcon
	badge?: string
	devOnly?: boolean
}

export const NAV_GROUP_LABELS: Record<NavGroupId, string> = {
	library: "Library",
	community: "Community",
	personal: "Personal",
	dev: "Developer",
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
		shortLabel: "Play",
		group: "library",
		icon: Gamepad2,
	},
	{
		path: "/modpacks",
		label: "Modpacks",
		shortLabel: "Packs",
		group: "library",
		icon: Package,
	},
	{
		path: "/news",
		label: "News & Events",
		shortLabel: "News",
		group: "community",
		icon: Newspaper,
		badge: "3",
	},
	{
		path: "/worlds",
		label: "Worlds",
		shortLabel: "Worlds",
		group: "personal",
		icon: Globe2,
	},
	{
		path: "/servers",
		label: "Servers",
		shortLabel: "Servers",
		group: "personal",
		icon: Server,
	},
	{
		path: "/skins",
		label: "Skins",
		shortLabel: "Skins",
		group: "personal",
		icon: Shirt,
	},
	{
		path: "/screenshots",
		label: "Screenshots",
		shortLabel: "Gallery",
		group: "personal",
		icon: Camera,
	},
	{
		path: "/settings",
		label: "Settings",
		shortLabel: "Settings",
		group: "personal",
		icon: Settings,
	},
	{
		path: "/ui-test",
		label: "UI Lab",
		shortLabel: "UI Lab",
		group: "dev",
		icon: FlaskConical,
		badge: "DEV",
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
