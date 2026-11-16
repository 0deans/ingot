import { AnimatePresence } from "motion/react"
import { useCallback, useEffect, useMemo, useRef, useState } from "react"
import { toast, useSonner, type ToasterProps, type ToastT } from "sonner"
import { cn } from "@/shared/lib/utils"
import AppleToastCard from "@/shared/ui/AppleToastCard"
import { appleToast, clearToastProgress } from "@/shared/ui/appleToastStore"

type ToastPosition = NonNullable<ToasterProps["position"]>

const ALL_POSITIONS: ToastPosition[] = [
	"top-left",
	"top-center",
	"top-right",
	"bottom-left",
	"bottom-center",
	"bottom-right",
]

const POSITION_CONTAINER_CLASS: Record<ToastPosition, string> = {
	"top-left": "top-12 left-3 md:left-4",
	"top-center": "top-12 left-1/2 -translate-x-1/2",
	"top-right": "top-12 right-3 md:right-4",
	"bottom-left": "bottom-20 left-3 md:bottom-4 md:left-4",
	"bottom-center": "bottom-20 left-1/2 -translate-x-1/2 md:bottom-4",
	"bottom-right": "bottom-20 right-3 md:bottom-4 md:right-4",
}

const DEFAULT_CARD_HEIGHT_PX = 58
const COLLAPSED_PEEK_OFFSET_PX = 9

const splitPositionAxes = (
	pos: ToastPosition,
): { yPosition: "top" | "bottom"; xPosition: "left" | "right" | "center" } => {
	const [yRaw, xRaw] = pos.split("-")
	const yPosition = yRaw === "top" ? "top" : "bottom"
	const xPosition = xRaw === "left" ? "left" : xRaw === "center" ? "center" : "right"
	return { yPosition, xPosition }
}

const resolveHotkeyMatch = (event: KeyboardEvent, hotkey: string[]): boolean => {
	if (hotkey.length === 0) {
		return false
	}
	return hotkey.every((key) => {
		if (key === "altKey") {
			return event.altKey
		}
		if (key === "ctrlKey") {
			return event.ctrlKey
		}
		if (key === "metaKey") {
			return event.metaKey
		}
		if (key === "shiftKey") {
			return event.shiftKey
		}
		return event.code === key || event.key === key
	})
}

const computeExpandedOffsets = (
	items: ToastT[],
	heightsById: Record<string, number>,
	gap: number,
): number[] => {
	const offsets: number[] = []
	let cumulative = 0
	for (const item of items) {
		offsets.push(cumulative)
		const cardHeight = heightsById[String(item.id)] ?? DEFAULT_CARD_HEIGHT_PX
		cumulative += cardHeight + gap
	}
	return offsets
}

const computeCollapsedOffsets = (
	items: ToastT[],
	heightsById: Record<string, number>,
): number[] => {
	if (items.length === 0) {
		return []
	}
	const frontItem = items[0]
	const frontHeight = frontItem
		? (heightsById[String(frontItem.id)] ?? DEFAULT_CARD_HEIGHT_PX)
		: DEFAULT_CARD_HEIGHT_PX

	return items.map((item, index) => {
		if (index === 0) {
			return 0
		}
		const selfHeight = heightsById[String(item.id)] ?? DEFAULT_CARD_HEIGHT_PX
		const scale = Math.max(0.84, 1 - index * 0.045)
		const targetTopAboveFront = frontHeight + index * COLLAPSED_PEEK_OFFSET_PX
		return Math.max(index * COLLAPSED_PEEK_OFFSET_PX, targetTopAboveFront - selfHeight * scale)
	})
}

