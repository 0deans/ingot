import { Check, Loader2, Plus, ShieldCheck, User, X } from "lucide-react"
import { AnimatePresence, motion } from "motion/react"
import { useEffect, type CSSProperties } from "react"
import { useTranslation } from "react-i18next"
import {
	ACCOUNT_TYPE_LABEL_KEYS,
	getAccountInitials,
	type UiAccountProfile,
} from "@/features/accounts/accountModel"
import { cn } from "@/shared/lib/utils"

export type AccountPopoverAnchor = "capsule" | "rail" | "sidebar" | "mobile"

export interface PopoverAnchorRect {
	top: number
	bottom: number
	left: number
	right: number
}

interface AccountSwitcherPopoverProps {
	open: boolean
	anchorMode: AccountPopoverAnchor
	anchorRect: PopoverAnchorRect | null
	accounts: UiAccountProfile[]
	activeAccountId: string | null
	isLoading?: boolean
	onClose: () => void
	onSelectAccount: (id: string) => void
	onAddAccount: () => void
}

const ANCHOR_POSITION_CLASS: Record<AccountPopoverAnchor, string> = {
	capsule: "top-12 right-4",
	rail: "bottom-2.5 left-18",
	sidebar: "bottom-16 left-2.5",
	mobile: "top-14 right-3",
}

export const resolveCenteredTitlebarPopoverStyle = (
	anchorRect: PopoverAnchorRect | null,
	maxPopoverWidth: number,
): CSSProperties | undefined => {
	if (!anchorRect || typeof window === "undefined") {
		return undefined
	}

	const viewportWidth = window.innerWidth
	const edgePadding = 12
	const popoverWidth = Math.min(maxPopoverWidth, viewportWidth - edgePadding * 2)
	const triggerCenterX = (anchorRect.left + anchorRect.right) / 2
	const desiredLeft = Math.round(triggerCenterX - popoverWidth / 2)
	const maxLeft = Math.max(edgePadding, viewportWidth - popoverWidth - edgePadding)

	return {
		top: Math.round(anchorRect.bottom + 8),
		left: Math.min(Math.max(edgePadding, desiredLeft), maxLeft),
	}
}

const resolveAccountPopoverStyle = (
	anchorMode: AccountPopoverAnchor,
	anchorRect: PopoverAnchorRect | null,
): CSSProperties | undefined => {
	if (!anchorRect || typeof window === "undefined") {
		return undefined
	}

	if (anchorMode === "capsule" || anchorMode === "mobile") {
		return resolveCenteredTitlebarPopoverStyle(anchorRect, 288)
	}

	const viewportHeight = window.innerHeight
	const edgePadding = 12

	if (anchorMode === "rail") {
		return {
			left: Math.round(anchorRect.right + 10),
			bottom: Math.max(edgePadding, Math.round(viewportHeight - anchorRect.bottom)),
		}
	}

	return {
		left: Math.max(10, Math.round(anchorRect.left)),
		bottom: Math.max(edgePadding, Math.round(viewportHeight - anchorRect.top + 8)),
	}
}

