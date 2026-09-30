import { Check, Globe, Monitor, Moon, Sun } from "lucide-react"
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
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { useLanguage } from "@/i18n/use-language"
import { useTheme } from "@/lib/theme"
import { cn } from "@/lib/utils"

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
					<div className="grid grid-cols-1 gap-1.5 p-1 sm:grid-cols-2">
						{locales.map((loc) => {
							const isSelected = language === loc.code
							const isSystemDefault = systemLanguage === loc.code
							return (
								<button
									key={loc.code}
									type="button"
									onClick={() => previewLanguage(loc.code)}
									className={cn(
										"flex min-w-0 items-center gap-3 rounded-lg border px-3 py-2.5 text-left transition-colors",
										isSelected
											? "border-primary/50 bg-primary/10"
											: "border-border/80 bg-card/30 hover:border-input hover:bg-card/70",
									)}
								>
									<span className="min-w-0 flex-1">
										<span
											className={cn(
												"block truncate font-medium text-sm",
												isSelected ? "text-foreground" : "text-foreground",
											)}
										>
											{loc.nativeName}
										</span>
										<span className="block truncate text-[11px] text-muted-foreground">
											{loc.name}
											{isSystemDefault && (
												<span className="text-primary"> · {t("onboarding.detectedBadge")}</span>
											)}
										</span>
									</span>
									<span
										className={cn(
											"flex size-4 shrink-0 items-center justify-center rounded-full border transition-colors",
											isSelected
												? "border-primary bg-primary text-primary-foreground"
												: "border-input",
										)}
									>
										{isSelected && <Check className="size-2.5" strokeWidth={3} />}
									</span>
								</button>
							)
						})}
					</div>
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
