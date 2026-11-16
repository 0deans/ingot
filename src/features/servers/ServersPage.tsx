import { Plus } from "lucide-react"
import { useState } from "react"
import LocalServerCreatorModal from "@/features/servers/components/LocalServerCreatorModal"
import ServerRow, { type ServerEntry } from "@/features/servers/components/ServerRow"
import { selectActiveAccount, useAccountStore } from "@/shared/stores/useAccountStore"
import { useInstancesStore } from "@/shared/stores/useInstancesStore"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"
import { useProcessStore } from "@/shared/stores/useProcessStore"

const INITIAL_SERVERS: ServerEntry[] = [
	{
		id: "srv-ingot-smp",
		name: "Ingot Community SMP",
		address: "play.ingot-smp.net",
		motd: "Season 4 · Proximity Voice + Fresh Nether Reset",
		playersOnline: 48,
		playersMax: 120,
		pingMs: 18,
		version: "1.21.4",
		instanceName: "Fabulously Optimized",
		engine: "Java",
	},
	{
		id: "srv-horizon-lab",
		name: "Horizon Rust Staging Realm",
		address: "rust.ingot.gg:25575",
		motd: "Dedicated Vulkan client protocol · 60 TPS Stress Test",
		playersOnline: 19,
		playersMax: 200,
		pingMs: 24,
		version: "0.9.4-alpha",
		instanceName: "Horizon Engine Nightly",
		engine: "Horizon (Rust)",
	},
	{
		id: "srv-create-brass",
		name: "Brassworks Guild",
		address: "brass.guildcraft.org",
		motd: "Automated train logistics and space station docking",
		playersOnline: 14,
		playersMax: 40,
		pingMs: 42,
		version: "1.18.2",
		instanceName: "Create: Astral",
		engine: "Java",
	},
	{
		id: "srv-hypixel",
		name: "Hypixel Network",
		address: "mc.hypixel.net",
		motd: "SkyBlock, BedWars & Competitive Duels",
		playersOnline: 38410,
		playersMax: 100000,
		pingMs: 61,
		version: "1.8.9",
		instanceName: "Hypixel 1.8.9 PvP",
		engine: "Java",
	},
]

const ServersPage = () => {
	const selectOrCreateByName = useInstancesStore((s) => {
		return s.selectOrCreateByName
	})
	const launchInstance = useProcessStore((s) => {
		return s.launchInstance
	})
	const activeAccount = useAccountStore(selectActiveAccount)
	const ramGb = useLauncherPrefsStore((s) => {
		return s.ramGb
	})

	const [servers, setServers] = useState<ServerEntry[]>(INITIAL_SERVERS)
	const [creatorOpen, setCreatorOpen] = useState(false)

	const handleConnect = (server: ServerEntry) => {
		selectOrCreateByName({
			name: server.instanceName,
			version: server.version,
			loader: server.engine === "Horizon (Rust)" ? "Horizon Native" : "Fabric",
			engine: server.engine,
			colorTag: server.engine === "Horizon (Rust)" ? "rose" : "cyan",
		})
		const { instances, selectedId } = useInstancesStore.getState()
		const target = instances.find((inst) => {
			return inst.id === selectedId
		})
		if (target) {
			launchInstance({
				instance: target,
				username: activeAccount.username,
				ramGb,
			})
		}
	}

	const handleCreateServer = (created: ServerEntry) => {
		setServers((prev) => {
			return [created, ...prev]
		})
	}

	return (
		<div className="flex flex-col gap-4">
			<div className="flex flex-wrap items-center justify-between gap-3">
				<div>
					<h1 className="text-lg font-semibold tracking-tight text-foreground">Servers</h1>
					<p className="text-[12px] text-muted-foreground">
						One-click server connections automatically bound to matching modpacks and accounts.
					</p>
				</div>

				<button
					type="button"
					onClick={() => {
						setCreatorOpen(true)
					}}
					className="flex h-8 cursor-pointer items-center gap-1.5 rounded-xl bg-primary px-3 text-[12px] font-semibold text-primary-foreground transition-opacity hover:opacity-90"
				>
					<Plus className="size-3.5" />
					<span>Add Server</span>
				</button>
			</div>

			<div className="flex flex-col gap-2.5">
				{servers.map((server) => {
					return <ServerRow key={server.id} server={server} onConnect={handleConnect} />
				})}
			</div>

			<LocalServerCreatorModal
				open={creatorOpen}
				onClose={() => {
					setCreatorOpen(false)
				}}
				onCreateServer={handleCreateServer}
			/>
		</div>
	)
}

export default ServersPage
