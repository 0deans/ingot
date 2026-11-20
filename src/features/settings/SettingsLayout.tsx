import { Outlet, useRouterState } from "@tanstack/react-router"
import { useTranslation } from "react-i18next"
import RouteNavTabs, { type RouteNavTabItem } from "@/shared/components/motion/RouteNavTabs"

interface SettingsTabDef {
	to:
		| "/settings"
		| "/settings/language"
		| "/settings/java"
		| "/settings/accounts"
		| "/settings/network"
	labelKey:
		| "settings.tabs.appearance"
		| "settings.tabs.language"
		| "settings.tabs.java"
		| "settings.tabs.accounts"
		| "settings.tabs.network"
}

const SETTINGS_TABS: readonly SettingsTabDef[] = [
	{ to: "/settings", labelKey: "settings.tabs.appearance" },
	{ to: "/settings/language", labelKey: "settings.tabs.language" },
	{ to: "/settings/java", labelKey: "settings.tabs.java" },
	{ to: "/settings/accounts", labelKey: "settings.tabs.accounts" },
	{ to: "/settings/network", labelKey: "settings.tabs.network" },
]

const SettingsLayout = () => {
	const { t } = useTranslation()
	const pathname = useRouterState({
		select: (state) => {
			return state.location.pathname
		},
	})

	const normalizedPath = pathname.replace(/\/+$/, "") || "/"
	const tabItems: RouteNavTabItem[] = SETTINGS_TABS.map((item) => {
		return {
			key: item.to,
			label: t(item.labelKey),
			isActive: normalizedPath === item.to,
			link: { to: item.to },
		}
	})

	return (
		<div className="flex flex-col gap-5">
			<div className="flex flex-wrap items-center justify-between gap-3">
				<div>
					<h1 className="text-lg font-semibold tracking-tight text-foreground">
						{t("settings.title")}
					</h1>
					<p className="text-[12px] text-muted-foreground">{t("settings.subtitle")}</p>
				</div>

				<RouteNavTabs
					ariaLabel={t("settings.sectionsAria")}
					layoutId="settings-subtab-pill"
					items={tabItems}
				/>
			</div>

			<Outlet />
		</div>
	)
}

export default SettingsLayout
