import { InfoIcon, OctagonXIcon, TriangleAlertIcon, XIcon } from "lucide-react"
import { AnimatePresence, motion, type PanInfo } from "motion/react"
import {
	isValidElement,
	useCallback,
	useEffect,
	useMemo,
	useRef,
	useState,
	type MouseEvent,
	type ReactNode,
} from "react"
import { type Action, type ToastT } from "sonner"
import { cn } from "@/shared/lib/utils"
import {
	clearToastProgress,
	type ToastProgressState,
	useToastProgress,
} from "@/shared/ui/appleToastStore"

type AppleToastTone = "success" | "error" | "warning" | "info" | "loading" | "default"

interface ToneStyleConfig {
	medallionClass: string
	auraClass: string
	actionClass: string
	haloClass: string
}

const TONE_STYLES: Record<AppleToastTone, ToneStyleConfig> = {
	success: {
		medallionClass:
			"border-emerald-500/35 bg-linear-to-b from-emerald-400/25 to-emerald-500/12 text-emerald-600 shadow-[inset_0_1px_0_rgba(255,255,255,0.35)] dark:border-emerald-400/30 dark:from-emerald-400/24 dark:to-emerald-500/10 dark:text-emerald-300 dark:shadow-[inset_0_1px_0_rgba(255,255,255,0.16)]",
		auraClass: "bg-emerald-500/28 dark:bg-emerald-400/24",
		actionClass:
			"border border-emerald-500/25 bg-emerald-500/14 text-emerald-700 hover:bg-emerald-500/22 dark:text-emerald-200 dark:hover:bg-emerald-400/25",
		haloClass: "border-emerald-400/60",
	},
	error: {
		medallionClass:
			"border-rose-500/35 bg-linear-to-b from-rose-400/25 to-rose-500/12 text-rose-600 shadow-[inset_0_1px_0_rgba(255,255,255,0.35)] dark:border-rose-400/30 dark:from-rose-400/24 dark:to-rose-500/10 dark:text-rose-300 dark:shadow-[inset_0_1px_0_rgba(255,255,255,0.16)]",
		auraClass: "bg-rose-500/28 dark:bg-rose-400/24",
		actionClass:
			"border border-rose-500/25 bg-rose-500/14 text-rose-700 hover:bg-rose-500/22 dark:text-rose-200 dark:hover:bg-rose-400/25",
		haloClass: "border-rose-400/60",
	},
	warning: {
		medallionClass:
			"border-amber-500/35 bg-linear-to-b from-amber-400/25 to-amber-500/12 text-amber-600 shadow-[inset_0_1px_0_rgba(255,255,255,0.35)] dark:border-amber-400/30 dark:from-amber-400/24 dark:to-amber-500/10 dark:text-amber-300 dark:shadow-[inset_0_1px_0_rgba(255,255,255,0.16)]",
		auraClass: "bg-amber-500/28 dark:bg-amber-400/24",
		actionClass:
			"border border-amber-500/25 bg-amber-500/14 text-amber-700 hover:bg-amber-500/22 dark:text-amber-200 dark:hover:bg-amber-400/25",
		haloClass: "border-amber-400/60",
	},
	info: {
		medallionClass:
			"border-sky-500/35 bg-linear-to-b from-sky-400/25 to-sky-500/12 text-sky-600 shadow-[inset_0_1px_0_rgba(255,255,255,0.35)] dark:border-sky-400/30 dark:from-sky-400/24 dark:to-sky-500/10 dark:text-sky-300 dark:shadow-[inset_0_1px_0_rgba(255,255,255,0.16)]",
		auraClass: "bg-sky-500/28 dark:bg-sky-400/24",
		actionClass:
			"border border-sky-500/25 bg-sky-500/14 text-sky-700 hover:bg-sky-500/22 dark:text-sky-200 dark:hover:bg-sky-400/25",
		haloClass: "border-sky-400/60",
	},
	loading: {
		medallionClass:
			"border-primary/35 bg-linear-to-b from-primary/24 to-primary/10 text-primary shadow-[inset_0_1px_0_rgba(255,255,255,0.35)] dark:shadow-[inset_0_1px_0_rgba(255,255,255,0.16)]",
		auraClass: "bg-primary/26",
		actionClass: "border border-primary/25 bg-primary/14 text-primary hover:bg-primary/22",
		haloClass: "border-primary/55",
	},
	default: {
		medallionClass:
			"border-(--ingot-border-strong) bg-linear-to-b from-foreground/10 to-foreground/4 text-foreground shadow-[inset_0_1px_0_rgba(255,255,255,0.35)] dark:shadow-[inset_0_1px_0_rgba(255,255,255,0.14)]",
		auraClass: "bg-primary/20",
		actionClass: "border border-border bg-foreground/8 text-foreground hover:bg-foreground/14",
		haloClass: "border-foreground/40",
	},
}

