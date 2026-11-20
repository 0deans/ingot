import { Outlet, useRouterState } from "@tanstack/react-router"
import { useTranslation } from "react-i18next"
import RouteNavTabs, { type RouteNavTabItem } from "@/shared/components/motion/RouteNavTabs"

interface SettingsTabDef {
	to:
		| "/settings"
		| "/settings/appearance"
		| "/settings/game"
		| "/settings/accounts"
		| "/settings/language"
	labelKey:
		| "settings.tabs.general"
		| "settings.tabs.appearance"
		| "settings.tabs.game"
		| "settings.tabs.accounts"
		| "settings.tabs.language"
}

const SETTINGS_TABS: readonly SettingsTabDef[] = [
	{ to: "/settings", labelKey: "settings.tabs.general" },
	{ to: "/settings/appearance", labelKey: "settings.tabs.appearance" },
	{ to: "/settings/game", labelKey: "settings.tabs.game" },
	{ to: "/settings/accounts", labelKey: "settings.tabs.accounts" },
	{ to: "/settings/language", labelKey: "settings.tabs.language" },
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
