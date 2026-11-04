import { AlertTriangle, Info, Package, Trash2 } from "lucide-react"
import { memo, useCallback } from "react"
import { useTranslation } from "react-i18next"
import type { InstalledContent } from "@/bindings"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Checkbox } from "@/components/ui/checkbox"
import { Switch } from "@/components/ui/switch"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"

export interface ContentRowProps {
	item: InstalledContent
	isSelected: boolean
	isBusy: boolean
	isInstanceRunning: boolean
	hasDependencyIssue?: boolean
	dependencyIssueText?: string
	onToggleSelect: (fileName: string) => void
	onToggleEnabled: (fileName: string, enabled: boolean) => void
	onOpenDetails: (item: InstalledContent) => void
	onDelete: (fileName: string) => void
}

const ContentRow = ({
	item,
	isSelected,
	isBusy,
	isInstanceRunning,
	hasDependencyIssue,
	dependencyIssueText,
	onToggleSelect,
	onToggleEnabled,
	onOpenDetails,
	onDelete,
}: ContentRowProps) => {
	const { t } = useTranslation()

	const isEnabled = item.state === "enabled"
	const authorsText = item.metadata.authors.slice(0, 3).join(", ")
	const iconSrc = item.metadata.iconDataUrl

	const handleCheckedChange = useCallback(() => {
		onToggleSelect(item.fileName)
	}, [item.fileName, onToggleSelect])

	const handleSwitchChange = useCallback(
		(checked: boolean) => {
			onToggleEnabled(item.fileName, checked)
		},
		[item.fileName, onToggleEnabled],
	)

	const handleDetailsClick = useCallback(() => {
		onOpenDetails(item)
	}, [item, onOpenDetails])

	const handleDeleteClick = useCallback(() => {
		onDelete(item.fileName)
	}, [item.fileName, onDelete])

	return (
		<div
			className={`group flex items-center justify-between gap-3 rounded-lg border p-3 transition-colors ${
				isEnabled
					? "border-border/60 bg-card hover:bg-accent/40"
					: "border-border/30 bg-muted/30 opacity-75 hover:bg-muted/50"
			} ${isSelected ? "ring-2 ring-primary/60" : ""}`}
		>
			<div className="flex min-w-0 items-center gap-3">
				<Checkbox
					checked={isSelected}
					onCheckedChange={handleCheckedChange}
					disabled={isBusy}
					aria-label={item.metadata.name}
				/>

				<div className="relative flex size-10 shrink-0 items-center justify-center overflow-hidden rounded-md border border-border/50 bg-muted/60">
					{iconSrc ? (
						<img
							src={iconSrc}
							alt={item.metadata.name}
							className="size-full object-contain"
							loading="lazy"
						/>
					) : (
						<Package className="size-5 text-muted-foreground" />
					)}
				</div>

				<div className="min-w-0 flex-1">
					<div className="flex flex-wrap items-center gap-2">
						<span className="truncate text-sm font-medium text-foreground">
							{item.metadata.name}
						</span>
						{item.metadata.version && (
							<Badge variant="outline" className="text-xs">
								{item.metadata.version}
							</Badge>
						)}
						{hasDependencyIssue && (
							<Tooltip>
								<TooltipTrigger
									render={
										<span className="flex cursor-help items-center text-amber-500">
											<AlertTriangle className="size-4" />
										</span>
									}
								/>
								<TooltipContent>
									<p className="text-xs">
										{dependencyIssueText ?? t("instanceContent.dependencyIssue")}
									</p>
								</TooltipContent>
							</Tooltip>
						)}
					</div>
					<div className="flex items-center gap-2 text-xs text-muted-foreground">
						<span className="truncate">{item.fileName}</span>
						{authorsText && (
							<>
								<span>•</span>
								<span className="truncate">{authorsText}</span>
							</>
						)}
					</div>
				</div>
			</div>

			<div className="flex shrink-0 items-center gap-2">
				<Button
					variant="ghost"
					size="icon-sm"
					onClick={handleDetailsClick}
					aria-label={t("instanceContent.details")}
					className="text-muted-foreground hover:text-foreground"
				>
					<Info className="size-4" />
				</Button>

				{isInstanceRunning ? (
					<Tooltip>
						<TooltipTrigger
							render={
								<div>
									<Switch
										checked={isEnabled}
										onCheckedChange={handleSwitchChange}
										disabled
										aria-label={t("instanceContent.toggleModState", {
											name: item.metadata.name,
										})}
									/>
								</div>
							}
						/>
						<TooltipContent>
							<p className="text-xs">{t("instanceContent.readOnlyWhileRunning")}</p>
						</TooltipContent>
					</Tooltip>
				) : (
					<Switch
						checked={isEnabled}
						onCheckedChange={handleSwitchChange}
						disabled={isBusy}
						aria-label={t("instanceContent.toggleModState", {
							name: item.metadata.name,
						})}
					/>
				)}

				<Button
					variant="ghost"
					size="icon-sm"
					onClick={handleDeleteClick}
					disabled={isBusy || isInstanceRunning}
					className="text-muted-foreground hover:text-destructive"
					aria-label={t("common.delete")}
				>
					<Trash2 className="size-4" />
				</Button>
			</div>
		</div>
	)
}

ContentRow.displayName = "ContentRow"

export default memo(ContentRow)
