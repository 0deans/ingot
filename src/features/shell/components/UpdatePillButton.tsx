import { Sparkles } from "lucide-react"
import { AnimatePresence, motion } from "motion/react"
import type { MouseEvent } from "react"
import { useTranslation } from "react-i18next"
import { cn } from "@/shared/lib/utils"

interface UpdatePillButtonProps {
	visible: boolean
	version: string
	open: boolean
	size?: "sm" | "md"
	onClick: (event: MouseEvent<HTMLButtonElement>) => void
}

const EASE_OUT_EXPO: readonly [number, number, number, number] = [0.22, 1, 0.36, 1]

const UpdatePillButton = ({
	visible,
	version,
	open,
	size = "sm",
	onClick,
}: UpdatePillButtonProps) => {
	const { t } = useTranslation()

	return (
		<AnimatePresence initial={false}>
			{visible ? (
				<motion.div
					key="app-update-pill"
					initial={{ opacity: 0, scale: 0.86, width: 0 }}
					animate={{ opacity: 1, scale: 1, width: "auto" }}
					exit={{ opacity: 0, scale: 0.86, width: 0 }}
					transition={{ duration: 0.22, ease: EASE_OUT_EXPO }}
					className="flex shrink-0 items-center overflow-hidden"
				>
					<button
						type="button"
						aria-expanded={open}
						aria-haspopup="dialog"
						aria-label={t("shell.update.pillAria", { version })}
						onClick={onClick}
						className={cn(
							"flex cursor-pointer items-center gap-1.5 rounded-full border font-medium whitespace-nowrap transition-colors",
							size === "sm" ? "h-7 px-2.5 text-[11px]" : "h-8 px-2.5 text-[11px]",
							open
								? "border-primary/50 bg-primary/22 text-primary"
								: "border-primary/35 bg-primary/12 text-primary hover:bg-primary/20",
						)}
					>
						<Sparkles className="size-3.5 shrink-0" />
						<span className={size === "md" ? "xs:inline hidden" : "hidden sm:inline"}>
							v{version}
						</span>
					</button>
				</motion.div>
			) : null}
		</AnimatePresence>
	)
}

export default UpdatePillButton
