import { Link } from "@tanstack/react-router"
import { AnimatePresence, motion } from "motion/react"
import { createPortal } from "react-dom"
import { useTranslation } from "react-i18next"
import {
	PANEL_SPRING_EASE,
	type NavGroupSection,
	type PopupAnchor,
} from "@/features/shell/components/desktop/capsule-nav/capsuleNavModel"
import { isNavItemActive, NAV_GROUP_LABEL_KEYS, type NavGroupId } from "@/features/shell/navigation"
import { cn } from "@/shared/lib/utils"

interface CapsuleGroupMenuProps {
	activeGroupSection: NavGroupSection | null
	anchor: PopupAnchor | null
	panelHeight: number | null
	slideDirection: number
	currentPath: string
	onRegisterPopup: (node: HTMLDivElement | null) => void
	onRegisterContent: (groupId: NavGroupId, node: HTMLDivElement | null) => void
	onMouseEnter: () => void
	onMouseLeave: () => void
	onSelectItem: () => void
}

const contentVariants = {
	enter: (direction: number) => {
		return {
			opacity: 0,
			x: direction * 18,
		}
	},
	center: {
		opacity: 1,
		x: 0,
	},
	exit: (direction: number) => {
		return {
			opacity: 0,
			x: direction * -18,
		}
	},
}

const CapsuleGroupMenu = ({
	activeGroupSection,
	anchor,
	panelHeight,
	slideDirection,
	currentPath,
	onRegisterPopup,
	onRegisterContent,
	onMouseEnter,
	onMouseLeave,
	onSelectItem,
}: CapsuleGroupMenuProps) => {
	const { t } = useTranslation()

	const isMultiColumn = activeGroupSection !== null && activeGroupSection.items.length > 2
	const activeGroupLabel = activeGroupSection ? t(NAV_GROUP_LABEL_KEYS[activeGroupSection.id]) : ""

	return createPortal(
		<AnimatePresence>
			{activeGroupSection !== null && anchor !== null ? (
				<motion.div
					key="capsule-grouped-nav-popup"
					ref={onRegisterPopup}
					role="menu"
					aria-label={activeGroupLabel}
					onMouseEnter={onMouseEnter}
					onMouseLeave={onMouseLeave}
					initial={{
						opacity: 0,
						scale: 0.96,
						y: -4,
						x: anchor.left,
					}}
					animate={{
						opacity: 1,
						scale: 1,
						y: 0,
						x: anchor.left,
					}}
					exit={{
						opacity: 0,
						scale: 0.96,
						y: -4,
						transition: { duration: 0.14, ease: PANEL_SPRING_EASE },
					}}
					transition={{
						opacity: { duration: 0.16, ease: PANEL_SPRING_EASE },
						scale: { duration: 0.18, ease: PANEL_SPRING_EASE },
						y: { duration: 0.18, ease: PANEL_SPRING_EASE },
						x: { duration: 0.22, ease: PANEL_SPRING_EASE },
					}}
					style={{ top: anchor.top }}
					className="fixed left-0 z-50 pt-2 outline-none"
				>
					<motion.div
						animate={{
							width: activeGroupSection.panelWidth,
							height: panelHeight ?? "auto",
						}}
						transition={{ duration: 0.22, ease: PANEL_SPRING_EASE }}
						className="ingot-glass-elevated relative overflow-hidden rounded-xl bg-popover text-popover-foreground shadow-xl ring-1 ring-border"
					>
						<AnimatePresence initial={false} mode="popLayout" custom={slideDirection}>
							<motion.div
								key={activeGroupSection.id}
								ref={(node) => {
									onRegisterContent(activeGroupSection.id, node)
								}}
								custom={slideDirection}
								variants={contentVariants}
								initial="enter"
								animate="center"
								exit="exit"
								transition={{ duration: 0.18, ease: PANEL_SPRING_EASE }}
								style={{ width: activeGroupSection.panelWidth }}
								className="p-2"
							>
								<div className="mb-1.5 px-2 text-[10px] font-semibold tracking-wider text-muted-foreground uppercase">
									{activeGroupLabel}
								</div>

								<ul className={cn("grid gap-1", isMultiColumn ? "grid-cols-2" : "grid-cols-1")}>
									{activeGroupSection.items.map((item) => {
										const ItemIcon = item.icon
										const itemLabel = t(item.labelKey)
										const itemBadge = item.badgeKey ? t(item.badgeKey) : item.badge
										const itemActive = isNavItemActive(currentPath, item.path)

										return (
											<li key={item.path}>
												<Link
													to={item.path}
													role="menuitem"
													onClick={onSelectItem}
													className={cn(
														"group/link flex items-center gap-2.5 rounded-lg px-2 py-1.5 text-[12px] font-medium outline-none",
														itemActive
															? "bg-primary/14 text-foreground ring-1 ring-primary/30"
															: "text-muted-foreground hover:bg-foreground/7 hover:text-foreground focus-visible:bg-foreground/7 focus-visible:text-foreground",
													)}
												>
													<span
														className={cn(
															"flex size-7 shrink-0 items-center justify-center rounded-md border",
															itemActive
																? "border-primary/30 bg-primary/15 text-primary"
																: "border-border/70 bg-foreground/5 text-muted-foreground group-hover/link:text-foreground",
														)}
													>
														<ItemIcon className="size-3.5" />
													</span>

													<span className="min-w-0 flex-1 truncate">{itemLabel}</span>

													{itemBadge ? (
														<span className="shrink-0 rounded-full bg-primary/20 px-1.5 py-0.5 text-[9px] font-semibold text-primary">
															{itemBadge}
														</span>
													) : null}
												</Link>
											</li>
										)
									})}
								</ul>
							</motion.div>
						</AnimatePresence>
					</motion.div>
				</motion.div>
			) : null}
		</AnimatePresence>,
		document.body,
	)
}

export default CapsuleGroupMenu
