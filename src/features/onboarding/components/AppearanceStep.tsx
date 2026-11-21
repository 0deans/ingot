import { LayoutTemplate, Palette } from "lucide-react"
import { useTranslation } from "react-i18next"
import { useShallow } from "zustand/react/shallow"
import OnboardingStepIntro from "@/features/onboarding/components/OnboardingStepIntro"
import SelectionCheckBadge from "@/features/onboarding/components/SelectionCheckBadge"
import { ONBOARDING_NAV_VARIANTS, ONBOARDING_THEMES } from "@/features/onboarding/onboardingSteps"
import { cn } from "@/shared/lib/utils"
import { useLauncherPrefsStore, type NavVariant } from "@/shared/stores/useLauncherPrefsStore"

interface AppearanceStepProps {
	isMobile: boolean
}

interface NavMiniPreviewProps {
	variant: NavVariant
}

const NavMiniPreview = ({ variant }: NavMiniPreviewProps) => {
	if (variant === "capsule") {
		return (
			<div className="flex h-10 w-full flex-col gap-1 rounded-md border border-border/80 bg-background/70 p-1">
				<div className="mx-auto h-1.5 w-10 rounded-full bg-primary/60" />
				<div className="flex-1 rounded-sm bg-foreground/8" />
			</div>
		)
	}

	if (variant === "rail") {
		return (
			<div className="flex h-10 w-full gap-1 rounded-md border border-border/80 bg-background/70 p-1">
				<div className="w-2 shrink-0 rounded-sm bg-primary/55" />
				<div className="flex-1 rounded-sm bg-foreground/8" />
			</div>
		)
	}

	if (variant === "sequoia") {
		return (
			<div className="flex h-10 w-full gap-1 rounded-md border border-border/80 bg-background/70 p-1">
				<div className="w-4 shrink-0 rounded-sm bg-primary/50" />
				<div className="flex-1 rounded-md border border-border/80 bg-foreground/10" />
			</div>
		)
	}

	return (
		<div className="flex h-10 w-full gap-1 rounded-md border border-border/80 bg-background/70 p-1">
			<div className="w-5 shrink-0 rounded-sm bg-primary/55" />
			<div className="flex-1 rounded-sm bg-foreground/8" />
		</div>
	)
}

const AppearanceStep = ({ isMobile }: AppearanceStepProps) => {
	const { t } = useTranslation()
	const { theme, setTheme, navVariant, setNavVariant } = useLauncherPrefsStore(
		useShallow((s) => {
			return {
				theme: s.theme,
				setTheme: s.setTheme,
				navVariant: s.navVariant,
				setNavVariant: s.setNavVariant,
			}
		}),
	)

	const subtitle = isMobile
		? t("onboarding.appearance.subtitleMobile")
		: t("onboarding.appearance.subtitle")

	return (
		<div className="flex flex-col gap-5">
			<OnboardingStepIntro
				icon={Palette}
				title={t("onboarding.appearance.title")}
				subtitle={subtitle}
			/>

			<div className="flex flex-col gap-2">
				<span className="text-[11px] font-semibold tracking-wider text-muted-foreground uppercase">
					{t("onboarding.appearance.themeHeading")}
				</span>

				<div className="grid grid-cols-1 gap-2 sm:grid-cols-3">
					{ONBOARDING_THEMES.map((option) => {
						const isSelected = theme === option.id

						return (
							<button
								key={option.id}
								type="button"
								onClick={() => {
									setTheme(option.id)
								}}
								className={cn(
									"relative flex cursor-pointer items-center gap-3 rounded-xl border p-3 text-left backdrop-blur-md outline-none sm:flex-col sm:items-stretch sm:gap-2 sm:p-2.5",
									isSelected
										? "border-primary/50 bg-primary/14 text-foreground"
										: "border-border/80 bg-card/55 text-muted-foreground hover:bg-card/80 hover:text-foreground",
								)}
							>
								<div
									className={cn(
										"flex h-10 w-16 shrink-0 items-end justify-between rounded-lg border p-1.5 sm:w-full",
										option.surfaceClass,
									)}
								>
									<span className={cn("h-2 w-6 rounded-full", option.accentClass)} />
									{isSelected ? <SelectionCheckBadge size="sm" className="hidden sm:flex" /> : null}
								</div>

								<span className="min-w-0 flex-1 truncate text-[13px] font-medium text-foreground sm:flex-initial sm:text-[12px]">
									{t(option.labelKey)}
								</span>

								{isSelected ? <SelectionCheckBadge size="md" className="sm:hidden" /> : null}
							</button>
						)
					})}
				</div>
			</div>

			{!isMobile ? (
				<div className="flex flex-col gap-2">
					<span className="inline-flex items-center gap-1.5 text-[11px] font-semibold tracking-wider text-muted-foreground uppercase">
						<LayoutTemplate className="size-3" />
						{t("onboarding.appearance.navHeading")}
					</span>

					<div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
						{ONBOARDING_NAV_VARIANTS.map((option) => {
							const isSelected = navVariant === option.id

							return (
								<button
									key={option.id}
									type="button"
									onClick={() => {
										setNavVariant(option.id)
									}}
									className={cn(
										"flex cursor-pointer flex-col gap-2 rounded-xl border p-2.5 text-left backdrop-blur-md outline-none",
										isSelected
											? "border-primary/50 bg-primary/14 text-foreground"
											: "border-border/80 bg-card/55 text-muted-foreground hover:bg-card/80 hover:text-foreground",
									)}
								>
									<NavMiniPreview variant={option.id} />
									<span className="truncate text-[11.5px] font-medium text-foreground">
										{t(option.labelKey)}
									</span>
								</button>
							)
						})}
					</div>
				</div>
			) : null}
		</div>
	)
}

export default AppearanceStep
