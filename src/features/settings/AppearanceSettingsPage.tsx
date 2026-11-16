import { Check, Copy, Layout, Palette } from "lucide-react"
import { useTheme } from "next-themes"
import { useCallback, useState } from "react"
import { toast } from "sonner"
import { useShallow } from "zustand/react/shallow"
import CustomThemeEditor from "@/features/settings/components/CustomThemeEditor"
import DeveloperSettingsCard from "@/features/settings/components/DeveloperSettingsCard"
import NavVariantCard, {
	type NavVariantOption,
} from "@/features/settings/components/NavVariantCard"
import NotificationSettingsCard from "@/features/settings/components/NotificationSettingsCard"
import ThemeOptionCard, {
	type ThemeOptionMeta,
} from "@/features/settings/components/ThemeOptionCard"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"
import {
	THEME_PRESETS,
	isAppThemeId,
	stringifyCustomThemeJson,
	type AppThemeId,
	type CustomThemeColors,
	type CustomThemeConfig,
} from "@/shared/theme/customTheme"

const NAV_VARIANTS: readonly NavVariantOption[] = [
	{
		id: "sonoma",
		title: "Sonoma Sidebar",
		description: "Grouped macOS-style persistent sidebar with fluid active pill.",
	},
	{
		id: "rail",
		title: "Collapsible Rail",
		description: "Compact icon rail with manual expand toggle for maximum canvas width.",
	},
	{
		id: "sequoia",
		title: "Sequoia Island",
		description: "Detached translucent glass panel floating inside the window frame.",
	},
	{
		id: "capsule",
		title: "Top Capsule",
		description: "Zero-sidebar layout with centered segmented navigation inside titlebar.",
	},
]

const THEME_OPTIONS: readonly ThemeOptionMeta[] = [
	{
		id: "light",
		title: "Light",
		description: "Crisp daylight surfaces with balanced emerald accents.",
	},
	{
		id: "dark",
		title: "Dark",
		description: "Deep obsidian graphite chrome with high-contrast text.",
	},
	{
		id: "purple",
		title: "Purple",
		description: "Royal amethyst glass panels paired with violet accents.",
	},
	{
		id: "custom",
		title: "Custom",
		description: "Personalize every color token, radius, and JSON style preset.",
	},
]

const getThemeColors = (themeId: AppThemeId, customTheme: CustomThemeConfig): CustomThemeColors => {
	if (themeId === "custom") {
		return customTheme.colors
	}

	return THEME_PRESETS[themeId].colors
}

const AppearanceSettingsPage = () => {
	const { theme, setTheme } = useTheme()
	const { navVariant, setNavVariant, customTheme } = useLauncherPrefsStore(
		useShallow((s) => {
			return {
				navVariant: s.navVariant,
				setNavVariant: s.setNavVariant,
				customTheme: s.customTheme,
			}
		}),
	)

	const [copiedActiveJson, setCopiedActiveJson] = useState<boolean>(false)

	const activeTheme: AppThemeId = isAppThemeId(theme) ? theme : "dark"

	const handleCopyActiveThemeJson = useCallback(async () => {
		const config = activeTheme === "custom" ? customTheme : THEME_PRESETS[activeTheme]
		const payload = stringifyCustomThemeJson(config)
		try {
			await navigator.clipboard.writeText(payload)
			setCopiedActiveJson(true)
			window.setTimeout(() => {
				setCopiedActiveJson(false)
			}, 1600)
			toast.success(`Copied ${activeTheme} theme JSON to clipboard.`)
		} catch {
			toast.error("Failed to copy theme JSON to clipboard.")
		}
	}, [activeTheme, customTheme])

	const CopyJsonIcon = copiedActiveJson ? Check : Copy
	const copyJsonLabel = copiedActiveJson ? "Copied JSON" : "Copy Theme JSON"

	return (
		<div className="flex flex-col gap-5">
			<section className="@container rounded-2xl border border-border bg-card/65 p-4 backdrop-blur-md">
				<div className="mb-3 flex flex-wrap items-center justify-between gap-2">
					<div className="flex items-center gap-2">
						<Palette className="size-4 text-primary" />
						<h2 className="text-[13.5px] font-semibold text-foreground">Color Theme</h2>
					</div>

					<button
						type="button"
						onClick={() => {
							void handleCopyActiveThemeJson()
						}}
						className="inline-flex cursor-pointer items-center gap-1.5 rounded-lg border border-border bg-foreground/5 px-2.5 py-1 text-[11.5px] font-medium text-foreground transition-colors hover:bg-foreground/10"
					>
						<CopyJsonIcon
							className={
								copiedActiveJson ? "size-3.5 text-primary" : "size-3.5 text-muted-foreground"
							}
						/>
						<span>{copyJsonLabel}</span>
					</button>
				</div>

				<div className="grid grid-cols-1 gap-2.5 @md:grid-cols-2 @3xl:grid-cols-4">
					{THEME_OPTIONS.map((option) => {
						return (
							<ThemeOptionCard
								key={option.id}
								option={option}
								colors={getThemeColors(option.id, customTheme)}
								isSelected={activeTheme === option.id}
								onSelect={setTheme}
							/>
						)
					})}
				</div>
			</section>

			{activeTheme === "custom" ? <CustomThemeEditor /> : null}

			<section className="rounded-2xl border border-border bg-card/65 p-4 backdrop-blur-md">
				<div className="mb-3 flex items-center gap-2">
					<Layout className="size-4 text-primary" />
					<h2 className="text-[13.5px] font-semibold text-foreground">Navigation Layout</h2>
				</div>

				<div className="grid grid-cols-1 gap-2.5 sm:grid-cols-2">
					{NAV_VARIANTS.map((variant) => {
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

			<DeveloperSettingsCard />
		</div>
	)
}

export default AppearanceSettingsPage