const APPLE_SPRING = {
	type: "spring",
	stiffness: 450,
	damping: 30,
	mass: 0.8,
} as const

const PROGRESS_RING_RADIUS = 8
const PROGRESS_RING_CIRCUMFERENCE = 2 * Math.PI * PROGRESS_RING_RADIUS

const resolveToastTone = (toastType: ToastT["type"]): AppleToastTone => {
	if (toastType === "success") {
		return "success"
	}
	if (toastType === "error") {
		return "error"
	}
	if (toastType === "warning") {
		return "warning"
	}
	if (toastType === "info") {
		return "info"
	}
	if (toastType === "loading") {
		return "loading"
	}
	return "default"
}

const resolveRenderableNode = (value: (() => ReactNode) | ReactNode): ReactNode => {
	if (typeof value === "function") {
		return value()
	}
	return value
}

const isToastAction = (candidate: Action | ReactNode): candidate is Action => {
	if (typeof candidate !== "object" || candidate === null || isValidElement(candidate)) {
		return false
	}
	return "label" in candidate && "onClick" in candidate && typeof candidate.onClick === "function"
}

interface AppleToastGlyphProps {
	tone: AppleToastTone
	customIcon: ReactNode | undefined
	progress: ToastProgressState | undefined
}

const AppleToastGlyph = ({ tone, customIcon, progress }: AppleToastGlyphProps) => {
	if (progress !== undefined && !progress.indeterminate) {
		const dashOffset = PROGRESS_RING_CIRCUMFERENCE * (1 - progress.value / 100)
		return (
			<svg viewBox="0 0 22 22" className="size-5 -rotate-90" aria-hidden="true">
				<circle
					cx="11"
					cy="11"
					r={PROGRESS_RING_RADIUS}
					fill="none"
					stroke="currentColor"
					strokeOpacity={0.2}
					strokeWidth={2.3}
				/>
				<motion.circle
					cx="11"
					cy="11"
					r={PROGRESS_RING_RADIUS}
					fill="none"
					stroke="currentColor"
					strokeWidth={2.3}
					strokeLinecap="round"
					strokeDasharray={PROGRESS_RING_CIRCUMFERENCE}
					animate={{ strokeDashoffset: dashOffset }}
					transition={{ type: "spring", stiffness: 260, damping: 28 }}
				/>
			</svg>
		)
	}

	if (customIcon !== undefined && customIcon !== null && tone !== "loading") {
		return <span className="flex items-center justify-center">{customIcon}</span>
	}

	if (tone === "success") {
		return (
			<svg
				viewBox="0 0 20 20"
				fill="none"
				stroke="currentColor"
				strokeWidth={2.3}
				strokeLinecap="round"
				strokeLinejoin="round"
				className="size-4"
				aria-hidden="true"
			>
				<motion.path
					d="M5.2 10.4L8.5 13.6L14.8 6.8"
					initial={{ pathLength: 0, opacity: 0.4 }}
					animate={{ pathLength: 1, opacity: 1 }}
					transition={{ duration: 0.34, delay: 0.06, ease: [0.22, 1, 0.36, 1] }}
				/>
			</svg>
		)
	}

	if (tone === "error") {
		return <OctagonXIcon className="size-4" strokeWidth={2.1} aria-hidden="true" />
	}

	if (tone === "warning") {
		return <TriangleAlertIcon className="size-4" strokeWidth={2.1} aria-hidden="true" />
	}

	if (tone === "info") {
		return <InfoIcon className="size-4" strokeWidth={2.1} aria-hidden="true" />
	}

	if (tone === "loading") {
		return (
			<svg viewBox="0 0 22 22" className="size-4.5 animate-spin" aria-hidden="true">
				<circle
					cx="11"
					cy="11"
					r={PROGRESS_RING_RADIUS}
					fill="none"
					stroke="currentColor"
					strokeOpacity={0.2}
					strokeWidth={2.2}
				/>
				<circle
					cx="11"
					cy="11"
					r={PROGRESS_RING_RADIUS}
					fill="none"
					stroke="currentColor"
					strokeWidth={2.2}
					strokeLinecap="round"
					strokeDasharray={`${PROGRESS_RING_CIRCUMFERENCE * 0.32} ${PROGRESS_RING_CIRCUMFERENCE}`}
				/>
			</svg>
		)
	}

	return (
		<img
			src="/ingot.svg"
			alt=""
			draggable={false}
			className="size-4.5 object-contain"
			aria-hidden="true"
		/>
	)
}

