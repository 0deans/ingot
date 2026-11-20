import { Link } from "@tanstack/react-router"
import { X } from "lucide-react"
import { AnimatePresence, motion } from "motion/react"
import { useEffect } from "react"
import { useTranslation } from "react-i18next"
import { isNavItemActive, type AppRoutePath, type NavItem } from "@/features/shell/navigation"
import { cn } from "@/shared/lib/utils"

interface MobileMoreSheetProps {
	open: boolean
	currentPath: string
	items: NavItem[]
	onClose: () => void
	onNavigate?: (path: AppRoutePath) => void
}

const MobileMoreSheet = ({
	open,
	currentPath,
	items,
	onClose,
	onNavigate,
}: MobileMoreSheetProps) => {
	const { t } = useTranslation()

	useEffect(() => {
		if (!open) {
			return
		}
		const handleKeyDown = (event: KeyboardEvent) => {
			if (event.key === "Escape") {
				onClose()
			}
		}
		window.addEventListener("keydown", handleKeyDown)
		return () => {
			window.removeEventListener("keydown", handleKeyDown)
		}
	}, [open, onClose])

	return (
		<AnimatePresence>
			{open ? (
				<div className="fixed inset-0 z-50 flex items-end justify-center">
					<motion.button
						type="button"
						aria-label={t("shell.closeNavigationSheet")}
						initial={{ opacity: 0 }}
						animate={{ opacity: 1 }}
						exit={{ opacity: 0 }}
						onClick={onClose}
						className="fixed inset-0 cursor-default bg-black/55 backdrop-blur-xs"
					/>
					<motion.div
						role="dialog"
						aria-label={t("shell.moreSections")}
						initial={{ y: "100%" }}
						animate={{ y: 0 }}
						exit={{ y: "100%" }}
						transition={{ type: "spring", stiffness: 380, damping: 34 }}
						className="ingot-glass-elevated relative z-10 w-full rounded-t-3xl px-4 pt-3 pb-6 shadow-2xl"
					>
						<div className="mx-auto mb-3 h-1 w-10 rounded-full bg-foreground/20" />
						<div className="mb-3 flex items-center justify-between gap-3 px-1">
							<span className="truncate text-[13px] font-semibold text-foreground">
								{t("shell.moreSections")}
							</span>
							<button
								type="button"
								onClick={onClose}
								aria-label={t("shell.dismissNavigationSheet")}
								className="shrink-0 cursor-pointer rounded-lg p-1 text-muted-foreground hover:bg-foreground/8 hover:text-foreground"
							>
								<X className="size-4" />
							</button>
						</div>

						<div className="grid grid-cols-2 gap-2">
							{items.map((item, index) => {
								const Icon = item.icon
								const isActive = isNavItemActive(currentPath, item.path)
								const isLastOddItem = items.length % 2 === 1 && index === items.length - 1
								const badgeText = item.badgeKey ? t(item.badgeKey) : item.badge
								return (
									<Link
										key={item.path}
										to={item.path}
										onClick={() => {
											if (onNavigate) {
												onNavigate(item.path)
											}
											onClose()
										}}
										className={cn(
											"flex items-center gap-2.5 rounded-2xl border px-3 py-2.5",
											isLastOddItem && "col-span-2",
											isActive
												? "border-primary/40 bg-primary/14 text-foreground"
												: "border-border bg-foreground/4 text-foreground/85 hover:bg-foreground/8",
										)}
									>
										<Icon className="size-4 shrink-0 text-primary" />
										<span className="min-w-0 flex-1 truncate text-[12.5px] font-medium">
											{t(item.labelKey)}
										</span>
										{badgeText ? (
											<span className="shrink-0 rounded-full bg-primary/20 px-1.5 py-0.5 text-[10px] font-semibold text-primary">
												{badgeText}
											</span>
										) : null}
									</Link>
								)
							})}
						</div>
					</motion.div>
				</div>
			) : null}
		</AnimatePresence>
	)
}

export default MobileMoreSheet
