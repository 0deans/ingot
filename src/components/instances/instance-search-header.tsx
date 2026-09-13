import { FolderDown, Plus, Search } from "lucide-react"
import { memo } from "react"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"

interface InstanceSearchHeaderProps {
	searchQuery: string
	onSearchChange: (query: string) => void
	onOpenNewInstance: () => void
	onOpenImport?: () => void
}

const InstanceSearchHeader = ({
	searchQuery,
	onSearchChange,
	onOpenNewInstance,
	onOpenImport,
}: InstanceSearchHeaderProps) => {
	return (
		<div className="flex items-center justify-between gap-4">
			<div className="relative max-w-sm flex-1">
				<Search className="absolute top-1/2 left-3 size-4 -translate-y-1/2 text-muted-foreground" />
				<Input
					value={searchQuery}
					onChange={(e) => onSearchChange(e.target.value)}
					placeholder="Search instances..."
					className="border-border/50 bg-zinc-900/50 pl-9 focus-visible:ring-1"
				/>
			</div>

			<div className="flex items-center gap-2">
				{onOpenImport && (
					<Button variant="outline" onClick={onOpenImport} className="gap-2">
						<FolderDown className="size-4" />
						Import
					</Button>
				)}
				<Button onClick={onOpenNewInstance} className="gap-2">
					<Plus className="size-4" />
					New Instance
				</Button>
			</div>
		</div>
	)
}

InstanceSearchHeader.displayName = "InstanceSearchHeader"

export default memo(InstanceSearchHeader)