const AccountSwitcherPopover = ({
	open,
	anchorMode,
	anchorRect,
	accounts,
	activeAccountId,
	isLoading = false,
	onClose,
	onSelectAccount,
	onAddAccount,
}: AccountSwitcherPopoverProps) => {
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

	const anchoredStyle = resolveAccountPopoverStyle(anchorMode, anchorRect)
	const fallbackClass = anchoredStyle ? "" : ANCHOR_POSITION_CLASS[anchorMode]

	const renderAccountList = () => {
		if (isLoading && accounts.length === 0) {
			return (
				<div className="flex items-center justify-center gap-2 px-3 py-6 text-[12px] text-muted-foreground">
					<Loader2 className="size-3.5 animate-spin text-primary" />
					<span>{t("accounts.actions.loading")}</span>
				</div>
			)
		}

		if (accounts.length === 0) {
			return (
				<div className="flex flex-col items-center justify-center gap-1.5 px-3 py-5 text-center">
					<div className="flex size-8 items-center justify-center rounded-lg bg-foreground/6 text-muted-foreground">
						<User className="size-4" />
					</div>
					<p className="text-[12px] font-medium text-popover-foreground">
						{t("accounts.empty.title")}
					</p>
					<p className="text-[11px] leading-snug text-muted-foreground">
						{t("accounts.empty.description")}
					</p>
				</div>
			)
		}

		return accounts.map((account) => {
			const isActive = activeAccountId !== null && account.id === activeAccountId
			return (
				<button
					key={account.id}
					type="button"
					onClick={() => {
						onSelectAccount(account.id)
						onClose()
					}}
					className={cn(
						"flex w-full cursor-pointer items-center gap-2.5 rounded-xl px-2.5 py-2 text-left transition-colors",
						isActive
							? "bg-foreground/10 text-popover-foreground"
							: "text-popover-foreground/80 hover:bg-foreground/6",
					)}
				>
					<div
						className={cn(
							"flex size-7 shrink-0 items-center justify-center rounded-lg bg-linear-to-br text-[11px] font-bold text-white shadow-inner",
							account.avatarBg,
						)}
					>
						{getAccountInitials(account.username)}
					</div>
					<div className="min-w-0 flex-1">
						<div className="flex items-center gap-1.5">
							<span className="truncate text-[12.5px] font-medium">{account.username}</span>
							{account.type === "Microsoft" ? (
								<ShieldCheck className="size-3 shrink-0 text-primary" />
							) : null}
						</div>
						<span className="block truncate text-[10.5px] text-muted-foreground">
							{t(ACCOUNT_TYPE_LABEL_KEYS[account.type])} · {account.uuid.slice(0, 8)}
						</span>
					</div>
					{isActive ? <Check className="size-3.5 shrink-0 text-primary" /> : null}
				</button>
			)
		})
	}

	return (
		<AnimatePresence>
			{open ? (
				<div className="fixed inset-0 z-50">
					<button
						type="button"
						aria-label={t("shell.accountSwitcher.closeAria")}
						onClick={onClose}
						className="fixed inset-0 cursor-default bg-black/25"
					/>
					<motion.div
						role="dialog"
						aria-label={t("shell.accountSwitcher.title")}
						style={anchoredStyle}
						initial={{
							opacity: 0,
							scale: 0.96,
							y: anchorMode === "capsule" || anchorMode === "mobile" ? -8 : 8,
						}}
						animate={{ opacity: 1, scale: 1, y: 0 }}
						exit={{
							opacity: 0,
							scale: 0.96,
							y: anchorMode === "capsule" || anchorMode === "mobile" ? -8 : 8,
						}}
						transition={{ duration: 0.16, ease: "easeOut" }}
						className={cn(
							"ingot-glass-elevated fixed z-50 flex w-72 max-w-[calc(100vw-1.5rem)] flex-col overflow-hidden rounded-2xl",
							fallbackClass,
						)}
					>
						<div className="flex items-center justify-between border-b border-border px-3.5 py-2.5">
							<div className="flex items-center gap-2">
								<User className="size-3.5 text-primary" />
								<span className="text-[12.5px] font-semibold text-popover-foreground">
									{t("shell.accountSwitcher.title")}
								</span>
							</div>
							<button
								type="button"
								onClick={onClose}
								aria-label={t("shell.accountSwitcher.dismissAria")}
								className="rounded-md p-1 text-muted-foreground transition-colors hover:bg-foreground/8 hover:text-foreground"
							>
								<X className="size-3.5" />
							</button>
						</div>

						<div className="custom-scrollbar max-h-64 overflow-y-auto p-1.5">
							{renderAccountList()}
						</div>

						<div className="flex items-center justify-between gap-2 border-t border-border bg-foreground/2 px-2.5 py-2">
							<button
								type="button"
								onClick={() => {
									onAddAccount()
								}}
								className="flex w-full cursor-pointer items-center justify-center gap-1.5 rounded-lg bg-foreground/8 px-2.5 py-1.5 text-[11.5px] font-medium text-popover-foreground transition-colors hover:bg-foreground/14"
							>
								<Plus className="size-3.5" />
								<span>{t("shell.accountSwitcher.addAccount")}</span>
							</button>
						</div>
					</motion.div>
				</div>
			) : null}
		</AnimatePresence>
	)
}

export default AccountSwitcherPopover
