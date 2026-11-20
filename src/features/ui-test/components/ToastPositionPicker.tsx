import type { TFunction } from "i18next"
import { motion } from "motion/react"
import { useTranslation } from "react-i18next"
import { useBreakpoint } from "@/features/shell/hooks/useBreakpoint"
import { cn } from "@/shared/lib/utils"
import {
	useLauncherPrefsStore,
	type NavVariant,
	type ToastPosition,
} from "@/shared/stores/useLauncherPrefsStore"

export type { ToastPosition }

type VerticalEdge = "top" | "bottom"

type PositionLabelKey =
	| "toastPicker.positions.topLeft"
	| "toastPicker.positions.topCenter"
	| "toastPicker.positions.topRight"
	| "toastPicker.positions.bottomLeft"
	| "toastPicker.positions.bottomCenter"
	| "toastPicker.positions.bottomRight"

type PositionShortLabelKey =
	| "toastPicker.positions.topL"
	| "toastPicker.positions.topC"
	| "toastPicker.positions.topR"
	| "toastPicker.positions.botL"
	| "toastPicker.positions.botC"
	| "toastPicker.positions.botR"

type MobileEdgeLabelKey = "toastPicker.positions.top" | "toastPicker.positions.bottom"

interface PositionSlot {
	value: ToastPosition
	labelKey: PositionLabelKey
	shortLabelKey: PositionShortLabelKey
	alignClass: string
}

interface MobileEdgeSlot {
	edge: VerticalEdge
	labelKey: MobileEdgeLabelKey
}

const POSITION_SLOTS: PositionSlot[] = [
	{
		value: "top-left",
		labelKey: "toastPicker.positions.topLeft",
		shortLabelKey: "toastPicker.positions.topL",
		alignClass: "items-start justify-start",
	},
	{
		value: "top-center",
		labelKey: "toastPicker.positions.topCenter",
		shortLabelKey: "toastPicker.positions.topC",
		alignClass: "items-start justify-center",
	},
	{
		value: "top-right",
		labelKey: "toastPicker.positions.topRight",
		shortLabelKey: "toastPicker.positions.topR",
		alignClass: "items-start justify-end",
	},
	{
		value: "bottom-left",
		labelKey: "toastPicker.positions.bottomLeft",
		shortLabelKey: "toastPicker.positions.botL",
		alignClass: "items-end justify-start",
	},
	{
		value: "bottom-center",
		labelKey: "toastPicker.positions.bottomCenter",
		shortLabelKey: "toastPicker.positions.botC",
		alignClass: "items-end justify-center",
	},
	{
		value: "bottom-right",
		labelKey: "toastPicker.positions.bottomRight",
		shortLabelKey: "toastPicker.positions.botR",
		alignClass: "items-end justify-end",
	},
]

const MOBILE_EDGE_SLOTS: MobileEdgeSlot[] = [
	{ edge: "top", labelKey: "toastPicker.positions.top" },
	{ edge: "bottom", labelKey: "toastPicker.positions.bottom" },
]

const findPositionLabel = (position: ToastPosition, t: TFunction): string => {
	const found = POSITION_SLOTS.find((slot) => {
		return slot.value === position
	})
	return found ? t(found.labelKey) : t("toastPicker.positions.bottomRight")
}

const getVerticalEdge = (position: ToastPosition): VerticalEdge => {
	return position.startsWith("top-") ? "top" : "bottom"
}

const replaceVerticalEdge = (position: ToastPosition, edge: VerticalEdge): ToastPosition => {
	if (position.endsWith("-left")) {
		return edge === "top" ? "top-left" : "bottom-left"
	}
	if (position.endsWith("-center")) {
		return edge === "top" ? "top-center" : "bottom-center"
	}
	return edge === "top" ? "top-right" : "bottom-right"
}

