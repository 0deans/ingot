import { Check, ChevronDown, Plus, Server, Settings2 } from "lucide-react"
import { memo, useEffect, useState } from "react"
import { useTranslation } from "react-i18next"
import type { PlayerDetails, ServerConfig } from "@/bindings"
import { NewServerWizard, NoServersState } from "@/components/servers/new-server-wizard"
import { ImportServerButton } from "@/components/servers/panels/transfer-card"
import { LicensesDialog } from "@/components/settings/licenses"
import { Button } from "@/components/ui/button"
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog"
import {
	Item,
	ItemActions,
	ItemContent,
	ItemDescription,
	ItemMedia,
	ItemTitle,
} from "@/components/ui/item"
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
			<div className="h-dvh overflow-hidden bg-background">
				<NewServerWizard
					onCancel={() => setWizardOpen(false)}
					onServerCreated={handleServerCreated}
					importSlot={
						<ImportServerButton
							className="h-12 gap-2 rounded-2xl border-border"
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
				<div className="flex h-dvh items-center justify-center bg-background">
					<div className="size-8 animate-spin rounded-full border-2 border-border border-t-primary" />
				</div>
			)
		}
		return (
			<div className="h-dvh overflow-hidden bg-background">
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
		<div className="flex h-dvh flex-col bg-background text-foreground">
			<header className="flex shrink-0 items-center gap-2 border-border border-b px-3 py-2.5">
				<ServerSwitcherButton server={server} onClick={() => setSwitcherOpen(true)} />
				<Button
					variant={tab === "settings" ? "secondary" : "ghost"}
					size="icon-lg"
					onClick={() => setTab("settings")}
					aria-label={t("mobileServers.settings")}
					aria-pressed={tab === "settings"}
				>
					<Settings2 />
				</Button>
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

			<nav className="grid shrink-0 grid-cols-5 border-border border-t bg-background pb-[env(safe-area-inset-bottom)]">
				{/* Settings lives behind the gear in the top bar, keeping five tabs here */}
				{WORKSPACE_TABS.filter((t) => t.id !== "settings").map(({ id, label, icon: Icon }) => (
					<button
						key={id}
						type="button"
						onClick={() => setTab(id)}
						className={cn(
							"flex min-w-0 flex-col items-center gap-1 px-0.5 pt-2.5 pb-2 font-medium text-3xs transition-colors",
							tab === id ? "text-primary" : "text-muted-foreground active:text-foreground/80",
						)}
					>
						<span
							className={cn(
								"flex h-7 w-12 items-center justify-center rounded-full transition-colors",
								tab === id && "bg-primary/15",
							)}
						>
							<Icon className="size-4.5" />
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
					<FadeScroll className="-mx-1 max-h-[55dvh] px-1">
						<div className="flex flex-col gap-1">
							{servers.map((s) => (
								<ServerRow
									key={s.id}
									server={s}
									selected={s.id === server.id}
									onClick={() => select(s.id)}
								/>
							))}
						</div>
					</FadeScroll>
					<Button
						size="lg"
						onClick={() => {
							setSwitcherOpen(false)
							setWizardOpen(true)
						}}
						className="h-12 w-full"
					>
						<Plus />
						{t("mobileServers.newServer")}
					</Button>
					<ImportServerButton
						className="h-11 gap-2 rounded-2xl border-border"
						onImported={(created) => {
							refresh()
							select(created.id)
						}}
					/>
					<Button
						variant="link"
						size="sm"
						onClick={() => {
							setSwitcherOpen(false)
							setLicensesOpen(true)
						}}
						className="mt-1 self-center text-muted-foreground"
					>
						{t("mobileServers.about")}
					</Button>
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
				"flex shrink-0 items-center justify-center overflow-hidden rounded-xl border border-border bg-card",
				className,
			)}
		>
			{icon ? (
				<img src={icon} alt="" className="size-full object-cover [image-rendering:pixelated]" />
			) : (
				<Server className="size-4 text-muted-foreground" />
			)}
		</div>
	)
}

function ServerSwitcherButton({ server, onClick }: { server: ServerConfig; onClick: () => void }) {
	const { status } = useServerStatus(server.id)
	return (
		<Item
			size="sm"
			render={<button type="button" onClick={onClick} />}
			className="min-w-0 flex-1 flex-nowrap px-1.5 py-1 text-left active:bg-muted/50"
		>
			<ItemMedia>
				<ServerIcon serverId={server.id} className="size-9" />
			</ItemMedia>
			<ItemContent className="min-w-0">
				<ItemTitle>
					<span className="truncate">{server.name}</span>
					<ChevronDown className="size-4 shrink-0 text-muted-foreground" />
				</ItemTitle>
				<ItemDescription className="truncate capitalize">
					{server.core} {server.gameVersion}
				</ItemDescription>
			</ItemContent>
			<ItemActions>
				<StatusPill status={status} />
			</ItemActions>
		</Item>
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
		<Item
			variant={selected ? "muted" : "default"}
			render={<button type="button" onClick={onClick} aria-current={selected || undefined} />}
			className="text-left active:bg-muted/50"
		>
			<ItemMedia>
				<ServerIcon serverId={server.id} className="size-11" />
			</ItemMedia>
			<ItemContent className="min-w-0">
				<ItemTitle className="truncate">{server.name}</ItemTitle>
				<ItemDescription className="truncate capitalize">
					{server.core} {server.gameVersion} · {status}
				</ItemDescription>
			</ItemContent>
			{selected && <Check className="size-4 text-primary" />}
		</Item>
	)
}

MobileServerDashboard.displayName = "MobileServerDashboard"
export default MobileServerDashboard