const computeContainerHeight = (
	items: ToastT[],
	heightsById: Record<string, number>,
	expanded: boolean,
	visibleLimit: number,
	gap: number,
): number => {
	if (items.length === 0) {
		return 0
	}
	if (expanded) {
		let total = 0
		for (let i = 0; i < items.length; i += 1) {
			const item = items[i]
			const cardHeight = item ? (heightsById[String(item.id)] ?? DEFAULT_CARD_HEIGHT_PX) : 0
			total += cardHeight
			if (i > 0) {
				total += gap
			}
		}
		return total
	}

	const frontItem = items[0]
	const frontHeight = frontItem
		? (heightsById[String(frontItem.id)] ?? DEFAULT_CARD_HEIGHT_PX)
		: DEFAULT_CARD_HEIGHT_PX
	const peekCount = Math.max(0, Math.min(items.length, visibleLimit) - 1)
	return frontHeight + peekCount * COLLAPSED_PEEK_OFFSET_PX
}

const Toaster = ({
	id,
	position = "bottom-right",
	hotkey = ["altKey", "KeyT"],
	expand = false,
	duration = 4000,
	gap = 10,
	visibleToasts = 3,
	closeButton = false,
	toastOptions,
	className,
	customAriaLabel,
	containerAriaLabel = "Notifications",
}: ToasterProps) => {
	const { toasts: rawToasts } = useSonner()

	const listRef = useRef<HTMLOListElement | null>(null)

	const [isHovered, setIsHovered] = useState(false)
	const [isDocumentHidden, setIsDocumentHidden] = useState(() => {
		if (typeof document === "undefined") {
			return false
		}
		return document.hidden
	})
	const [heightsById, setHeightsById] = useState<Record<string, number>>({})

	const filteredToasts = useMemo(() => {
		if (id) {
			return rawToasts.filter((item) => {
				return item.toasterId === id
			})
		}
		return rawToasts.filter((item) => {
			return !item.toasterId
		})
	}, [id, rawToasts])

	const toastsByPosition = useMemo(() => {
		const grouped: Record<ToastPosition, ToastT[]> = {
			"top-left": [],
			"top-center": [],
			"top-right": [],
			"bottom-left": [],
			"bottom-center": [],
			"bottom-right": [],
		}
		for (const item of filteredToasts) {
			const effectivePosition = item.position ?? position
			grouped[effectivePosition].push(item)
		}
		return grouped
	}, [filteredToasts, position])

	const effectiveDuration = toastOptions?.duration ?? duration
	const effectiveCloseButton = toastOptions?.closeButton ?? closeButton
	const activeHovered = isHovered && filteredToasts.length > 0
	const isExpanded = expand || (activeHovered && filteredToasts.length > 1)
	const isStackPaused = activeHovered || isDocumentHidden

	const handleHeightChange = useCallback((toastId: string | number, height: number) => {
		const key = String(toastId)
		setHeightsById((prev) => {
			const current = prev[key]
			if (current !== undefined && Math.abs(current - height) < 0.5) {
				return prev
			}
			return { ...prev, [key]: height }
		})
	}, [])

	const pruneToastState = useCallback(
		(toastId: string | number) => {
			clearToastProgress(toastId)
			if (filteredToasts.length <= 1) {
				setIsHovered(false)
			}
			const key = String(toastId)
			setHeightsById((prev) => {
				if (!(key in prev)) {
					return prev
				}
				const next = { ...prev }
				delete next[key]
				return next
			})
		},
		[filteredToasts.length],
	)

	const handleAutoClose = useCallback(
		(toastItem: ToastT) => {
			toastItem.onAutoClose?.(toastItem)
			toast.dismiss(toastItem.id)
			pruneToastState(toastItem.id)
		},
		[pruneToastState],
	)

	const handleDismiss = useCallback(
		(toastItem: ToastT) => {
			toastItem.onDismiss?.(toastItem)
			toast.dismiss(toastItem.id)
			pruneToastState(toastItem.id)
		},
		[pruneToastState],
	)

	useEffect(() => {
		if (!isHovered || filteredToasts.length > 0) {
			return
		}
		const handlePointerMove = () => {
			setIsHovered(false)
		}
		window.addEventListener("pointermove", handlePointerMove, { once: true })
		return () => {
			window.removeEventListener("pointermove", handlePointerMove)
		}
	}, [filteredToasts.length, isHovered])

	useEffect(() => {
		const handleVisibilityChange = () => {
			setIsDocumentHidden(document.hidden)
		}
		document.addEventListener("visibilitychange", handleVisibilityChange)
		return () => {
			document.removeEventListener("visibilitychange", handleVisibilityChange)
		}
	}, [])

	useEffect(() => {
		const handleKeyDown = (event: KeyboardEvent) => {
			if (resolveHotkeyMatch(event, hotkey)) {
				setIsHovered(true)
				listRef.current?.focus()
				return
			}
			if (event.code === "Escape" && listRef.current?.contains(document.activeElement)) {
				setIsHovered(false)
			}
		}
		document.addEventListener("keydown", handleKeyDown)
		return () => {
			document.removeEventListener("keydown", handleKeyDown)
		}
	}, [hotkey])

	const hotkeyLabel = hotkey.join("+").replace(/Key/g, "")

	return (
		<section
			aria-label={customAriaLabel ?? `${containerAriaLabel} ${hotkeyLabel}`}
			aria-live="polite"
			aria-relevant="additions text"
			aria-atomic="false"
			tabIndex={-1}
			className="pointer-events-none fixed inset-0 z-50 overflow-hidden"
		>
			{ALL_POSITIONS.map((slotPosition) => {
				const slotItems = toastsByPosition[slotPosition]
				const maxRendered = isExpanded ? Math.max(visibleToasts, 5) : visibleToasts + 1
				const renderedItems = slotItems.slice(0, maxRendered)
				const hasItems = renderedItems.length > 0
				const { yPosition, xPosition } = splitPositionAxes(slotPosition)
				const expandedOffsets = computeExpandedOffsets(renderedItems, heightsById, gap)
				const collapsedOffsets = computeCollapsedOffsets(renderedItems, heightsById)
				const stackHeight = computeContainerHeight(
					renderedItems,
					heightsById,
					isExpanded,
					visibleToasts,
					gap,
				)

				return (
					<ol
						key={slotPosition}
						ref={slotPosition === position ? listRef : undefined}
						tabIndex={hasItems ? -1 : undefined}
						onMouseEnter={() => {
							if (hasItems) {
								setIsHovered(true)
							}
						}}
						onMouseLeave={() => {
							setIsHovered(false)
						}}
						style={{
							height: `${stackHeight}px`,
						}}
						className={cn(
							"fixed m-0 w-88 max-w-[calc(100vw-1.5rem)] list-none p-0 outline-none",
							POSITION_CONTAINER_CLASS[slotPosition],
							hasItems ? "pointer-events-auto" : "pointer-events-none",
							className,
						)}
					>
						<AnimatePresence initial={true}>
							{renderedItems.map((toastItem, stackIndex) => {
								const expandedOffset = expandedOffsets[stackIndex] ?? 0
								const collapsedOffset =
									collapsedOffsets[stackIndex] ?? stackIndex * COLLAPSED_PEEK_OFFSET_PX
								return (
									<AppleToastCard
										key={String(toastItem.id)}
										toastItem={toastItem}
										stackIndex={stackIndex}
										visibleLimit={visibleToasts}
										expanded={isExpanded}
										collapsedOffset={collapsedOffset}
										expandedOffset={expandedOffset}
										yPosition={yPosition}
										xPosition={xPosition}
										defaultDuration={effectiveDuration}
										isStackPaused={isStackPaused}
										closeButtonVisible={effectiveCloseButton}
										onHeightChange={handleHeightChange}
										onAutoClose={handleAutoClose}
										onDismiss={handleDismiss}
									/>
								)
							})}
						</AnimatePresence>
					</ol>
				)
			})}
		</section>
	)
}

export { appleToast, toast, Toaster }
export default Toaster
