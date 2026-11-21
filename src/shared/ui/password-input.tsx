import { Eye, EyeOff } from "lucide-react"
import { type ComponentProps, useState } from "react"
import { cn } from "@/shared/lib/utils"
import { Input } from "@/shared/ui/input"

interface PasswordInputProps extends Omit<ComponentProps<"input">, "type"> {
	showLabel?: string
	hideLabel?: string
}

const PasswordInput = ({
	className,
	showLabel = "Show password",
	hideLabel = "Hide password",
	disabled,
	ref,
	...props
}: PasswordInputProps) => {
	const [isVisible, setIsVisible] = useState(false)

	const toggleLabel = isVisible ? hideLabel : showLabel

	return (
		<div className="relative">
			<Input
				ref={ref}
				type={isVisible ? "text" : "password"}
				disabled={disabled}
				className={cn("pr-10", className)}
				{...props}
			/>
			<div className="absolute inset-y-0 right-1 flex items-center">
				<button
					type="button"
					disabled={disabled}
					aria-label={toggleLabel}
					aria-pressed={isVisible}
					onMouseDown={(event) => {
						event.preventDefault()
					}}
					onClick={() => {
						setIsVisible((prev) => {
							return !prev
						})
					}}
					className="flex size-7 cursor-pointer items-center justify-center rounded-md text-muted-foreground transition-colors select-none hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none disabled:pointer-events-none disabled:opacity-50"
				>
					{isVisible ? <EyeOff className="size-4" /> : <Eye className="size-4" />}
				</button>
			</div>
		</div>
	)
}

export { PasswordInput }
export default PasswordInput
