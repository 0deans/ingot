import { Outlet, useRouterState } from "@tanstack/react-router"
import RouteNavTabs, { type RouteNavTabItem } from "@/shared/components/motion/RouteNavTabs"

interface SettingsTabDef {
	to: "/settings" | "/settings/java" | "/settings/accounts" | "/settings/network"
	label: string
}

const SETTINGS_TABS: readonly SettingsTabDef[] = [
	{ to: "/settings", label: "Appearance" },
	{ to: "/settings/java", label: "Java & Memory" },
	{ to: "/settings/accounts", label: "Accounts" },
	{ to: "/settings/network", label: "Network" },
]

const SettingsLayout = () => {
	const pathname = useRouterState({
		select: (state) => {
			return state.location.pathname
		},
	})

	const normalizedPath = pathname.replace(/\/+$/, "") || "/"
	const tabItems: RouteNavTabItem[] = SETTINGS_TABS.map((item) => {
		return {
			key: item.to,
			label: item.label,
			isActive: normalizedPath === item.to,
			link: { to: item.to },
		}
	})

	return (
		<div className="flex flex-col gap-5">
			<div className="flex flex-wrap items-center justify-between gap-3">
				<div>
					<h1 className="text-lg font-semibold tracking-tight text-foreground">Settings</h1>
					<p className="text-[12px] text-muted-foreground">
						Customize theme colors, desktop chrome layout, notifications, JVM memory, network, and
						developer tools.
					</p>
				</div>

				<RouteNavTabs
					ariaLabel="Settings Sections"
					layoutId="settings-subtab-pill"
					items={tabItems}
				/>
			</div>

			<Outlet />
		</div>
	)
}

export default SettingsLayout
