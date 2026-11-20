import { Check, Copy, Layout, Monitor, Palette } from "lucide-react"
import { useCallback, useMemo, useState } from "react"
import { useTranslation } from "react-i18next"
import { toast } from "sonner"
import { useShallow } from "zustand/react/shallow"
import CustomThemeEditor from "@/features/settings/components/CustomThemeEditor"
import NavVariantCard, {
	type NavVariantOption,
} from "@/features/settings/components/NavVariantCard"
import NotificationSettingsCard from "@/features/settings/components/NotificationSettingsCard"
import ThemeOptionCard, {
	type ThemeOptionMeta,
} from "@/features/settings/components/ThemeOptionCard"
import { useBreakpoint } from "@/features/shell/hooks/useBreakpoint"
import { cn } from "@/shared/lib/utils"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"
import {
	THEME_PRESETS,
	stringifyCustomThemeJson,
	type AppThemeId,
	type CustomThemeColors,
	type CustomThemeConfig,
} from "@/shared/theme/customTheme"
import { Button } from "@/shared/ui/button"

type ThemeTitleKey =
	| "settings.appearance.themes.light.title"
	| "settings.appearance.themes.dark.title"
	| "settings.appearance.themes.purple.title"
	| "settings.appearance.themes.custom.title"

const THEME_TITLE_KEYS: Record<AppThemeId, ThemeTitleKey> = {
	light: "settings.appearance.themes.light.title",
	dark: "settings.appearance.themes.dark.title",
	purple: "settings.appearance.themes.purple.title",
	custom: "settings.appearance.themes.custom.title",
}

const getThemeColors = (themeId: AppThemeId, customTheme: CustomThemeConfig): CustomThemeColors => {
	if (themeId === "custom") {
		return customTheme.colors
	}

	return THEME_PRESETS[themeId].colors
}

const AppearanceSettingsPage = () => {
	const { t } = useTranslation()
	const breakpoint = useBreakpoint()
	const { theme, setTheme, navVariant, setNavVariant, customTheme } = useLauncherPrefsStore(
		useShallow((s) => {
			return {
				theme: s.theme,
				setTheme: s.setTheme,
				navVariant: s.navVariant,
				setNavVariant: s.setNavVariant,
				customTheme: s.customTheme,
			}
		}),
	)

	const [copiedActiveJson, setCopiedActiveJson] = useState<boolean>(false)

	const themeOptions = useMemo<readonly ThemeOptionMeta[]>(() => {
		return [
			{
				id: "light",
				title: t("settings.appearance.themes.light.title"),
				description: t("settings.appearance.themes.light.description"),
			},
			{
				id: "dark",
				title: t("settings.appearance.themes.dark.title"),
				description: t("settings.appearance.themes.dark.description"),
			},
			{
				id: "purple",
				title: t("settings.appearance.themes.purple.title"),
				description: t("settings.appearance.themes.purple.description"),
			},
			{
				id: "custom",
				title: t("settings.appearance.themes.custom.title"),
				description: t("settings.appearance.themes.custom.description"),
			},
		]
	}, [t])

	const navVariants = useMemo<readonly NavVariantOption[]>(() => {
		return [
			{
				id: "sonoma",
				title: t("settings.appearance.navVariants.sonoma.title"),
				description: t("settings.appearance.navVariants.sonoma.description"),
			},
			{
				id: "rail",
				title: t("settings.appearance.navVariants.rail.title"),
				description: t("settings.appearance.navVariants.rail.description"),
			},
			{
				id: "sequoia",
				title: t("settings.appearance.navVariants.sequoia.title"),
				description: t("settings.appearance.navVariants.sequoia.description"),
			},
			{
				id: "capsule",
				title: t("settings.appearance.navVariants.capsule.title"),
				description: t("settings.appearance.navVariants.capsule.description"),
			},
		]
	}, [t])

	const isMobile = breakpoint === "mobile"

	const handleCopyActiveThemeJson = useCallback(async () => {
		const config = theme === "custom" ? customTheme : THEME_PRESETS[theme]
		const payload = stringifyCustomThemeJson(config)
		try {
			await navigator.clipboard.writeText(payload)
			setCopiedActiveJson(true)
			window.setTimeout(() => {
				setCopiedActiveJson(false)
			}, 1600)
			toast.success(
				t("settings.appearance.copiedThemeToast", { theme: t(THEME_TITLE_KEYS[theme]) }),
			)
		} catch {
			toast.error(t("settings.appearance.copyThemeErrorToast"))
		}
	}, [customTheme, t, theme])

	const CopyJsonIcon = copiedActiveJson ? Check : Copy
	const copyJsonLabel = copiedActiveJson
		? t("settings.appearance.copiedJson")
		: t("settings.appearance.copyThemeJson")

	return (
		<div className="flex flex-col gap-5">
			<section className="@container rounded-2xl border border-border bg-card/65 p-4 backdrop-blur-md">
				<div className="mb-3 flex flex-wrap items-center justify-between gap-2">
					<div className="flex items-center gap-2">
						<Palette className="size-4 text-primary" />
						<h2 className="text-[13.5px] font-semibold text-foreground">
							{t("settings.appearance.colorTheme")}
						</h2>
					</div>

					<Button
						type="button"
						variant="outline"
						size="sm"
						onClick={() => {
							void handleCopyActiveThemeJson()
						}}
						className="cursor-pointer bg-foreground/5 text-[11.5px] hover:bg-foreground/10"
					>
						<CopyJsonIcon
							className={
								copiedActiveJson ? "size-3.5 text-primary" : "size-3.5 text-muted-foreground"
							}
						/>
						<span>{copyJsonLabel}</span>
					</Button>
				</div>

				<div className="grid grid-cols-1 gap-2.5 @md:grid-cols-2 @3xl:grid-cols-4">
					{themeOptions.map((option) => {
						return (
							<ThemeOptionCard
								key={option.id}
								option={option}
								colors={getThemeColors(option.id, customTheme)}
								isSelected={theme === option.id}
								onSelect={setTheme}
							/>
						)
					})}
				</div>
			</section>

			{theme === "custom" ? <CustomThemeEditor /> : null}

			<section className="rounded-2xl border border-border bg-card/65 p-4 backdrop-blur-md">
				<div className="mb-3 flex flex-wrap items-center justify-between gap-2">
					<div className="flex items-center gap-2">
						<Layout className="size-4 text-primary" />
						<h2 className="text-[13.5px] font-semibold text-foreground">
							{t("settings.appearance.navigationLayout")}
						</h2>
					</div>

					{isMobile ? (
						<span className="inline-flex items-center gap-1 rounded-full border border-border bg-foreground/5 px-2.5 py-0.5 text-[10.5px] font-medium text-muted-foreground">
							<Monitor className="size-3 shrink-0" />
							<span>{t("settings.appearance.mobileLayoutOnly")}</span>
						</span>
					) : null}
				</div>

				<div className={cn("grid grid-cols-1 gap-2.5 sm:grid-cols-2", isMobile && "opacity-75")}>
					{navVariants.map((variant) => {
						return (
							<NavVariantCard
								key={variant.id}
								variant={variant}
								isSelected={navVariant === variant.id}
								onSelect={setNavVariant}
							/>
						)
					})}
				</div>
			</section>

			<NotificationSettingsCard />
		</div>
	)
}

export default AppearanceSettingsPage
