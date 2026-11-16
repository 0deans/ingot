import { motion } from "motion/react"
import { cn } from "@/shared/lib/utils"

export interface SegmentedOption<T extends string> {
	value: T
	label: string
	dotColor?: string
	dotClass?: string
}

interface SegmentedControlProps<T extends string> {
	layoutId: string
	options: SegmentedOption<T>[]
	value: T
	onChange: (value: T) => void
	size?: "sm" | "md"
	tone?: "neutral" | "emerald"
}

const SegmentedControl = <T extends string>({
	layoutId,
	options,
	value,
	onChange,
	size = "sm",
	tone = "neutral",
}: SegmentedControlProps<T>) => {
	const activePillClass =
		tone === "emerald" ? "border-primary/35 bg-primary/15" : "border-foreground/15 bg-foreground/10"
	const activeTextClass = tone === "emerald" ? "text-primary" : "text-foreground"

	return (
		<div
			className="no-scrollbar inline-flex items-center gap-0.5 overflow-x-auto rounded-xl border border-border bg-card/80 p-1"
			role="tablist"
		>
			{options.map((opt) => {
				const isActive = opt.value === value
				const sizeClass = size === "sm" ? "h-7 px-2.5 text-xs" : "h-8 px-3 text-xs"
				const textClass = isActive ? activeTextClass : "text-muted-foreground hover:text-foreground"

				return (
					<button
						key={opt.value}
						type="button"
						role="tab"
						aria-selected={isActive}
						onClick={() => {
							onChange(opt.value)
						}}
						className={cn(
							"relative flex cursor-pointer items-center gap-1.5 rounded-lg font-medium whitespace-nowrap transition-colors select-none",
							sizeClass,
							textClass,
						)}
					>
						{isActive ? (
							<motion.span
								layoutId={layoutId}
								transition={{ duration: 0.24, ease: [0.22, 1, 0.36, 1] }}
								className={cn(
									"pointer-events-none absolute inset-0 rounded-lg border shadow-xs",
									activePillClass,
								)}
							/>
						) : null}
						{opt.dotClass ? (
							<span className={cn("relative z-10 h-2 w-2 shrink-0 rounded-full", opt.dotClass)} />
						) : null}
						{!opt.dotClass && opt.dotColor ? (
							<span
								className="relative z-10 h-2 w-2 shrink-0 rounded-full"
								style={{ backgroundColor: opt.dotColor }}
							/>
						) : null}
						<span className="relative z-10">{opt.label}</span>
					</button>
				)
			})}
		</div>
	)
}

export default SegmentedControl
