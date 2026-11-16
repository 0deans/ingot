import { Link } from "@tanstack/react-router"
import { ChevronDown, Copy, Minus, Square, X } from "lucide-react"
import { AnimatePresence, LayoutGroup, motion } from "motion/react"
import { useCallback, useEffect, useState, type MouseEvent } from "react"
import { getVisibleNavItems, isNavItemActive, type NavItem } from "@/features/shell/navigation"
import {
	closeMainWindow,
	isTauriRuntime,
	minimizeMainWindow,
	queryMainWindowMaximized,
	toggleMaximizeMainWindow,
} from "@/shared/lib/tauriCommands"
import { subscribeWindowResized } from "@/shared/lib/tauriEvents"
import { cn } from "@/shared/lib/utils"
import { getAccountInitials, type AccountProfile } from "@/shared/stores/useAccountStore"
import { type NavVariant } from "@/shared/stores/useLauncherPrefsStore"

interface WindowsTitlebarProps {
	navVariant: NavVariant
	currentPath: string
	activeProcessesCount: number
	totalProcessRamMb: number
	processesOpen: boolean
	accountOpen: boolean
	activeAccount: AccountProfile
	onToggleProcesses: (event: MouseEvent<HTMLButtonElement>) => void
	onToggleAccount: (event: MouseEvent<HTMLButtonElement>) => void
}

