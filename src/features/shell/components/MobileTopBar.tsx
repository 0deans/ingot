import { Boxes } from "lucide-react"
import { type MouseEvent } from "react"
import { findNavItem } from "@/features/shell/navigation"
import { cn } from "@/shared/lib/utils"
import { getAccountInitials, type AccountProfile } from "@/shared/stores/useAccountStore"

interface MobileTopBarProps {
	currentPath: string
	activeProcessesCount: number
	activeAccount: AccountProfile
	onToggleProcesses: (event: MouseEvent<HTMLButtonElement>) => void
	onToggleAccount: (event: MouseEvent<HTMLButtonElement>) => void
}

const MobileTopBar = ({
	currentPath,
	activeProcessesCount,
	activeAccount,
	onToggleProcesses,
	onToggleAccount,
}: MobileTopBarProps) => {
	const currentItem = findNavItem(currentPath)

	return (
		<header className="relative z-30 flex shrink-0 items-center justify-between border-b border-border bg-sidebar/85 px-3.5 pt-[calc(0.5rem+env(safe-area-inset-top))] pb-2.5 backdrop-blur-xl">
			<div className="flex min-w-0 items-center gap-2.5">
				<div className="flex size-7 shrink-0 items-center justify-center rounded-xl bg-primary/18 text-primary ring-1 ring-primary/30">
					<Boxes className="size-3.5" />
				</div>
				<div className="min-w-0">
					<p className="truncate text-[13.5px] font-semibold text-foreground">
						{currentItem.label}
					</p>
					<p className="truncate text-[10px] text-muted-foreground">Ingot Launcher</p>
				</div>
			</div>

			<div className="flex shrink-0 items-center gap-2">
				<button
					type="button"
					onClick={onToggleProcesses}
					aria-label="Toggle active processes"
					className="flex h-8 cursor-pointer items-center gap-1.5 rounded-full border border-border bg-foreground/5 px-2.5 text-[11px] font-medium text-foreground transition-colors hover:bg-foreground/10"
				>
					<span
						className={cn(
							"size-1.5 rounded-full",
							activeProcessesCount > 0 ? "animate-pulse bg-primary" : "bg-muted-foreground",
						)}
					/>
					<span className="tabular-nums">{activeProcessesCount}</span>
				</button>

				<button
					type="button"
					onClick={onToggleAccount}
					aria-label={`Account ${activeAccount.username}`}
					className={cn(
						"flex size-8 cursor-pointer items-center justify-center rounded-full bg-linear-to-br text-[11px] font-bold text-white shadow-inner ring-1 ring-border",
						activeAccount.avatarBg,
					)}
				>
					{getAccountInitials(activeAccount.username)}
				</button>
			</div>
		</header>
	)
}

export default MobileTopBar
