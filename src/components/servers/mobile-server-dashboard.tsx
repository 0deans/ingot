import { Check, ChevronDown, Plus, Server, Settings2 } from "lucide-react"
import { memo, useEffect, useState } from "react"
import type { PlayerDetails, ServerConfig } from "@/bindings"
import NewServerDialog from "@/components/servers/new-server-dialog"
import { Button } from "@/components/ui/button"
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog"
import { cn } from "@/lib/utils"
import { useAndroidHostService } from "@/services/hosting"
import { useServerIcon, useServerStatus } from "@/services/server-data"
import { useServers } from "@/services/server-service"
import type { WorkspaceTab } from "./panels/overview-panel"
import { StatusPill } from "./panels/overview-panel"
import { FILL_TABS, tabLabel, WORKSPACE_TABS, WorkspaceContent } from "./server-workspace"

const SELECTED_KEY = "ingot:mobile-server"

function readSelected(): string | null {
	try {
		return localStorage.getItem(SELECTED_KEY)
	} catch {
		return null
	}
}

function writeSelected(id: string) {
	try {
		localStorage.setItem(SELECTED_KEY, id)
	} catch {
		// Storage can be unavailable; the first server is used then
	}
}

/** Phone UI: server switcher on top, the shared panels in the middle, tabs at the bottom */
export const MobileServerDashboard = memo(() => {
	const { servers, isLoading } = useServers()
	const [selectedId, setSelectedId] = useState<string | null>(readSelected)
	const [tab, setTab] = useState<WorkspaceTab>("overview")
	const [switcherOpen, setSwitcherOpen] = useState(false)
	const [newOpen, setNewOpen] = useState(false)
	const [mapFocus, setMapFocus] = useState<PlayerDetails | null>(null)

	useAndroidHostService(servers)

	const server = servers.find((s) => s.id === selectedId) ?? servers[0]

	useEffect(() => {
		if (server && server.id !== selectedId) setSelectedId(server.id)
	}, [server, selectedId])

	const select = (id: string) => {
		setSelectedId(id)
		writeSelected(id)
		setTab("overview")
		setSwitcherOpen(false)
	}

	if (!server) {
		return (
			<div className="flex h-dvh flex-col items-center justify-center gap-4 bg-zinc-950 px-8 text-center">
				<div className="flex size-16 items-center justify-center rounded-3xl border border-emerald-500/20 bg-emerald-500/10">
					<Server className="size-7 text-emerald-400" />
				</div>
				<div>
					<h1 className="font-bold text-xl text-zinc-50">
						{isLoading ? "Loading..." : "Host a Minecraft server"}
					</h1>
					{!isLoading && (
						<p className="mt-2 text-sm text-zinc-400 leading-relaxed">
							Run a server right on this phone and invite friends from anywhere.
						</p>
					)}
				</div>
				{!isLoading && (
					<Button
						onClick={() => setNewOpen(true)}
						className="h-12 gap-2 rounded-2xl bg-emerald-600 px-6 font-semibold text-white hover:bg-emerald-500"
					>
						<Plus className="size-4" /> Create server
					</Button>
				)}
				<NewServerDialog open={newOpen} onOpenChange={setNewOpen} />
			</div>
		)
	}

	const fill = FILL_TABS.includes(tab)

	return (
		<div className="flex h-dvh flex-col bg-zinc-950 text-zinc-100">
			<header className="flex shrink-0 items-center gap-2 border-zinc-900 border-b px-3 py-2.5">
				<ServerSwitcherButton server={server} onClick={() => setSwitcherOpen(true)} />
				<button
					type="button"
					onClick={() => setTab("settings")}
					aria-label="Server settings"
					className={cn(
						"flex size-10 shrink-0 items-center justify-center rounded-xl transition-colors",
						tab === "settings"
							? "bg-emerald-500/15 text-emerald-400"
							: "text-zinc-400 active:bg-zinc-900",
					)}
				>
					<Settings2 className="size-5" />
				</button>
			</header>

			<main
				className={cn(
					"min-h-0 flex-1",
					fill ? "p-3" : "overflow-y-auto overflow-x-hidden px-3 pt-3 pb-6",
				)}
			>
				<WorkspaceContent
					server={server}
					tab={tab}
					onTabChange={setTab}
					onDeleted={() => setTab("overview")}
					mapFocus={mapFocus}
					onMapFocus={setMapFocus}
				/>
			</main>

			<nav className="grid shrink-0 grid-cols-5 border-zinc-900 border-t bg-zinc-950 pb-[env(safe-area-inset-bottom)]">
				{/* Settings lives behind the gear in the top bar, keeping five tabs here */}
				{WORKSPACE_TABS.filter((t) => t.id !== "settings").map(({ id, label, icon: Icon }) => (
					<button
						key={id}
						type="button"
						onClick={() => setTab(id)}
						className={cn(
							"flex flex-col items-center gap-1 pt-2.5 pb-2 font-medium text-[10px] transition-colors",
							tab === id ? "text-emerald-400" : "text-zinc-500 active:text-zinc-300",
						)}
					>
						<span
							className={cn(
								"flex h-7 w-12 items-center justify-center rounded-full transition-colors",
								tab === id && "bg-emerald-500/15",
							)}
						>
							<Icon className="size-[18px]" />
						</span>
						{id === "overview" ? "Home" : tabLabel(id, label, server.core)}
					</button>
				))}
			</nav>

			<Dialog open={switcherOpen} onOpenChange={setSwitcherOpen}>
				<DialogContent className="gap-3 p-4">
					<DialogTitle className="px-1 text-base">Your servers</DialogTitle>
					<div className="-mx-1 flex max-h-[55dvh] flex-col gap-1 overflow-y-auto">
						{servers.map((s) => (
							<ServerRow
								key={s.id}
								server={s}
								selected={s.id === server.id}
								onClick={() => select(s.id)}
							/>
						))}
					</div>
					<Button
						onClick={() => {
							setSwitcherOpen(false)
							setNewOpen(true)
						}}
						className="h-12 gap-2 rounded-2xl bg-emerald-600 font-semibold text-white hover:bg-emerald-500"
					>
						<Plus className="size-4" /> New server
					</Button>
				</DialogContent>
			</Dialog>

			<NewServerDialog
				open={newOpen}
				onOpenChange={setNewOpen}
				onServerCreated={(created) => select(created.id)}
			/>
		</div>
	)
})