const WindowsTitlebar = ({
	navVariant,
	currentPath,
	activeProcessesCount,
	totalProcessRamMb,
	processesOpen,
	accountOpen,
	activeAccount,
	onToggleProcesses,
	onToggleAccount,
}: WindowsTitlebarProps) => {
	const [isMaximized, setIsMaximized] = useState(false)

	useEffect(() => {
		if (!isTauriRuntime()) {
			return
		}
		let active = true
		let unlisten: (() => void) | undefined

		const syncMaximized = async () => {
			try {
				const maximized = await queryMainWindowMaximized()
				if (active) {
					setIsMaximized(maximized)
				}
			} catch (error: unknown) {
				console.error("Failed to query window maximized state:", error)
			}
		}

		void syncMaximized()

		void subscribeWindowResized(() => {
			void syncMaximized()
		})
			.then((dispose) => {
				if (active) {
					unlisten = dispose
				} else {
					dispose()
				}
			})
			.catch((error: unknown) => {
				console.error("Failed to subscribe to window resize events:", error)
			})

		return () => {
			active = false
			if (unlisten) {
				unlisten()
			}
		}
	}, [])

	const handleMinimize = useCallback(() => {
		void minimizeMainWindow().catch((error: unknown) => {
			console.error("Failed to minimize window:", error)
		})
	}, [])

	const handleToggleMaximize = useCallback(() => {
		void toggleMaximizeMainWindow().catch((error: unknown) => {
			console.error("Failed to toggle maximize window:", error)
		})
	}, [])

	const handleClose = useCallback(() => {
		void closeMainWindow().catch((error: unknown) => {
			console.error("Failed to close window:", error)
		})
	}, [])

	const isCapsule = navVariant === "capsule"
	const capsuleItems: NavItem[] = getVisibleNavItems(import.meta.env.DEV)
	const hasRunning = activeProcessesCount > 0
	const totalRamLabel =
		totalProcessRamMb >= 1024
			? `${(totalProcessRamMb / 1024).toFixed(1)} GB`
			: `${totalProcessRamMb} MB`
	const statusLabel = hasRunning ? `${activeProcessesCount} Active` : "Idle"
	const versionVisibilityClass = isCapsule ? "hidden md:inline-block" : "inline-block"

	return (
		<header
			data-tauri-drag-region
			className="relative z-40 flex h-10 shrink-0 items-center justify-between gap-2 border-b border-border bg-sidebar/85 pr-0 pl-3 backdrop-blur-xl select-none"
		>
			<div data-tauri-drag-region className="flex shrink-0 items-center gap-2">
				<img
					src="/ingot.svg"
					alt="Ingot logo"
					draggable={false}
					className="pointer-events-none size-5 shrink-0 object-contain"
				/>
				<span className="pointer-events-none text-[12.5px] font-semibold tracking-tight text-foreground">
					Ingot
				</span>
				<span
					className={cn(
						"pointer-events-none rounded-full border border-border bg-foreground/5 px-1.5 py-0.5 text-[9.5px] font-medium text-muted-foreground",
						versionVisibilityClass,
					)}
				>
					v{APP_VERSION}
				</span>
			</div>

			<div
				data-tauri-drag-region
				className="@container flex min-w-0 flex-1 items-center justify-center overflow-hidden px-1"
			>
				<AnimatePresence initial={false}>
					{isCapsule ? (
						<LayoutGroup id="windows-titlebar-capsule">
							<motion.nav
								key="top-capsule-nav"
								aria-label="Primary Navigation"
								initial={{ opacity: 0, scale: 0.96, y: -3 }}
								animate={{ opacity: 1, scale: 1, y: 0 }}
								exit={{ opacity: 0, scale: 0.96, y: -3 }}
								transition={{ duration: 0.22, ease: [0.22, 1, 0.36, 1] }}
								className="no-scrollbar flex max-w-full min-w-0 items-center gap-0.5 overflow-x-auto rounded-full border border-border bg-foreground/5 p-0.5 backdrop-blur-md"
							>
								{capsuleItems.map((item) => {
									const Icon = item.icon
									const isActive = isNavItemActive(currentPath, item.path)
									return (
										<Link
											key={item.path}
											to={item.path}
											title={item.label}
											aria-label={item.label}
											className={cn(
												"relative flex shrink-0 items-center justify-center gap-1 rounded-full px-1.5 py-1 text-[11.5px] font-medium @2xl:px-2 @3xl:gap-1.5 @3xl:px-2.5",
												isActive
													? "text-foreground"
													: "text-muted-foreground hover:bg-foreground/6 hover:text-foreground",
											)}
										>
											{isActive ? (
												<motion.span
													layoutId="capsule-active-pill"
													transition={{
														type: "spring",
														stiffness: 440,
														damping: 34,
														mass: 0.85,
													}}
													className="absolute inset-0 rounded-full bg-foreground/12 shadow-xs ring-1 ring-border"
												/>
											) : null}
											<span className="relative z-10 flex shrink-0 items-center justify-center">
												<Icon className={cn("size-3.5", isActive && "text-primary")} />
												{item.badge ? (
													<span
														aria-hidden="true"
														className="absolute -top-0.5 -right-0.5 size-1.5 rounded-full bg-primary ring-1 ring-sidebar/90 @2xl:hidden"
													/>
												) : null}
											</span>
											<span className="relative z-10 hidden whitespace-nowrap @2xl:inline">
												{item.shortLabel}
											</span>
											{item.badge ? (
												<>
													<span
														aria-hidden="true"
														className="relative z-10 hidden size-1.5 shrink-0 rounded-full bg-primary @2xl:inline-block @3xl:hidden"
													/>
													<span className="py-0.2 relative z-10 hidden rounded-full bg-primary/20 px-1.5 text-[9px] font-semibold text-primary @3xl:inline-flex">
														{item.badge}
													</span>
												</>
											) : null}
										</Link>
									)
								})}
							</motion.nav>
						</LayoutGroup>
					) : null}
				</AnimatePresence>
			</div>

			<div data-tauri-drag-region className="flex shrink-0 items-center gap-1.5">
				<AnimatePresence initial={false}>
					{isCapsule ? (
						<motion.button
							key="capsule-account-trigger"
							type="button"
							aria-expanded={accountOpen}
							aria-haspopup="dialog"
							title={`${activeAccount.username} (${activeAccount.type})`}
							onClick={onToggleAccount}
							initial={{ opacity: 0, scale: 0.94 }}
							animate={{ opacity: 1, scale: 1 }}
							exit={{ opacity: 0, scale: 0.94 }}
							transition={{ duration: 0.2, ease: [0.22, 1, 0.36, 1] }}
							className="flex h-7 cursor-pointer items-center gap-1 rounded-full border border-border bg-foreground/5 p-1 text-[11.5px] font-medium text-foreground transition-colors hover:bg-foreground/10 xl:gap-1.5 xl:pr-2 xl:pl-1"
						>
							<span
								className={cn(
									"flex size-5 shrink-0 items-center justify-center rounded-full bg-linear-to-br text-[9px] leading-none font-bold tracking-tight text-white",
									activeAccount.avatarBg,
								)}
							>
								{getAccountInitials(activeAccount.username)}
							</span>
							<span className="hidden max-w-24 truncate xl:inline">{activeAccount.username}</span>
							<ChevronDown className="size-3 text-muted-foreground" />
						</motion.button>
					) : null}
				</AnimatePresence>

				<button
					type="button"
					aria-expanded={processesOpen}
					aria-haspopup="dialog"
					onClick={onToggleProcesses}
					className={cn(
						"flex h-7 cursor-pointer items-center gap-1.5 rounded-full border px-2.5 text-[11px] font-medium whitespace-nowrap transition-colors",
						processesOpen
							? "border-primary/40 bg-primary/18 text-primary"
							: "border-border bg-foreground/5 text-foreground hover:bg-foreground/10",
					)}
				>
					<span
						className={cn(
							"size-1.5 rounded-full",
							hasRunning ? "animate-pulse bg-primary" : "bg-muted-foreground",
						)}
					/>
					<span className="tabular-nums">
						{statusLabel}
						{hasRunning ? <span className="hidden md:inline"> · {totalRamLabel}</span> : null}
					</span>
				</button>

				<div className="ml-1 flex h-10 items-center">
					<button
						type="button"
						onClick={handleMinimize}
						aria-label="Minimize window"
						className="flex h-10 w-10 cursor-pointer items-center justify-center text-muted-foreground transition-colors hover:bg-foreground/8 hover:text-foreground"
					>
						<Minus className="size-3.5" />
					</button>
					<button
						type="button"
						onClick={handleToggleMaximize}
						aria-label={isMaximized ? "Restore window" : "Maximize window"}
						className="flex h-10 w-10 cursor-pointer items-center justify-center text-muted-foreground transition-colors hover:bg-foreground/8 hover:text-foreground"
					>
						{isMaximized ? <Copy className="size-3" /> : <Square className="size-3" />}
					</button>
					<button
						type="button"
						onClick={handleClose}
						aria-label="Close window"
						className="flex h-10 w-10 cursor-pointer items-center justify-center text-muted-foreground transition-colors hover:bg-rose-600 hover:text-white"
					>
						<X className="size-3.5" />
					</button>
				</div>
			</div>
		</header>
	)
}

export default WindowsTitlebar
