import { Check, Globe } from "lucide-react"
import { memo } from "react"
import { Button } from "@/components/ui/button"
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog"
import { useLanguage } from "@/i18n/use-language"
import { cn } from "@/lib/utils"

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
			<DialogContent className="max-w-md border-border/60 bg-zinc-950 p-6 shadow-2xl backdrop-blur-2xl">
				<DialogHeader className="gap-2">
					<div className="flex items-center gap-2.5 text-emerald-400">
						<div className="flex size-9 items-center justify-center rounded-lg bg-emerald-500/10 text-emerald-400">
							<Globe className="size-5" />
						</div>
						<DialogTitle className="font-semibold text-lg text-white">
							{t("onboarding.selectLanguageTitle")}
						</DialogTitle>
					</div>
					<DialogDescription className="text-xs text-zinc-300 leading-relaxed">
						{t("onboarding.selectLanguageSubtitle")}
					</DialogDescription>
				</DialogHeader>

				<div className="grid max-h-72 grid-cols-1 gap-2 overflow-y-auto py-2 sm:grid-cols-2">
					{locales.map((loc) => {
						const isSelected = language === loc.code
						const isSystemDefault = systemLanguage === loc.code
						return (
							<button
								key={loc.code}
								type="button"
								onClick={() => previewLanguage(loc.code)}
								className={cn(
									"group flex flex-col items-start gap-1.5 rounded-xl border p-3.5 text-left transition-all",
									isSelected
										? "border-primary/60 bg-primary/10 shadow-sm"
										: "border-zinc-800 bg-zinc-900/40 hover:border-zinc-700 hover:bg-zinc-900/80",
								)}
							>
								<div className="flex w-full items-center justify-between gap-2">
									<span className="rounded bg-zinc-900 px-1.5 py-0.5 font-mono text-[10px] text-zinc-400 uppercase">
										{loc.code}
									</span>
									<div className="flex items-center gap-1.5">
										{isSystemDefault && (
											<span className="rounded-full bg-emerald-500/15 px-2 py-0.5 font-medium text-[10px] text-emerald-400">
												{t("onboarding.detectedBadge")}
											</span>
										)}
										{isSelected && (
											<span className="flex size-4 items-center justify-center rounded-full bg-primary text-primary-foreground">
												<Check className="size-2.5" />
											</span>
										)}
									</div>
								</div>

								<div className="mt-0.5">
									<div
										className={cn(
											"font-semibold text-sm",
											isSelected ? "text-foreground" : "text-zinc-200",
										)}
									>
										{loc.nativeName}
									</div>
									<div className="text-[11px] text-muted-foreground">{loc.name}</div>
								</div>
							</button>
						)
					})}
				</div>

				<DialogFooter className="pt-2">
					<Button onClick={handleConfirm} className="w-full gap-2">
						<span>
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
