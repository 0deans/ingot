import type { MouseEvent } from "react"
import InstanceCard from "@/features/play/components/InstanceCard"
import type { GameInstance, InstanceSection } from "@/features/play/types"
import { cn } from "@/shared/lib/utils"

interface InstanceGroupProps {
	section: InstanceSection
	selectedId: string
	runningNames: Set<string>
	onSelect: (id: string) => void
	onCycleTag: (id: string) => void
	onOpenDetails: (id: string) => void
	onContextMenu: (instance: GameInstance, event: MouseEvent<HTMLDivElement>) => void
}

const InstanceGroup = ({
	section,
	selectedId,
	runningNames,
	onSelect,
	onCycleTag,
	onOpenDetails,
	onContextMenu,
}: InstanceGroupProps) => {
	return (
		<section className="flex flex-col gap-2">
			<div className="flex items-center gap-2 px-0.5">
				{section.dotClass ? (
					<span className={cn("size-2 shrink-0 rounded-full", section.dotClass)} />
				) : null}
				<h3 className="text-[11px] font-semibold tracking-wider text-muted-foreground uppercase">
					{section.title}
				</h3>
				<span className="rounded-full bg-foreground/6 px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground">
					{section.items.length}
				</span>
			</div>

			<div className="grid grid-cols-1 gap-2 sm:grid-cols-2 lg:grid-cols-3">
				{section.items.map((inst) => {
					return (
						<InstanceCard
							key={inst.id}
							instance={inst}
							isSelected={inst.id === selectedId}
							isRunning={runningNames.has(inst.name)}
							onSelect={onSelect}
							onCycleTag={onCycleTag}
							onOpenDetails={onOpenDetails}
							onContextMenu={(event) => {
								onContextMenu(inst, event)
							}}
						/>
					)
				})}
			</div>
		</section>
	)
}

export default InstanceGroup
