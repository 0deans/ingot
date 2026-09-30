import {
	type AnimateLayoutChanges,
	defaultAnimateLayoutChanges,
	useSortable,
} from "@dnd-kit/sortable"
import { CSS } from "@dnd-kit/utilities"
import { Check, Eye, GripVertical, Trash2 } from "lucide-react"
import { memo, type Ref } from "react"
import { useTranslation } from "react-i18next"
import { AccountTypeBadge } from "@/components/accounts/account-type-badge"
import SkinAvatar from "@/components/common/skin-avatar"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
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
	const { t } = useTranslation()
	return (
		<div
			ref={ref}
			id={id}
			className={cn(
				"relative flex flex-col gap-3 p-3.5 transition-colors sm:flex-row sm:items-center sm:justify-between",
				isOverlay
					? "cursor-grabbing rounded-lg bg-card shadow-2xl ring-2 ring-primary/60"
					: isDragging
						? "bg-card/40 opacity-25 ring-1 ring-dashed ring-primary/40 ring-inset"
						: "hover:bg-card/30",
			)}
		>
			<div className="flex min-w-0 items-center gap-2 sm:gap-3">
				{/* Drag handle */}
				<Tooltip>
					<TooltipTrigger
						render={
							<button
								type="button"
								aria-label={t("accounts.reorder", { name: account.username })}
								{...dragHandleProps}
								style={{ touchAction: "none" }}
								className={cn(
									"flex size-8 shrink-0 cursor-grab select-none items-center justify-center rounded-md text-muted-foreground/30 transition-colors hover:bg-muted/80 hover:text-foreground focus-visible:outline-hidden focus-visible:ring-1 focus-visible:ring-primary active:cursor-grabbing",
									(isDragging || isOverlay) && "cursor-grabbing bg-muted/80 text-primary",
								)}
							/>
						}
					>
						<GripVertical className="size-4" />
					</TooltipTrigger>
					<TooltipContent>{t("accounts.dragToReorder")}</TooltipContent>
				</Tooltip>

				{/* Avatar preview button */}
				<Tooltip>
					<TooltipTrigger
						render={
							<button
								type="button"
								onClick={() => onPreviewSkin(account)}
								className="group/avatar relative shrink-0 rounded-lg focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-primary"
								aria-label={t("accounts.clickToPreview")}
							/>
						}
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
					</TooltipTrigger>
					<TooltipContent>{t("accounts.clickToPreview")}</TooltipContent>
				</Tooltip>

				<div className="flex min-w-0 flex-col">
					<div className="flex flex-wrap items-center gap-2">
						<span className="truncate font-semibold text-foreground text-sm">
							{account.username}
						</span>
						<AccountTypeBadge type={account.accountType} />
						{account.isActive && (
							<Badge>
								<Check /> {t("common.active")}
							</Badge>
						)}
					</div>
					<span className="truncate font-mono text-[11px] text-muted-foreground">
						UUID: {account.uuid}
					</span>
				</div>
			</div>

			<div className="flex flex-wrap items-center justify-end gap-1.5 self-end sm:self-auto">
				<Tooltip>
					<TooltipTrigger
						render={
							<Button
								variant="outline"
								size="xs"
								onClick={() => onPreviewSkin(account)}
								className="gap-1.5 text-xs"
							/>
						}
					>
						<Eye className="size-3.5" />
						{t("accounts.skin")}
					</TooltipTrigger>
					<TooltipContent>{t("accounts.previewSkin")}</TooltipContent>
				</Tooltip>
				{!account.isActive && (
					<Button variant="outline" size="xs" onClick={() => onSetActive(account.id)}>
						{t("accounts.setAsActive")}
					</Button>
				)}
				<Tooltip>
					<TooltipTrigger
						render={
							<Button
								variant="ghost"
								size="icon-xs"
								className="text-muted-foreground hover:text-destructive"
								onClick={() => onRemove(account.id)}
								aria-label={t("accounts.removeAccount")}
							/>
						}
					>
						<Trash2 className="size-3.5" />
					</TooltipTrigger>
					<TooltipContent>{t("accounts.removeAccount")}</TooltipContent>
				</Tooltip>
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
