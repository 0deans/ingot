import { Play, Server, Signal, Users } from "lucide-react"
import { useTranslation } from "react-i18next"
import type { InstanceEngine } from "@/shared/stores/useInstancesStore"

export type ServerMotdKey =
	| "servers.catalog.srv-ingot-smp.motd"
	| "servers.catalog.srv-horizon-lab.motd"
	| "servers.catalog.srv-create-brass.motd"
	| "servers.catalog.srv-hypixel.motd"

export interface ServerEntry {
	id: string
	name: string
	address: string
	motd: string
	motdKey?: ServerMotdKey
	playersOnline: number
	playersMax: number
	pingMs: number
	version: string
	instanceName: string
	engine: InstanceEngine
}

interface ServerRowProps {
	server: ServerEntry
	onConnect: (server: ServerEntry) => void
}

const ServerRow = ({ server, onConnect }: ServerRowProps) => {
	const { t } = useTranslation()

	return (
		<div className="flex flex-wrap items-center justify-between gap-3 rounded-2xl border border-border bg-card/65 px-4 py-3 transition-colors hover:border-foreground/20 hover:bg-card">
			<div className="flex min-w-0 items-center gap-3">
				<div className="flex size-10 shrink-0 items-center justify-center rounded-xl bg-foreground/6 text-primary">
					<Server className="size-4.5" />
				</div>
				<div className="min-w-0">
					<div className="flex items-center gap-2">
						<h2 className="truncate text-[14px] font-semibold text-foreground">{server.name}</h2>
						<span className="rounded-md bg-foreground/8 px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground">
							{server.address}
						</span>
					</div>
					<p className="mt-0.5 truncate text-[12px] text-muted-foreground">
						{server.motdKey ? t(server.motdKey) : server.motd}
					</p>
				</div>
			</div>

			<div className="flex items-center gap-4">
				<div className="flex items-center gap-3 text-[11.5px] text-muted-foreground">
					<span className="flex items-center gap-1 tabular-nums">
						<Users className="size-3.5 text-muted-foreground" />
						<span>
							{server.playersOnline}/{server.playersMax}
						</span>
					</span>
					<span className="flex items-center gap-1 text-primary tabular-nums">
						<Signal className="size-3.5" />
						<span>{server.pingMs} ms</span>
					</span>
				</div>

				<button
					type="button"
					onClick={() => {
						onConnect(server)
					}}
					className="flex h-8 cursor-pointer items-center gap-1.5 rounded-xl bg-primary px-3.5 text-[12px] font-semibold text-primary-foreground transition-opacity hover:opacity-90"
				>
					<Play className="size-3.5 fill-current" />
					<span>{t("servers.connect")}</span>
				</button>
			</div>
		</div>
	)
}

export default ServerRow