const resolveActivePositionLabel = (
	position: ToastPosition,
	isMobile: boolean,
	t: TFunction,
): string => {
	if (!isMobile) {
		return findPositionLabel(position, t)
	}
	return getVerticalEdge(position) === "top"
		? t("toastPicker.positions.top")
		: t("toastPicker.positions.bottom")
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
			<div className="flex h-6 shrink-0 items-center justify-between border-b border-border/70 bg-sidebar/85 px-3">
				<div className="flex items-center gap-1.5">
					<img src="/ingot.svg" alt="" draggable={false} className="size-3 object-contain" />
					<span className="text-[9.5px] font-semibold tracking-tight text-foreground/90">
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

const MiniMobileShellChrome = () => {
	return (
		<div aria-hidden="true" className="pointer-events-none flex flex-1 flex-col">
			<div className="flex h-8 shrink-0 items-center justify-between border-b border-border/70 bg-sidebar/85 px-3">
				<div className="flex items-center gap-1.5">
					<span className="size-3 rounded-md bg-primary/30 ring-1 ring-primary/45" />
					<span className="h-1.5 w-9 rounded-full bg-foreground/70" />
				</div>

				<div className="flex items-center gap-1">
					<span className="h-2.5 w-4 rounded-full border border-border/60 bg-foreground/6" />
					<span className="size-2.5 rounded-full border border-border/60 bg-foreground/6" />
					<span className="size-3 rounded-full bg-primary/35" />
				</div>
			</div>

			<div className="flex flex-1 flex-col items-center justify-center gap-2 bg-background/30 px-4">
				<span className="h-1.5 w-2/3 rounded-full bg-foreground/8" />
				<span className="h-1.5 w-1/2 rounded-full bg-foreground/6" />
			</div>

			<div className="flex h-8 shrink-0 items-center justify-around border-t border-border/70 bg-sidebar/85 px-3">
				<span className="h-2.5 w-6 rounded-full bg-primary/45" />
				<span className="size-2 rounded-full bg-foreground/15" />
				<span className="size-2 rounded-full bg-foreground/15" />
				<span className="size-2 rounded-full bg-foreground/15" />
			</div>
		</div>
	)
}

interface PositionSlotButtonProps {
	label: string
	shortLabel: string
	isSelected: boolean
	layoutId: string
	showDetails: boolean
	fullWidth?: boolean
	onSelect: () => void
}

const PositionSlotButton = ({
	label,
	shortLabel,
	isSelected,
	layoutId,
	showDetails,
	fullWidth = false,
	onSelect,
}: PositionSlotButtonProps) => {
	if (isSelected) {
		return (
			<motion.button
				type="button"
				aria-label={label}
				aria-pressed={true}
				layoutId={layoutId}
				transition={{ type: "spring", stiffness: 460, damping: 32 }}
				onClick={onSelect}
				className={cn(
					"flex cursor-pointer items-center gap-1.5 rounded-lg border border-primary/50 bg-(--ingot-surface-elevated) shadow-[0_6px_16px_rgba(0,0,0,0.35)] backdrop-blur-md focus-visible:ring-1 focus-visible:ring-primary/70 focus-visible:outline-none",
					fullWidth ? "w-full px-2.5 py-2" : "w-24 px-2 py-1.5",
				)}
			>
				<span className="size-2 shrink-0 rounded-sm bg-primary shadow-[0_0_6px_var(--primary)]" />
				<span className="flex min-w-0 flex-1 flex-col gap-0.5">
					<span className="h-1 w-full rounded-full bg-foreground/85" />
					{showDetails ? <span className="h-1 w-2/3 rounded-full bg-foreground/35" /> : null}
				</span>
			</motion.button>
		)
	}

	return (
		<button
			type="button"
			aria-label={label}
			aria-pressed={false}
			onClick={onSelect}
			className={cn(
				"flex cursor-pointer items-center justify-center rounded-lg border border-border/75 bg-background/55 px-1.5 font-medium text-muted-foreground/85 transition-[transform,border-color,background-color,color,box-shadow] duration-150 ease-out hover:scale-[1.04] hover:border-primary/65 hover:bg-primary/14 hover:text-foreground hover:shadow-[0_0_12px_color-mix(in_srgb,var(--primary)_24%,transparent)] focus-visible:ring-1 focus-visible:ring-primary/70 focus-visible:outline-none active:scale-95",
				fullWidth ? "h-8 w-full text-[10px]" : "h-7.5 w-19 text-[10px]",
			)}
		>
			{shortLabel}
		</button>
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
	const { t } = useTranslation()
	const breakpoint = useBreakpoint()
	const navVariant = useLauncherPrefsStore((s) => {
		return s.navVariant
	})

	const isMobile = breakpoint === "mobile"
	const activeVerticalEdge = getVerticalEdge(value)
	const activeLabel = resolveActivePositionLabel(value, isMobile, t)
	const groupAria = t("toastPicker.groupAria")

	return (
		<div className="flex min-w-0 flex-col gap-2">
			<div className="flex items-center justify-between gap-3">
				<span className="text-[11.5px] font-medium text-foreground/90">
					{t("toastPicker.screenPosition")}
				</span>
				<span className="rounded-full border border-primary/25 bg-primary/14 px-2 py-0.5 text-[10.5px] font-semibold text-primary">
					{activeLabel}
				</span>
			</div>

			{isMobile ? (
				<div
					role="group"
					aria-label={groupAria}
					className="relative mx-auto flex h-76 w-46 max-w-full flex-col overflow-hidden rounded-3xl border-2 border-border/90 bg-background/90 shadow-inner select-none"
				>
					<MiniMobileShellChrome />

					<div className="absolute inset-x-0 top-8 bottom-8 flex flex-col justify-between p-2.5">
						{MOBILE_EDGE_SLOTS.map((slot) => {
							const slotLabel = t(slot.labelKey)
							return (
								<PositionSlotButton
									key={slot.edge}
									label={slotLabel}
									shortLabel={slotLabel}
									isSelected={activeVerticalEdge === slot.edge}
									layoutId={`${layoutId}-mobile`}
									showDetails={showDetails}
									fullWidth={true}
									onSelect={() => {
										onChange(replaceVerticalEdge(value, slot.edge))
									}}
								/>
							)
						})}
					</div>
				</div>
			) : (
				<div
					role="group"
					aria-label={groupAria}
					className="relative flex h-48 w-96 max-w-full flex-col overflow-hidden rounded-xl border border-border bg-background/90 shadow-inner select-none"
				>
					<MiniAppShellChrome navVariant={navVariant} />

					<div className="absolute inset-x-0 top-6 bottom-0 grid grid-cols-3 grid-rows-2 gap-2 p-3">
						{POSITION_SLOTS.map((slot) => {
							return (
								<div key={slot.value} className={cn("flex p-0.5", slot.alignClass)}>
									<PositionSlotButton
										label={t(slot.labelKey)}
										shortLabel={t(slot.shortLabelKey)}
										isSelected={slot.value === value}
										layoutId={layoutId}
										showDetails={showDetails}
										onSelect={() => {
											onChange(slot.value)
										}}
									/>
								</div>
							)
						})}
					</div>
				</div>
			)}

			<span className="text-[11px] text-muted-foreground">
				{isMobile ? t("toastPicker.mobileHint") : t("toastPicker.desktopHint")}
			</span>
		</div>
	)
}

export default ToastPositionPicker
