import { FolderDown, Plus, Search } from "lucide-react"
import { memo } from "react"
import { useTranslation } from "react-i18next"
import { Button } from "@/components/ui/button"
import { InputGroup, InputGroupAddon, InputGroupInput } from "@/components/ui/input-group"

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
	const { t } = useTranslation()

	return (
		<div className="flex items-center justify-between gap-4">
			<InputGroup className="max-w-sm flex-1">
				<InputGroupAddon>
					<Search />
				</InputGroupAddon>
				<InputGroupInput
					value={searchQuery}
					onChange={(e) => onSearchChange(e.target.value)}
					placeholder={t("instances.searchPlaceholder")}
				/>
			</InputGroup>

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
