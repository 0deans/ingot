import { Check, ChevronDown, Plus, Server, Settings2 } from "lucide-react"
import { memo, useEffect, useState } from "react"
import { useTranslation } from "react-i18next"
import type { PlayerDetails, ServerConfig } from "@/bindings"
import { Dialog, DialogContent, DialogTitle } from "@/components/common/dialog"
import { NewServerWizard, NoServersState } from "@/components/servers/new-server-wizard"
import { ImportServerButton } from "@/components/servers/panels/transfer-card"
import { LicensesDialog } from "@/components/settings/licenses"
import { cn } from "@/lib/utils"
import { useAndroidHostService } from "@/services/hosting"
import { useServerIcon, useServerStatus } from "@/services/server-data"
import { useServers } from "@/services/server-service"
import type { WorkspaceTab } from "./panels/overview-panel"
import { StatusPill } from "./panels/overview-panel"
import { FILL_TABS, tabLabel, WORKSPACE_TABS, WorkspaceContent } from "./server-workspace"
import { FadeScroll } from "./shared/primitives"

const SELECTED_KEY = "ingot:mobile-server"
const TAB_KEY = "ingot:mobile-tab"

function read(key: string): string | null {
	try {
		return localStorage.getItem(key)
	} catch {
		return null
	}
}

function write(key: string, value: string) {
	try {
		localStorage.setItem(key, value)
	} catch {
		// Storage can be unavailable; defaults are used then
	}
}

/** Last tab, so a WebView reload (Android can recreate it) returns to the same screen */
function readTab(): WorkspaceTab {
	const tab = read(TAB_KEY)
	return WORKSPACE_TABS.some((t) => t.id === tab) ? (tab as WorkspaceTab) : "overview"
}

/** Phone UI: server switcher on top, the shared panels in the middle, tabs at the bottom */
export const MobileServerDashboard = memo(() => {
	const { t } = useTranslation()
	const { servers, isLoading, refresh } = useServers()
	const [selectedId, setSelectedId] = useState<string | null>(() => read(SELECTED_KEY))
	const [tab, setTabState] = useState<WorkspaceTab>(readTab)
	const setTab = (next: WorkspaceTab) => {
		setTabState(next)
		write(TAB_KEY, next)
	}
	const [switcherOpen, setSwitcherOpen] = useState(false)
	const [licensesOpen, setLicensesOpen] = useState(false)
	const [wizardOpen, setWizardOpen] = useState(false)
	const [mapFocus, setMapFocus] = useState<PlayerDetails | null>(null)

	useAndroidHostService(servers)

	const server = servers.find((s) => s.id === selectedId) ?? servers[0]

	useEffect(() => {
		if (server && server.id !== selectedId) setSelectedId(server.id)
	}, [server, selectedId])

	const select = (id: string) => {
		setSelectedId(id)
		write(SELECTED_KEY, id)
		setTab("overview")
		setSwitcherOpen(false)
	}

	const handleServerCreated = (created: ServerConfig) => {
		setWizardOpen(false)
		refresh()
		select(created.id)
	}

	// ── Wizard (full-screen) ──
	if (wizardOpen) {
		return (
			<div className="h-dvh overflow-hidden bg-zinc-950">
				<NewServerWizard
					onCancel={() => setWizardOpen(false)}
					onServerCreated={handleServerCreated}
					importSlot={
						<ImportServerButton
							className="h-12 gap-2 rounded-2xl border-zinc-800"
							onImported={(created) => {
								setWizardOpen(false)
								refresh()
								select(created.id)
							}}
						/>
					}
				/>
			</div>
		)
	}

	// ── Empty state ──
	if (!server) {
		if (isLoading) {
			return (
				<div className="flex h-dvh items-center justify-center bg-zinc-950">
					<div className="size-8 animate-spin rounded-full border-2 border-zinc-800 border-t-emerald-400" />
				</div>
			)
		}
		return (
			<div className="h-dvh overflow-hidden bg-zinc-950">
				<NoServersState
					onCreate={() => setWizardOpen(true)}
					importSlot={
						<ImportServerButton
							onImported={(created) => {
								refresh()
								select(created.id)
							}}
						/>
					}
				/>
			</div>
		)
	}

	const fill = FILL_TABS.includes(tab)
	const content = (
		<WorkspaceContent
			server={server}
			tab={tab}
			onTabChange={setTab}
			onDeleted={() => setTab("overview")}
			mapFocus={mapFocus}
			onMapFocus={setMapFocus}
		/>
	)

	return (
		<div className="flex h-dvh flex-col bg-zinc-950 text-zinc-100">
			<header className="flex shrink-0 items-center gap-2 border-zinc-900 border-b px-3 py-2.5">
				<ServerSwitcherButton server={server} onClick={() => setSwitcherOpen(true)} />
				<button
					type="button"
					onClick={() => setTab("settings")}
					aria-label={t("mobileServers.settings")}
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

			{/* Console and map fill the whole area; other tabs scroll */}
			{fill ? (
				<main className="min-h-0 flex-1">{content}</main>
			) : (
				// Keyed so each tab starts at the top instead of inheriting the last tab's scroll
				<FadeScroll
					key={`${server.id}:${tab}`}
					className="min-h-0 flex-1 overflow-x-hidden px-3 pt-3 pb-6"
				>
					{content}
				</FadeScroll>
			)}

			<nav className="grid shrink-0 grid-cols-5 border-zinc-900 border-t bg-zinc-950 pb-[env(safe-area-inset-bottom)]">
				{/* Settings lives behind the gear in the top bar, keeping five tabs here */}
				{WORKSPACE_TABS.filter((t) => t.id !== "settings").map(({ id, label, icon: Icon }) => (
					<button
						key={id}
						type="button"
						onClick={() => setTab(id)}
						className={cn(
							"flex min-w-0 flex-col items-center gap-1 px-0.5 pt-2.5 pb-2 font-medium text-[10px] transition-colors",
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
						<span className="max-w-full truncate">
							{id === "overview" ? t("serverTabs.home") : tabLabel(id, label, server.core)}
						</span>
					</button>
				))}
			</nav>

			<Dialog open={switcherOpen} onOpenChange={setSwitcherOpen}>
				<DialogContent className="gap-3 p-4">
					<DialogTitle className="px-1 text-base">{t("mobileServers.yourServers")}</DialogTitle>
					<FadeScroll className="-mx-1 flex max-h-[55dvh] flex-col gap-1">
						{servers.map((s) => (
							<ServerRow
								key={s.id}
								server={s}
								selected={s.id === server.id}
								onClick={() => select(s.id)}
							/>
						))}
					</FadeScroll>
					<button
						type="button"
						onClick={() => {
							setSwitcherOpen(false)
							setWizardOpen(true)
						}}
						className="flex h-12 w-full items-center justify-center gap-2 rounded-2xl bg-emerald-600 font-semibold text-sm text-white transition-colors hover:bg-emerald-500 active:bg-emerald-700"
					>
						<Plus className="size-4" /> {t("mobileServers.newServer")}
					</button>
					<ImportServerButton
						className="h-11 gap-2 rounded-2xl border-zinc-800"
						onImported={(created) => {
							refresh()
							select(created.id)
						}}
					/>
					<button
						type="button"
						onClick={() => {
							setSwitcherOpen(false)
							setLicensesOpen(true)
						}}
						className="mt-1 self-center px-3 py-1.5 text-xs text-zinc-500 transition-colors hover:text-zinc-300"
					>
						{t("mobileServers.about")}
					</button>
				</DialogContent>
			</Dialog>
			<LicensesDialog open={licensesOpen} onOpenChange={setLicensesOpen} />
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