export interface AppleToastCardProps {
	toastItem: ToastT
	stackIndex: number
	visibleLimit: number
	expanded: boolean
	collapsedOffset: number
	expandedOffset: number
	yPosition: "top" | "bottom"
	xPosition: "left" | "right" | "center"
	defaultDuration: number
	isStackPaused: boolean
	closeButtonVisible: boolean
	onHeightChange: (id: string | number, height: number) => void
	onAutoClose: (toastItem: ToastT) => void
	onDismiss: (toastItem: ToastT) => void
}

const AppleToastCard = ({
	toastItem,
	stackIndex,
	visibleLimit,
	expanded,
	collapsedOffset,
	expandedOffset,
	yPosition,
	xPosition,
	defaultDuration,
	isStackPaused,
	closeButtonVisible,
	onHeightChange,
	onAutoClose,
	onDismiss,
}: AppleToastCardProps) => {
	const progressState = useToastProgress(toastItem.id)

	const cardRef = useRef<HTMLLIElement | null>(null)
	const latestToastRef = useRef<ToastT>(toastItem)

	const tone = resolveToastTone(toastItem.type)
	const toneStyle = TONE_STYLES[tone]
	const dismissible = toastItem.dismissible !== false
	const isInteractiveCard = toastItem.jsx !== undefined && toastItem.jsx !== null
	const rawDuration = toastItem.duration ?? defaultDuration
	const effectiveDuration =
		tone === "loading" || progressState !== undefined ? Number.POSITIVE_INFINITY : rawDuration
	const hasAutoClose = Number.isFinite(effectiveDuration) && effectiveDuration > 0

	const remainingMsRef = useRef<number>(effectiveDuration)
	const timerStartedAtRef = useRef<number>(0)

	const [isDragging, setIsDragging] = useState(false)
	const [exitOffsetX, setExitOffsetX] = useState(0)

	const isPaused = isStackPaused || isDragging
	const titleNode = useMemo(() => {
		return resolveRenderableNode(toastItem.title)
	}, [toastItem.title])
	const descriptionNode = useMemo(() => {
		return resolveRenderableNode(toastItem.description)
	}, [toastItem.description])

	const hasAction = isValidElement(toastItem.action) || isToastAction(toastItem.action)
	const hasCancel = isValidElement(toastItem.cancel) || isToastAction(toastItem.cancel)
	const useFooterActions = hasAction && hasCancel

	const handleMeasureHeight = useCallback(() => {
		const element = cardRef.current
		if (!element) {
			return
		}
		onHeightChange(toastItem.id, element.getBoundingClientRect().height)
	}, [onHeightChange, toastItem.id])

	useEffect(() => {
		latestToastRef.current = toastItem
	}, [toastItem])

	useEffect(() => {
		const toastId = toastItem.id
		return () => {
			clearToastProgress(toastId)
		}
	}, [toastItem.id])

	useEffect(() => {
		const element = cardRef.current
		if (!element) {
			return
		}
		handleMeasureHeight()
		if (typeof ResizeObserver === "undefined") {
			return
		}
		const observer = new ResizeObserver(() => {
			handleMeasureHeight()
		})
		observer.observe(element)
		return () => {
			observer.disconnect()
		}
	}, [handleMeasureHeight])

	useEffect(() => {
		remainingMsRef.current = effectiveDuration
		timerStartedAtRef.current = Date.now()
	}, [effectiveDuration, tone, toastItem.title, toastItem.description])

	useEffect(() => {
		if (!hasAutoClose) {
			return
		}
		if (isPaused) {
			if (timerStartedAtRef.current > 0) {
				const elapsed = Date.now() - timerStartedAtRef.current
				remainingMsRef.current = Math.max(0, remainingMsRef.current - elapsed)
				timerStartedAtRef.current = 0
			}
			return
		}

		const remaining = remainingMsRef.current
		if (remaining <= 0) {
			onAutoClose(latestToastRef.current)
			return
		}

		timerStartedAtRef.current = Date.now()
		const timerId = window.setTimeout(() => {
			onAutoClose(latestToastRef.current)
		}, remaining)

		return () => {
			window.clearTimeout(timerId)
			if (timerStartedAtRef.current > 0) {
				const elapsed = Date.now() - timerStartedAtRef.current
				remainingMsRef.current = Math.max(0, remainingMsRef.current - elapsed)
				timerStartedAtRef.current = 0
			}
		}
	}, [
		hasAutoClose,
		isPaused,
		onAutoClose,
		toastItem.id,
		effectiveDuration,
		tone,
		toastItem.title,
		toastItem.description,
	])

	const handleDismissClick = useCallback(
		(event: MouseEvent<HTMLButtonElement>) => {
			event.stopPropagation()
			if (!dismissible) {
				return
			}
			onDismiss(toastItem)
		},
		[dismissible, onDismiss, toastItem],
	)

	const handleDragStart = useCallback(() => {
		setIsDragging(true)
	}, [])

	const handleDragEnd = useCallback(
		(_event: globalThis.MouseEvent | TouchEvent | PointerEvent, info: PanInfo) => {
			setIsDragging(false)
			if (!dismissible) {
				return
			}
			const swipedFar = Math.abs(info.offset.x) > 72
			const flickedFast = Math.abs(info.velocity.x) > 360
			if (swipedFar || flickedFast) {
				const direction = info.offset.x >= 0 ? 110 : -110
				setExitOffsetX(direction)
				onDismiss(toastItem)
			}
		},
		[dismissible, onDismiss, toastItem],
	)

	const isTop = yPosition === "top"
	const isWithinCollapsedLimit = stackIndex < visibleLimit
	const collapsedY = isTop ? collapsedOffset : -collapsedOffset
	const collapsedScale = Math.max(0.84, 1 - stackIndex * 0.045)
	const collapsedOpacity = isWithinCollapsedLimit ? Math.max(0.55, 1 - stackIndex * 0.17) : 0
	const targetY = expanded ? (isTop ? expandedOffset : -expandedOffset) : collapsedY
	const targetScale = expanded ? 1 : collapsedScale
	const targetOpacity = expanded ? 1 : collapsedOpacity
	const targetFilter = !expanded && stackIndex > 0 ? "blur(0.45px)" : "blur(0px)"
	const initialY = isTop ? -20 : 20
	const defaultExitX = xPosition === "left" ? -32 : xPosition === "center" ? 0 : 32
	const exitX = exitOffsetX !== 0 ? exitOffsetX : defaultExitX
	const zIndex = Math.max(1, 60 - stackIndex)

	const anchorClass = isTop ? "top-0 origin-top" : "bottom-0 origin-bottom"
	const horizontalClass =
		xPosition === "left"
			? "left-0"
			: xPosition === "center"
				? "left-1/2 -translate-x-1/2"
				: "right-0"
	const hasDescription = descriptionNode !== undefined && descriptionNode !== null
	const hasExpandedBody = hasDescription || progressState !== undefined || useFooterActions
	const headerAlignClass = hasExpandedBody ? "items-start" : "items-center"

	const renderActionControl = () => {
		if (isValidElement(toastItem.action)) {
			return toastItem.action
		}
		if (!isToastAction(toastItem.action)) {
			return null
		}
		const action = toastItem.action
		return (
			<button
				type="button"
				style={action.actionButtonStyle}
				onClick={(event) => {
					action.onClick(event)
					if (!event.defaultPrevented) {
						onDismiss(toastItem)
					}
				}}
				className={cn(
					"cursor-pointer rounded-full px-3 py-1 text-[11px] font-semibold transition-colors",
					toneStyle.actionClass,
				)}
			>
				{action.label}
			</button>
		)
	}

	const renderCancelControl = () => {
		if (isValidElement(toastItem.cancel)) {
			return toastItem.cancel
		}
		if (!isToastAction(toastItem.cancel)) {
			return null
		}
		const cancel = toastItem.cancel
		return (
			<button
				type="button"
				style={cancel.actionButtonStyle}
				onClick={(event) => {
					if (!dismissible) {
						return
					}
					cancel.onClick(event)
					onDismiss(toastItem)
				}}
				className="cursor-pointer rounded-full border border-border/70 bg-foreground/7 px-3 py-1 text-[11px] font-medium text-muted-foreground transition-colors hover:bg-foreground/14 hover:text-foreground"
			>
				{cancel.label}
			</button>
		)
	}

	return (
		<motion.li
			ref={cardRef}
			layout={false}
			initial={{ opacity: 0, y: initialY, scale: 0.9, filter: "blur(6px)" }}
			animate={{
				opacity: targetOpacity,
				y: targetY,
				scale: targetScale,
				filter: targetFilter,
			}}
			exit={{
				opacity: 0,
				x: exitX,
				y: isTop ? -10 : 10,
				scale: 0.92,
				filter: "blur(5px)",
			}}
			transition={APPLE_SPRING}
			drag={dismissible ? "x" : false}
			dragConstraints={{ left: 0, right: 0 }}
			dragElastic={0.42}
			onDragStart={handleDragStart}
			onDragEnd={handleDragEnd}
			whileTap={dismissible ? { scale: 0.985 } : undefined}
			style={{ zIndex }}
			data-testid={toastItem.testId}
			className={cn(
				"group/toast pointer-events-auto absolute w-88 max-w-[calc(100vw-1.5rem)] list-none select-none",
				anchorClass,
				horizontalClass,
				!expanded && !isWithinCollapsedLimit && "pointer-events-none",
			)}
		>
			{isInteractiveCard ? (
				<div className="w-full">{toastItem.jsx}</div>
			) : (
				<motion.div
					key={tone === "error" ? `error-shake-${String(toastItem.id)}` : "steady"}
					initial={tone === "error" ? { x: 0 } : false}
					animate={tone === "error" ? { x: [0, -5, 5, -3, 2, 0] } : undefined}
					transition={tone === "error" ? { duration: 0.36, ease: "easeOut" } : undefined}
					role={tone === "error" ? "alert" : "status"}
					className={cn(
						"relative overflow-hidden rounded-[22px] border border-white/55 bg-(--ingot-surface-elevated) px-3.5 py-3 text-popover-foreground ring-1 ring-black/6 backdrop-blur-2xl backdrop-saturate-180 dark:border-white/14 dark:ring-black/40",
						"shadow-[0_18px_44px_-12px_rgba(15,23,42,0.26),0_4px_12px_rgba(15,23,42,0.08),inset_0_1px_0_rgba(255,255,255,0.72)]",
						"dark:shadow-[0_24px_52px_-12px_rgba(0,0,0,0.68),0_4px_14px_rgba(0,0,0,0.34),inset_0_1px_0_rgba(255,255,255,0.16)]",
						toastItem.className,
					)}
				>
					<span
						aria-hidden="true"
						className="pointer-events-none absolute inset-0 bg-linear-to-b from-white/45 via-white/4 to-black/3 dark:from-white/8 dark:via-transparent dark:to-black/22"
					/>
					<span
						aria-hidden="true"
						className={cn(
							"pointer-events-none absolute -top-9 -left-7 size-28 rounded-full opacity-55 blur-2xl",
							toneStyle.auraClass,
						)}
					/>

					<div className="relative z-10 flex flex-col">
						<div className={cn("flex gap-3", headerAlignClass)}>
							{toastItem.icon !== null ? (
								<div
									className={cn(
										"relative flex size-9 shrink-0 items-center justify-center rounded-[13px] border",
										hasExpandedBody && "mt-0.5",
										toneStyle.medallionClass,
									)}
								>
									{tone === "success" && progressState === undefined ? (
										<motion.span
											key={`halo-${String(toastItem.id)}`}
											aria-hidden="true"
											initial={{ scale: 0.72, opacity: 0.65 }}
											animate={{ scale: 1.55, opacity: 0 }}
											transition={{ duration: 0.58, ease: [0.22, 1, 0.36, 1] }}
											className={cn(
												"pointer-events-none absolute inset-0 rounded-[13px] border",
												toneStyle.haloClass,
											)}
										/>
									) : null}

									<AnimatePresence mode="popLayout" initial={false}>
										<motion.span
											key={
												progressState !== undefined ? "progress" : toastItem.icon ? "custom" : tone
											}
											initial={{ opacity: 0, scale: 0.55, rotate: -10 }}
											animate={{ opacity: 1, scale: 1, rotate: 0 }}
											exit={{ opacity: 0, scale: 0.55, rotate: 10 }}
											transition={{ type: "spring", stiffness: 520, damping: 28 }}
											className="flex items-center justify-center"
										>
											<AppleToastGlyph
												tone={tone}
												customIcon={toastItem.icon}
												progress={progressState}
											/>
										</motion.span>
									</AnimatePresence>
								</div>
							) : null}

							<div className={cn("min-w-0 flex-1", hasExpandedBody && "pt-0.5")}>
								<div className="flex items-center justify-between gap-2">
									{titleNode !== undefined && titleNode !== null ? (
										<p className="truncate text-[12.5px] leading-snug font-semibold tracking-tight text-popover-foreground">
											{titleNode}
										</p>
									) : null}
									{progressState !== undefined && !progressState.indeterminate ? (
										<span className="shrink-0 rounded-full bg-primary/14 px-1.5 py-0.5 font-mono text-[10.5px] font-semibold text-primary tabular-nums">
											{Math.round(progressState.value)}%
										</span>
									) : null}
								</div>

								{descriptionNode !== undefined && descriptionNode !== null ? (
									<p className="mt-0.5 text-[11.5px] leading-snug text-muted-foreground">
										{descriptionNode}
									</p>
								) : null}
							</div>

							<div
								className={cn("flex shrink-0 items-center gap-1.5", hasExpandedBody && "pt-0.5")}
								onPointerDown={(event) => {
									event.stopPropagation()
								}}
							>
								{!useFooterActions ? renderActionControl() : null}

								{dismissible ? (
									<button
										type="button"
										onClick={handleDismissClick}
										aria-label="Dismiss notification"
										className={cn(
											"flex size-5.5 shrink-0 cursor-pointer items-center justify-center rounded-full border border-border/50 bg-foreground/8 text-muted-foreground transition-all hover:border-border hover:bg-foreground/15 hover:text-foreground focus-visible:opacity-100",
											closeButtonVisible
												? "opacity-100"
												: "opacity-0 group-hover/toast:opacity-100",
										)}
									>
										<XIcon className="size-3" aria-hidden="true" />
									</button>
								) : null}
							</div>
						</div>

						{progressState !== undefined ? (
							<div className="mt-2.5 flex flex-col gap-1">
								{progressState.detail ? (
									<span className="truncate font-mono text-[10.5px] text-muted-foreground tabular-nums">
										{progressState.detail}
									</span>
								) : null}
								<div className="h-1.5 w-full overflow-hidden rounded-full bg-foreground/10 p-px">
									<motion.div
										animate={{ width: `${progressState.value}%` }}
										transition={{ type: "spring", stiffness: 280, damping: 30 }}
										className="h-full rounded-full bg-linear-to-r from-primary via-primary to-emerald-400 shadow-[0_0_10px_var(--primary)]"
									/>
								</div>
							</div>
						) : null}

						{useFooterActions ? (
							<div
								className="mt-2.5 flex items-center justify-end gap-1.5 border-t border-border/50 pt-2"
								onPointerDown={(event) => {
									event.stopPropagation()
								}}
							>
								{renderCancelControl()}
								{renderActionControl()}
							</div>
						) : null}
					</div>
				</motion.div>
			)}
		</motion.li>
	)
}

export default AppleToastCard
