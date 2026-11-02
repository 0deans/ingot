import { ArrowUpDown, Check, FolderDown, Palette, Plus, Search, X } from "lucide-react"
import { memo } from "react"
import { useTranslation } from "react-i18next"
import type { InstanceColorTag } from "@/bindings"
import { COLOR_TAGS, getColorTagDefinition } from "@/components/instances/instance-icon"
import { Button } from "@/components/ui/button"
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuGroup,
	DropdownMenuItem,
	DropdownMenuLabel,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { InputGroup, InputGroupAddon, InputGroupInput } from "@/components/ui/input-group"

export type InstanceSortOption = "lastPlayed" | "playtime" | "name" | "createdAt"

interface InstanceSearchHeaderProps {
	searchQuery: string
	onSearchChange: (query: string) => void
	sortBy: InstanceSortOption
	onSortChange: (sort: InstanceSortOption) => void
	selectedColorTag: InstanceColorTag | null
	onColorTagChange: (colorTag: InstanceColorTag | null) => void
	onOpenNewInstance: () => void
	onOpenImport?: () => void
}

const SORT_OPTIONS: readonly { id: InstanceSortOption; labelKey: string; defaultLabel: string }[] =
	[
		{
			id: "lastPlayed",
			labelKey: "instances.sort.lastPlayed",
			defaultLabel: "Last Played",
		},
		{
			id: "playtime",
			labelKey: "instances.sort.playtime",
			defaultLabel: "Total Playtime",
		},
		{
			id: "name",
			labelKey: "instances.sort.name",
			defaultLabel: "Name (A–Z)",
		},
		{
			id: "createdAt",
			labelKey: "instances.sort.createdAt",
			defaultLabel: "Recently Created",
		},
	]

const InstanceSearchHeader = ({
	searchQuery,
	onSearchChange,
	sortBy,
	onSortChange,
	selectedColorTag,
	onColorTagChange,
	onOpenNewInstance,
	onOpenImport,
}: InstanceSearchHeaderProps) => {
	const { t } = useTranslation()
	const activeSort =
		SORT_OPTIONS.find((opt) => {
			return opt.id === sortBy
		}) ?? SORT_OPTIONS[0]
	const activeColorDef = getColorTagDefinition(selectedColorTag)

	return (
		<div className="flex flex-wrap items-center justify-between gap-3">
			<div className="flex flex-1 flex-wrap items-center gap-2">
				<InputGroup className="max-w-sm min-w-[200px] flex-1">
					<InputGroupAddon>
						<Search />
					</InputGroupAddon>
					<InputGroupInput
						value={searchQuery}
						onChange={(e) => onSearchChange(e.target.value)}
						placeholder={t("instances.searchPlaceholder")}
					/>
				</InputGroup>

				{/* Sort Selector */}
				<DropdownMenu>
					<DropdownMenuTrigger
						render={
							<Button variant="outline" size="sm" className="h-9 gap-1.5 text-xs">
								<ArrowUpDown className="size-3.5 text-muted-foreground" />
								<span>{t(activeSort.labelKey, { defaultValue: activeSort.defaultLabel })}</span>
							</Button>
						}
					/>
					<DropdownMenuContent align="start" className="w-48">
						<DropdownMenuGroup>
							<DropdownMenuLabel className="text-3xs font-semibold tracking-wider text-muted-foreground uppercase">
								{t("instances.sort.label", { defaultValue: "Sort By" })}
							</DropdownMenuLabel>
							{SORT_OPTIONS.map((option) => {
								const isSelected = option.id === sortBy
								return (
									<DropdownMenuItem
										key={option.id}
										onClick={() => onSortChange(option.id)}
										className="cursor-pointer justify-between gap-2 text-xs"
									>
										<span>{t(option.labelKey, { defaultValue: option.defaultLabel })}</span>
										{isSelected && <Check className="size-3.5 text-primary" />}
									</DropdownMenuItem>
								)
							})}
						</DropdownMenuGroup>
					</DropdownMenuContent>
				</DropdownMenu>

				{/* Color Tag Filter */}
				<DropdownMenu>
					<DropdownMenuTrigger
						render={
							<Button
								variant="outline"
								size="sm"
								className={`h-9 gap-1.5 text-xs ${activeColorDef ? activeColorDef.badgeClass : ""}`}
							>
								{activeColorDef ? (
									<span className={`size-2.5 rounded-full ${activeColorDef.dotClass}`} />
								) : (
									<Palette className="size-3.5 text-muted-foreground" />
								)}
								<span>
									{activeColorDef
										? t(activeColorDef.labelKey, { defaultValue: activeColorDef.defaultLabel })
										: t("instances.colors.filter", { defaultValue: "Color" })}
								</span>
							</Button>
						}
					/>
					<DropdownMenuContent align="start" className="w-44">
						<DropdownMenuGroup>
							<DropdownMenuLabel className="text-3xs font-semibold tracking-wider text-muted-foreground uppercase">
								{t("instances.colors.filterBy", { defaultValue: "Filter by Color" })}
							</DropdownMenuLabel>
							<DropdownMenuItem
								onClick={() => onColorTagChange(null)}
								className="cursor-pointer justify-between gap-2 text-xs"
							>
								<span className="flex items-center gap-2 text-muted-foreground">
									<X className="size-3.5" />
									{t("instances.colors.all", { defaultValue: "All Colors" })}
								</span>
								{!selectedColorTag && <Check className="size-3.5 text-primary" />}
							</DropdownMenuItem>
							<DropdownMenuSeparator />
							{COLOR_TAGS.map((tag) => {
								const isSelected = selectedColorTag === tag.id
								return (
									<DropdownMenuItem
										key={tag.id}
										onClick={() => onColorTagChange(isSelected ? null : tag.id)}
										className="cursor-pointer justify-between gap-2 text-xs"
									>
										<span className="flex items-center gap-2">
											<span className={`size-2.5 rounded-full ${tag.dotClass}`} />
											{t(tag.labelKey, { defaultValue: tag.defaultLabel })}
										</span>
										{isSelected && <Check className="size-3.5 text-primary" />}
									</DropdownMenuItem>
								)
							})}
						</DropdownMenuGroup>
					</DropdownMenuContent>
				</DropdownMenu>
			</div>

			<div className="flex items-center gap-2">
				{onOpenImport && (
					<Button variant="outline" onClick={onOpenImport} className="gap-2">
						<FolderDown className="size-4" />
						{t("instances.import")}
					</Button>
				)}
				<Button onClick={onOpenNewInstance} className="gap-2">
					<Plus className="size-4" />
					{t("instances.newInstance")}
				</Button>
			</div>
		</div>
	)
}

InstanceSearchHeader.displayName = "InstanceSearchHeader"

export default memo(InstanceSearchHeader)