function ServerIcon({ serverId, className }: { serverId: string; className?: string }) {
	const { data: icon } = useServerIcon(serverId)
	return (
		<div
			className={cn(
				"flex shrink-0 items-center justify-center overflow-hidden rounded-xl border border-zinc-800 bg-zinc-900",
				className,
			)}
		>
			{icon ? (
				<img src={icon} alt="" className="size-full object-cover [image-rendering:pixelated]" />
			) : (
				<Server className="size-4 text-zinc-500" />
			)}
		</div>
	)
}

function ServerSwitcherButton({ server, onClick }: { server: ServerConfig; onClick: () => void }) {
	const { status } = useServerStatus(server.id)
	return (
		<button
			type="button"
			onClick={onClick}
			className="flex min-w-0 flex-1 items-center gap-2.5 rounded-2xl px-1.5 py-1 text-left transition-colors active:bg-zinc-900"
		>
			<ServerIcon serverId={server.id} className="size-9" />
			<div className="min-w-0 flex-1">
				<div className="flex items-center gap-1">
					<span className="truncate font-semibold text-sm text-zinc-50">{server.name}</span>
					<ChevronDown className="size-4 shrink-0 text-zinc-500" />
				</div>
				<p className="truncate text-[11px] text-zinc-500 capitalize">
					{server.core} {server.gameVersion}
				</p>
			</div>
			<StatusPill status={status} />
		</button>
	)
}

function ServerRow({
	server,
	selected,
	onClick,
}: {
	server: ServerConfig
	selected: boolean
	onClick: () => void
}) {
	const { status } = useServerStatus(server.id)
	return (
		<button
			type="button"
			onClick={onClick}
			className={cn(
				"flex items-center gap-3 rounded-2xl p-2.5 text-left transition-colors",
				selected ? "bg-zinc-900" : "active:bg-zinc-900",
			)}
		>
			<ServerIcon serverId={server.id} className="size-11" />
			<div className="min-w-0 flex-1">
				<p className="truncate font-medium text-sm text-zinc-100">{server.name}</p>
				<p className="truncate text-[11px] text-zinc-500 capitalize">
					{server.core} {server.gameVersion} · {status}
				</p>
			</div>
			{selected && <Check className="size-4 text-emerald-400" />}
		</button>
	)
}

MobileServerDashboard.displayName = "MobileServerDashboard"
export default MobileServerDashboard
