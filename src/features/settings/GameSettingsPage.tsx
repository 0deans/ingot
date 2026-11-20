import GameDataSyncCard from "@/features/settings/components/GameDataSyncCard"
import JavaMemoryCard from "@/features/settings/components/JavaMemoryCard"
import WindowDisplayCard from "@/features/settings/components/WindowDisplayCard"

const GameSettingsPage = () => {
	return (
		<div className="flex flex-col gap-5">
			<JavaMemoryCard />
			<WindowDisplayCard />
			<GameDataSyncCard />
		</div>
	)
}

export default GameSettingsPage
