import { Folder, FolderOpen } from "lucide-react"
import { memo, useMemo, useState } from "react"
import { useTranslation } from "react-i18next"
import type { ExportPreviewEntry } from "@/bindings"
import { Button } from "@/components/ui/button"
import { Checkbox } from "@/components/ui/checkbox"
import { ScrollArea } from "@/components/ui/scroll-area"
import { formatBytes } from "@/lib/format"

interface ExportFileTreeProps {
	entries: ExportPreviewEntry[]
	selectedPaths: Set<string>
	onTogglePath: (path: string) => void
	onToggleAll: (selectAll: boolean) => void
}

interface FileGroup {
	name: string
	entries: ExportPreviewEntry[]
}

const ExportFileTree = ({
	entries,
	selectedPaths,
	onTogglePath,
	onToggleAll,
}: ExportFileTreeProps) => {
	const { t } = useTranslation()
	const [collapsedFolders, setCollapsedFolders] = useState<Set<string>>(() => {
		return new Set<string>()
	})

	const groups = useMemo(() => {
		const map = new Map<string, ExportPreviewEntry[]>()

		for (const entry of entries) {
			const slashIdx = entry.path.indexOf("/")
			const folderName = slashIdx > 0 ? entry.path.slice(0, slashIdx) : "root"
			const existing = map.get(folderName)
			if (existing) {
				existing.push(entry)
			} else {
				map.set(folderName, [entry])
			}
		}

		const result: FileGroup[] = []
		for (const [name, groupEntries] of map.entries()) {
			result.push({ name, entries: groupEntries })
		}
		result.sort((a, b) => {
			if (a.name === "root") return 1
			if (b.name === "root") return -1
			return a.name.localeCompare(b.name)
		})
		return result
	}, [entries])

	const toggleFolderCollapse = (folderName: string) => {
		setCollapsedFolders((prev) => {
			const next = new Set(prev)
			if (next.has(folderName)) {
				next.delete(folderName)
			} else {
				next.add(folderName)
			}
			return next
		})
	}

	const allSelected = entries.length > 0 && selectedPaths.size === entries.length

	return (
		<div className="flex flex-col gap-2 rounded-md border border-border bg-card/40 p-2">
			<div className="flex items-center justify-between border-b border-border/60 pb-2 text-xs">
				<span className="text-muted-foreground">
					{t("exportInstance.selectedCount", {
						count: selectedPaths.size,
						total: entries.length,
					})}
				</span>
				<div className="flex items-center gap-2">
					<Button
						type="button"
						variant="ghost"
						size="sm"
						className="h-7 text-xs"
						onClick={() => {
							onToggleAll(!allSelected)
						}}
					>
						{allSelected ? t("exportInstance.deselectAll") : t("exportInstance.selectAll")}
					</Button>
				</div>
			</div>

			<ScrollArea className="h-72 scroll-fade-y pr-2">
				<div className="flex flex-col gap-3 py-1">
					{groups.map((group) => {
						const isCollapsed = collapsedFolders.has(group.name)
						const selectedInGroup = group.entries.filter((e) => {
							return selectedPaths.has(e.path)
						}).length

						return (
							<div key={group.name} className="flex flex-col gap-1">
								<div className="flex items-center justify-between rounded px-1.5 py-1 text-xs hover:bg-muted/40">
									<button
										type="button"
										className="flex flex-1 cursor-pointer items-center gap-1.5 text-left font-medium text-foreground"
										onClick={() => {
											toggleFolderCollapse(group.name)
										}}
									>
										{isCollapsed ? (
											<Folder className="size-3.5 text-primary" />
										) : (
											<FolderOpen className="size-3.5 text-primary" />
										)}
										<span>
											{group.name === "root" ? t("exportInstance.rootFiles") : group.name}
										</span>
										<span className="text-xs text-muted-foreground">
											({selectedInGroup}/{group.entries.length})
										</span>
									</button>
								</div>

								{!isCollapsed && (
									<div className="ml-4 flex flex-col gap-1 border-l border-border/50 pl-2">
										{group.entries.map((entry) => {
											const isSelected = selectedPaths.has(entry.path)
											const filename = entry.path.slice(entry.path.lastIndexOf("/") + 1)

											return (
												<label
													key={entry.path}
													className="flex cursor-pointer items-center justify-between rounded px-1.5 py-0.5 text-xs hover:bg-muted/50"
												>
													<div className="flex items-center gap-2 overflow-hidden pr-2">
														<Checkbox
															checked={isSelected}
															onCheckedChange={() => {
																onTogglePath(entry.path)
															}}
															aria-label={entry.path}
														/>
														<span
															className="truncate font-mono text-foreground/90"
															title={entry.path}
														>
															{filename}
														</span>
													</div>
													<span className="shrink-0 text-muted-foreground">
														{formatBytes(entry.sizeBytes)}
													</span>
												</label>
											)
										})}
									</div>
								)}
							</div>
						)
					})}
				</div>
			</ScrollArea>
		</div>
	)
}

ExportFileTree.displayName = "ExportFileTree"

export default memo(ExportFileTree)
