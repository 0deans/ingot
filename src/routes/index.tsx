import { createFileRoute } from "@tanstack/react-router"
import { memo, useState } from "react"
import InstanceHero from "@/components/instances/instance-hero"
import InstanceSearchHeader from "@/components/instances/instance-search-header"
import NewInstanceDialog from "@/components/instances/new-instance-dialog"

const InstancesPage = () => {
	const [searchQuery, setSearchQuery] = useState("")
	const [isNewInstanceOpen, setIsNewInstanceOpen] = useState(false)

	const handleCreateInstance = (name: string) => {
		console.log("Created instance:", name)
	}

	const handlePlay = () => {
		console.log("Launching Minecraft instance...")
	}

	return (
		<div className="flex flex-1 flex-col">
			<InstanceSearchHeader
				searchQuery={searchQuery}
				onSearchChange={setSearchQuery}
				onOpenNewInstance={() => setIsNewInstanceOpen(true)}
			/>
			<InstanceHero
				name="Minecraft 1.21.4"
				version="1.21.4"
				loader="Fabric 0.16.9"
				memory="4.0 GB / 16.0 GB"
				javaVersion="Java 21 (Temurin)"
				onPlay={handlePlay}
			/>
			<NewInstanceDialog
				open={isNewInstanceOpen}
				onOpenChange={setIsNewInstanceOpen}
				onCreateInstance={handleCreateInstance}
			/>
		</div>
	)
}

InstancesPage.displayName = "InstancesPage"

const MemoizedInstancesPage = memo(InstancesPage)

export const Route = createFileRoute("/")({
	component: MemoizedInstancesPage,
})
