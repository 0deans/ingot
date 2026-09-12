import {
	type AnimateLayoutChanges,
	defaultAnimateLayoutChanges,
	useSortable,
} from "@dnd-kit/sortable"
import { CSS } from "@dnd-kit/utilities"
import { Check, Eye, GripVertical, Trash2 } from "lucide-react"
import { memo, type Ref } from "react"
import { Button } from "@/components/ui/button"
import SkinAvatar from "@/components/ui/skin-avatar"
import { cn } from "@/lib/utils"
import type { AccountProfile } from "@/types/account"

export interface AccountItemProps {
	ref?: Ref<HTMLDivElement>
	id?: string
	account: AccountProfile
	onPreviewSkin: (account: AccountProfile) => void
	onSetActive: (id: string) => void
	onRemove: (id: string) => void
	isOverlay?: boolean
	dragHandleProps?: Record<string, unknown>
	isDragging?: boolean
}

export const AccountCardContent = memo(function AccountCardContent({
	ref,
	id,
	account,
	onPreviewSkin,
	onSetActive,
	onRemove,
	isOverlay = false,
	dragHandleProps,
	isDragging = false,
}: AccountItemProps) {
	return (
		<div
			ref={ref}
			id={id}
			className={cn(
				"relative flex flex-col gap-3 p-3.5 transition-colors sm:flex-row sm:items-center sm:justify-between",
				isOverlay
					? "cursor-grabbing rounded-lg bg-zinc-900 shadow-2xl ring-2 ring-primary/60"
					: isDragging
						? "bg-zinc-900/40 opacity-25 ring-1 ring-dashed ring-primary/40 ring-inset"
						: "hover:bg-zinc-900/30",
			)}
		>
			<div className="flex min-w-0 items-center gap-2 sm:gap-3">
				{/* Drag handle */}
				<button
					type="button"
					aria-label={`Reorder account ${account.username}`}
					{...dragHandleProps}
					style={{ touchAction: "none" }}
					className={cn(
						"flex size-8 shrink-0 cursor-grab select-none items-center justify-center rounded-md text-muted-foreground/30 transition-colors hover:bg-zinc-800/80 hover:text-foreground focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-primary active:cursor-grabbing",
						(isDragging || isOverlay) && "cursor-grabbing bg-zinc-800/80 text-primary",
					)}
					title="Drag to reorder account"
				>
					<GripVertical className="size-4" />
				</button>

				{/* Avatar preview button */}
				<button
					type="button"
					onClick={() => onPreviewSkin(account)}
					className="group/avatar relative shrink-0 rounded-lg focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-primary"
					title="Click to preview 3D skin"
				>
					<SkinAvatar
						username={account.username}
						skinUrl={account.skinUrl}
						size={38}
						className="transition-transform group-hover/avatar:scale-105"
					/>
					<div className="absolute inset-0 flex items-center justify-center rounded-lg bg-black/50 opacity-0 transition-opacity group-hover/avatar:opacity-100">
						<Eye className="size-3.5 text-white" />
					</div>
				</button>

				<div className="flex min-w-0 flex-col">
					<div className="flex flex-wrap items-center gap-2">
						<span className="truncate font-semibold text-foreground text-sm">
							{account.username}
						</span>
						{account.accountType === "ely" && (
							<span className="inline-flex items-center rounded-xs bg-emerald-500/15 px-1.5 py-0.5 font-medium text-[10px] text-emerald-400">
								Ely.by
							</span>
						)}
						{account.accountType === "offline" && (
							<span className="inline-flex items-center rounded-xs bg-zinc-800 px-1.5 py-0.5 font-medium text-[10px] text-zinc-400">
								Offline
							</span>
						)}
						{account.isActive && (
							<span className="inline-flex items-center gap-1 rounded-xs bg-primary/20 px-1.5 py-0.5 font-medium text-[10px] text-primary">
								<Check className="size-2.5" /> Active
							</span>
						)}
					</div>
					<span className="truncate font-mono text-[11px] text-muted-foreground">
						UUID: {account.uuid}
					</span>
				</div>
			</div>

			<div className="flex flex-wrap items-center justify-end gap-1.5 self-end sm:self-auto">
				<Button
					variant="outline"
					size="xs"
					onClick={() => onPreviewSkin(account)}
					className="gap-1.5 text-xs"
					title="Preview 3D skin"
				>
					<Eye className="size-3.5" />
					Skin
				</Button>
				{!account.isActive && (
					<Button variant="outline" size="xs" onClick={() => onSetActive(account.id)}>
						Set Active
					</Button>
				)}
				<Button
					variant="ghost"
					size="icon-xs"
					className="text-muted-foreground hover:text-destructive"
					onClick={() => onRemove(account.id)}
					title="Remove account"
				>
					<Trash2 className="size-3.5" />
				</Button>
			</div>
		</div>
	)
})

AccountCardContent.displayName = "AccountCardContent"

const animateLayoutChanges: AnimateLayoutChanges = (args) => {
	const { isSorting, wasDragging } = args
	if (isSorting || wasDragging) {
		return false
	}
	return defaultAnimateLayoutChanges(args)
}

export interface SortableAccountItemProps {
	account: AccountProfile
	onPreviewSkin: (account: AccountProfile) => void
	onSetActive: (id: string) => void
	onRemove: (id: string) => void
}

export const SortableAccountItem = memo(function SortableAccountItem({
	account,
	onPreviewSkin,
	onSetActive,
	onRemove,
}: SortableAccountItemProps) {
	const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
		id: account.id,
		animateLayoutChanges,
	})

	const style: React.CSSProperties = {
		transform: CSS.Translate.toString(transform),
		transition,
	}

	return (
		<div ref={setNodeRef} style={style}>
			<AccountCardContent
				id={account.id}
				account={account}
				onPreviewSkin={onPreviewSkin}
				onSetActive={onSetActive}
				onRemove={onRemove}
				isDragging={isDragging}
				dragHandleProps={{ ...attributes, ...listeners }}
			/>
		</div>
	)
})
