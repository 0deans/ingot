import { motion } from "motion/react"
import { cn } from "@/shared/lib/utils"
import {
	useLauncherPrefsStore,
	type NavVariant,
	type ToastPosition,
} from "@/shared/stores/useLauncherPrefsStore"

export type { ToastPosition }

interface PositionSlot {
	value: ToastPosition
	label: string
	shortLabel: string
	alignClass: string
}

const POSITION_SLOTS: PositionSlot[] = [
	{
		value: "top-left",
		label: "Top Left",
		shortLabel: "Top L",
		alignClass: "items-start justify-start",
	},
	{
		value: "top-center",
		label: "Top Center",
		shortLabel: "Top C",
		alignClass: "items-start justify-center",
	},
	{
		value: "top-right",
		label: "Top Right",
		shortLabel: "Top R",
		alignClass: "items-start justify-end",
	},
	{
		value: "bottom-left",
		label: "Bottom Left",
		shortLabel: "Bot L",
		alignClass: "items-end justify-start",
	},
	{
		value: "bottom-center",
		label: "Bottom Center",
		shortLabel: "Bot C",
		alignClass: "items-end justify-center",
	},
	{
		value: "bottom-right",
		label: "Bottom Right",
		shortLabel: "Bot R",
		alignClass: "items-end justify-end",
	},
]

const findPositionLabel = (position: ToastPosition): string => {
	const found = POSITION_SLOTS.find((slot) => {
		return slot.value === position
	})
	return found ? found.label : "Bottom Right"
}

interface MiniAppShellChromeProps {
	navVariant: NavVariant
}

const MiniAppShellChrome = ({ navVariant }: MiniAppShellChromeProps) => {
	const isCapsule = navVariant === "capsule"
	const isSequoia = navVariant === "sequoia"
	const isRail = navVariant === "rail"

	return (
		<div aria-hidden="true" className="pointer-events-none flex flex-1 flex-col">
			<div className="flex h-5 shrink-0 items-center justify-between border-b border-border/70 bg-sidebar/85 px-2.5">
				<div className="flex items-center gap-1.5">
					<img src="/ingot.svg" alt="" draggable={false} className="size-2.5 object-contain" />
					<span className="text-[8.5px] font-semibold tracking-tight text-foreground/90">
						Ingot
					</span>
				</div>

				{isCapsule ? (
					<div className="flex items-center gap-1 rounded-full border border-border/70 bg-background/75 px-1.5 py-0.5">
						<span className="h-1 w-3 rounded-full bg-primary/80" />
						<span className="size-1 rounded-full bg-foreground/22" />
						<span className="size-1 rounded-full bg-foreground/22" />
						<span className="size-1 rounded-full bg-foreground/22" />
					</div>
				) : null}

				<div className="flex items-center gap-1.5">
					<span className="h-1.5 w-4 rounded-full border border-border/60 bg-foreground/6" />
					<div className="flex items-center gap-1.5 text-muted-foreground/80">
						<svg viewBox="0 0 8 8" className="size-1.5" fill="none" stroke="currentColor">
							<line x1="1" y1="4" x2="7" y2="4" strokeWidth="1.2" strokeLinecap="round" />
						</svg>
						<svg viewBox="0 0 8 8" className="size-1.5" fill="none" stroke="currentColor">
							<rect x="1.2" y="1.2" width="5.6" height="5.6" rx="0.8" strokeWidth="1.1" />
						</svg>
						<svg viewBox="0 0 8 8" className="size-1.5" fill="none" stroke="currentColor">
							<line x1="1.5" y1="1.5" x2="6.5" y2="6.5" strokeWidth="1.2" strokeLinecap="round" />
							<line x1="6.5" y1="1.5" x2="1.5" y2="6.5" strokeWidth="1.2" strokeLinecap="round" />
						</svg>
					</div>
				</div>
			</div>

			<div className="flex min-h-0 flex-1">
				{!isCapsule ? (
					<div
						className={cn(
							"flex shrink-0 flex-col gap-1 border-r border-border/55 bg-sidebar/65 p-1.5",
							isRail ? "w-5 items-center" : "w-12",
							isSequoia && "my-1.5 ml-1.5 rounded-lg border border-sidebar-border shadow-xs",
						)}
					>
						<span className={cn("rounded-sm bg-primary/40", isRail ? "size-2" : "h-1.5 w-full")} />
						<span
							className={cn("rounded-sm bg-foreground/12", isRail ? "size-2" : "h-1.5 w-4/5")}
						/>
						<span
							className={cn("rounded-sm bg-foreground/10", isRail ? "size-2" : "h-1.5 w-3/4")}
						/>
					</div>
				) : null}

				<div
					className={cn(
						"flex-1 bg-background/30",
						isSequoia && "my-1.5 mr-1.5 ml-1 rounded-lg border border-border/40",
					)}
				/>
			</div>
		</div>
	)
}

