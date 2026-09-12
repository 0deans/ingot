import { Slider as BaseSlider } from "@base-ui/react"
import { memo } from "react"
import { cn } from "@/lib/utils"

export interface SliderProps {
	value?: number | readonly number[]
	defaultValue?: number | readonly number[]
	onValueChange?: (value: number | readonly number[]) => void
	onValueCommitted?: (value: number | readonly number[]) => void
	min?: number
	max?: number
	step?: number
	minStepsBetweenValues?: number
	disabled?: boolean
	className?: string
}

const Slider = ({
	value,
	defaultValue,
	onValueChange,
	onValueCommitted,
	min = 0,
	max = 100,
	step = 1,
	minStepsBetweenValues = 0,
	disabled = false,
	className = "",
}: SliderProps) => {
	const currentValues = Array.isArray(value)
		? value
		: Array.isArray(defaultValue)
			? defaultValue
			: typeof value === "number"
				? [value]
				: typeof defaultValue === "number"
					? [defaultValue]
					: [0]

	return (
		<BaseSlider.Root
			value={value}
			defaultValue={defaultValue}
			onValueChange={onValueChange}
			onValueCommitted={onValueCommitted}
			min={min}
			max={max}
			step={step}
			minStepsBetweenValues={minStepsBetweenValues}
			disabled={disabled}
			className={cn("relative flex w-full touch-none select-none items-center py-2.5", className)}
		>
			<BaseSlider.Control className="relative flex w-full items-center">
				<BaseSlider.Track className="relative h-2 w-full grow overflow-hidden rounded-full bg-zinc-800/90 shadow-inner">
					<BaseSlider.Indicator className="rounded-full bg-primary" />
				</BaseSlider.Track>
				{currentValues.length > 1 ? (
					<>
						<BaseSlider.Thumb
							key="thumb-min"
							index={0}
							className="block size-4.5 rounded-full border-2 border-primary bg-zinc-950 shadow-md ring-offset-background transition-shadow hover:ring-2 hover:ring-primary/40 focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 active:ring-3 active:ring-primary/60 disabled:pointer-events-none disabled:opacity-50"
						/>
						<BaseSlider.Thumb
							key="thumb-max"
							index={1}
							className="block size-4.5 rounded-full border-2 border-primary bg-zinc-950 shadow-md ring-offset-background transition-shadow hover:ring-2 hover:ring-primary/40 focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 active:ring-3 active:ring-primary/60 disabled:pointer-events-none disabled:opacity-50"
						/>
					</>
				) : (
					<BaseSlider.Thumb
						key="thumb-single"
						className="block size-4.5 rounded-full border-2 border-primary bg-zinc-950 shadow-md ring-offset-background transition-shadow hover:ring-2 hover:ring-primary/40 focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-primary focus-visible:ring-offset-2 active:ring-3 active:ring-primary/60 disabled:pointer-events-none disabled:opacity-50"
					/>
				)}
			</BaseSlider.Control>
		</BaseSlider.Root>
	)
}

Slider.displayName = "Slider"

export default memo(Slider)
