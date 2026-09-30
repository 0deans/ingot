import { Globe, Monitor, Moon, Sun } from "lucide-react"
import { memo } from "react"
import { ScrollArea } from "@/components/common/scroll-area"
import { Button } from "@/components/ui/button"
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog"
import {
	Field,
	FieldContent,
	FieldDescription,
	FieldLabel,
	FieldTitle,
} from "@/components/ui/field"
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { useLanguage } from "@/i18n/use-language"
import { useTheme } from "@/lib/theme"

const THEMES = [
	{ mode: "auto", icon: Monitor, label: "settings.theme.auto" },
	{ mode: "dark", icon: Moon, label: "settings.theme.dark" },
	{ mode: "light", icon: Sun, label: "settings.theme.light" },
] as const

export const FirstLaunchLanguageDialog = () => {
	const {
		language,
		currentLocale,
		systemLanguage,
		locales,
		isFirstLaunchPending,
		previewLanguage,
		confirmLanguage,
		t,
	} = useLanguage()
	const { themeMode, setThemeMode } = useTheme()

	if (!isFirstLaunchPending) {
		return null
	}

	const handleConfirm = () => {
		confirmLanguage(language)
	}

	return (
		<Dialog
			open={isFirstLaunchPending}
			onOpenChange={(open) => {
				if (!open) {
					handleConfirm()
				}
			}}
		>
			<DialogContent className="flex max-h-[85vh] flex-col gap-4 p-5 sm:max-w-lg">
				<DialogHeader className="gap-1.5 pr-8">
					<div className="flex items-center gap-2.5">
						<div className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
							<Globe className="size-5" />
						</div>
						<DialogTitle className="font-semibold text-foreground text-lg">
							{t("onboarding.selectLanguageTitle")}
						</DialogTitle>
					</div>
					<DialogDescription className="text-muted-foreground text-xs leading-relaxed">
						{t("onboarding.selectLanguageSubtitle")}
					</DialogDescription>
				</DialogHeader>

				<ScrollArea scrollFade className="-mx-1 min-h-0 flex-1">
					<RadioGroup
						value={language}
						onValueChange={(code) => previewLanguage(code as string)}
						className="grid-cols-1 gap-1.5 p-1 sm:grid-cols-2"
					>
						{locales.map((loc) => (
							<FieldLabel key={loc.code} htmlFor={`language-${loc.code}`}>
								<Field orientation="horizontal" className="items-center">
									<FieldContent className="min-w-0">
										<FieldTitle className="truncate">{loc.nativeName}</FieldTitle>
										<FieldDescription className="truncate">
											{loc.name}
											{systemLanguage === loc.code && (
												<span className="text-primary"> · {t("onboarding.detectedBadge")}</span>
											)}
										</FieldDescription>
									</FieldContent>
									<RadioGroupItem value={loc.code} id={`language-${loc.code}`} />
								</Field>
							</FieldLabel>
						))}
					</RadioGroup>
				</ScrollArea>

				<DialogFooter className="flex flex-row items-center gap-2 pt-4 sm:justify-between">
					<ToggleGroup
						variant="outline"
						spacing={0}
						value={[themeMode]}
						onValueChange={(value) => {
							const mode = THEMES.find((th) => th.mode === value[0])?.mode
							if (mode) setThemeMode(mode)
						}}
						className="shrink-0"
					>
						{THEMES.map(({ mode, icon: Icon, label }) => (
							<Tooltip key={mode}>
								<TooltipTrigger render={<ToggleGroupItem value={mode} aria-label={t(label)} />}>
									<Icon />
								</TooltipTrigger>
								<TooltipContent>{t(label)}</TooltipContent>
							</Tooltip>
						))}
					</ToggleGroup>
					<Button onClick={handleConfirm} className="h-9 min-w-0 flex-1 sm:flex-none sm:px-6">
						<span className="truncate">
							{t("onboarding.continueWith", {
								language: currentLocale.nativeName,
							})}
						</span>
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	)
}

FirstLaunchLanguageDialog.displayName = "FirstLaunchLanguageDialog"

export default memo(FirstLaunchLanguageDialog)