interface ToastPositionPickerProps {
	value: ToastPosition
	showDetails?: boolean
	layoutId?: string
	onChange: (next: ToastPosition) => void
}

const ToastPositionPicker = ({
	value,
	showDetails = true,
	layoutId = "toast-position-card",
	onChange,
}: ToastPositionPickerProps) => {
	const navVariant = useLauncherPrefsStore((s) => {
		return s.navVariant
	})

	const activeLabel = findPositionLabel(value)

	return (
		<div className="flex min-w-0 flex-col gap-2">
			<div className="flex items-center justify-between gap-3">
				<span className="text-[11.5px] font-medium text-foreground/90">Screen Position</span>
				<span className="rounded-full border border-primary/25 bg-primary/14 px-2 py-0.5 text-[10.5px] font-semibold text-primary">
					{activeLabel}
				</span>
			</div>

			<div
				role="group"
				aria-label="Notification screen position"
				className="relative flex h-34 w-74 max-w-full flex-col overflow-hidden rounded-xl border border-border bg-background/90 shadow-inner select-none"
			>
				<MiniAppShellChrome navVariant={navVariant} />

				<div className="absolute inset-x-0 top-5 bottom-0 grid grid-cols-3 grid-rows-2 gap-1.5 p-2">
					{POSITION_SLOTS.map((slot) => {
						const isSelected = slot.value === value

						return (
							<div key={slot.value} className={cn("flex p-0.5", slot.alignClass)}>
								{isSelected ? (
									<motion.button
										type="button"
										aria-label={slot.label}
										aria-pressed={true}
										layoutId={layoutId}
										transition={{ type: "spring", stiffness: 460, damping: 32 }}
										onClick={() => {
											onChange(slot.value)
										}}
										className="flex w-19 cursor-pointer items-center gap-1.5 rounded-lg border border-primary/50 bg-(--ingot-surface-elevated) px-1.5 py-1 shadow-[0_6px_16px_rgba(0,0,0,0.35)] backdrop-blur-md focus-visible:ring-1 focus-visible:ring-primary/70 focus-visible:outline-none"
									>
										<span className="size-2 shrink-0 rounded-sm bg-primary shadow-[0_0_6px_var(--primary)]" />
										<span className="flex min-w-0 flex-1 flex-col gap-0.5">
											<span className="h-1 w-full rounded-full bg-foreground/85" />
											{showDetails ? (
												<span className="h-1 w-2/3 rounded-full bg-foreground/35" />
											) : null}
										</span>
									</motion.button>
								) : (
									<button
										type="button"
										aria-label={slot.label}
										aria-pressed={false}
										onClick={() => {
											onChange(slot.value)
										}}
										className="flex h-6.5 w-15.5 cursor-pointer items-center justify-center rounded-lg border border-border/75 bg-background/55 px-1.5 text-[9.5px] font-medium text-muted-foreground/85 transition-[transform,border-color,background-color,color,box-shadow] duration-150 ease-out hover:scale-[1.04] hover:border-primary/65 hover:bg-primary/14 hover:text-foreground hover:shadow-[0_0_12px_color-mix(in_srgb,var(--primary)_24%,transparent)] focus-visible:ring-1 focus-visible:ring-primary/70 focus-visible:outline-none active:scale-95"
									>
										{slot.shortLabel}
									</button>
								)}
							</div>
						)
					})}
				</div>
			</div>

			<span className="text-[11px] text-muted-foreground">
				Click any corner or edge inside the window frame.
			</span>
		</div>
	)
}

export default ToastPositionPicker
