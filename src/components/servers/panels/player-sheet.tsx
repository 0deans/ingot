import {
	Apple,
	Ban,
	Crown,
	Droplets,
	Flame,
	Heart,
	Loader2,
	LogOut,
	MapPin,
	Shield,
	Sparkles,
} from "lucide-react"
import { useState } from "react"
import type { ItemStack, PlayerDetails } from "@/bindings"
import { Button } from "@/components/ui/button"
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog"
import {
	dimensionStyle,
	formatRelativeTime,
	formatTicks,
	GAMEMODES,
	prettyId,
	romanNumeral,
} from "@/lib/minecraft"
import { cn } from "@/lib/utils"
import { useAccessList, usePlayerDetails } from "@/services/server-data"
import { serverService } from "@/services/server-service"
import {
	EmptyState,
	ErrorNote,
	ItemIcon,
	ItemSlot,
	PlayerAvatar,
	Segmented,
	StatBar,
} from "../shared/primitives"

type SheetTab = "inventory" | "ender" | "effects"

export function PlayerSheet({
	serverId,
	name,
	isRunning,
	onClose,
	onShowOnMap,
}: {
	serverId: string
	name: string | null
	isRunning: boolean
	onClose: () => void
	onShowOnMap?: (player: PlayerDetails) => void
}) {
	const { data: player, isLoading, error } = usePlayerDetails(serverId, name, isRunning)
	const [tab, setTab] = useState<SheetTab>("inventory")
	const [selectedItem, setSelectedItem] = useState<ItemStack | null>(null)

	return (
		<Dialog open={Boolean(name)} onOpenChange={(open) => !open && onClose()}>
			<DialogContent className="flex max-h-[90vh] flex-col gap-0 overflow-hidden p-0 sm:max-w-2xl">
				<DialogTitle className="sr-only">{name}</DialogTitle>
				{isLoading && !player ? (
					<div className="flex items-center justify-center gap-2 py-16 text-sm text-zinc-500">
						<Loader2 className="size-4 animate-spin" /> Loading player...
					</div>
				) : !player ? (
					<div className="p-5">
						<ErrorNote>{String(error ?? "No data for this player yet.")}</ErrorNote>
					</div>
				) : (
					<div className="flex min-h-0 flex-col overflow-y-auto">
						<PlayerHeader player={player} />

						<div className="grid gap-4 px-4 pb-4 sm:grid-cols-2 sm:px-5">
							<div className="flex flex-col gap-3 rounded-2xl border border-zinc-800/80 bg-zinc-900/40 p-4">
								<StatBar
									icon={Heart}
									label="Health"
									value={player.health + player.absorption}
									max={player.maxHealth}
									color="#f43f5e"
								/>
								<StatBar icon={Apple} label="Hunger" value={player.food} max={20} color="#f59e0b" />
								{player.air < 300 && (
									<StatBar
										icon={Droplets}
										label="Air"
										value={player.air}
										max={300}
										color="#38bdf8"
									/>
								)}
								<div className="flex flex-col gap-1.5">
									<div className="flex items-center justify-between text-xs">
										<span className="flex items-center gap-1.5 text-zinc-400">
											<Sparkles className="size-3.5 text-lime-400" />
											Experience
										</span>
										<span className="font-medium font-mono text-lime-300">
											Level {player.xpLevel}
										</span>
									</div>
									<div className="h-1.5 overflow-hidden rounded-full bg-zinc-800">
										<div
											className="h-full rounded-full bg-lime-400"
											style={{ width: `${player.xpProgress * 100}%` }}
										/>
									</div>
								</div>
							</div>

							<div className="flex flex-col gap-2 rounded-2xl border border-zinc-800/80 bg-zinc-900/40 p-4">
								<span className="text-[11px] text-zinc-500 uppercase tracking-wider">
									Equipment
								</span>
								<div className="grid grid-cols-5 gap-1.5">
									{(
										[
											["Head", player.head],
											["Chest", player.chest],
											["Legs", player.legs],
											["Feet", player.feet],
											["Offhand", player.offhand],
										] as const
									).map(([label, item]) => (
										<ItemSlot
											key={label}
											item={item}
											onSelect={setSelectedItem}
											placeholder={<span className="text-[9px] text-zinc-600">{label}</span>}
										/>
									))}
								</div>
								<div className="mt-auto flex items-center justify-between pt-2 text-xs">
									<span className="flex items-center gap-1.5 text-zinc-500">
										<MapPin className="size-3.5" />
										{Math.floor(player.x)}, {Math.floor(player.y)}, {Math.floor(player.z)}
									</span>
									{onShowOnMap && (
										<button
											type="button"
											onClick={() => onShowOnMap(player)}
											className="font-medium text-emerald-400 hover:text-emerald-300"
										>
											Show on map
										</button>
									)}
								</div>
							</div>
						</div>

						<div className="flex flex-col gap-3 px-4 pb-4 sm:px-5">
							<Segmented
								value={tab}
								onChange={(t) => {
									setTab(t)
									setSelectedItem(null)
								}}
								options={[
									{ value: "inventory", label: "Inventory" },
									{ value: "ender", label: `Ender chest (${player.enderItems.length})` },
									{ value: "effects", label: `Effects (${player.effects.length})` },
								]}
							/>

							{tab === "inventory" && (
								<InventoryGrid player={player} selected={selectedItem} onSelect={setSelectedItem} />
							)}
							{tab === "ender" && (
								<SlotGrid
									items={player.enderItems}
									slots={27}
									offset={0}
									onSelect={setSelectedItem}
								/>
							)}
							{tab === "effects" && <EffectsList player={player} />}

							{selectedItem && <ItemDetails item={selectedItem} />}
						</div>

						{player.online && isRunning && <PlayerActions serverId={serverId} player={player} />}
					</div>
				)}
			</DialogContent>
		</Dialog>
	)
}

function PlayerHeader({ player }: { player: PlayerDetails }) {
	const dim = dimensionStyle(player.dimension)
	return (
		<div className="flex items-center gap-3.5 px-4 pt-2 pb-4 sm:px-5 sm:pt-5">
			<PlayerAvatar name={player.name} size={56} online={player.online} />
			<div className="min-w-0 flex-1">
				<h2 className="truncate font-bold text-lg text-zinc-50">{player.name}</h2>
				<div className="mt-1 flex flex-wrap items-center gap-1.5 text-[11px]">
					<span className="rounded-full border border-zinc-700 bg-zinc-800/60 px-2 py-0.5 text-zinc-300 capitalize">
						{player.gamemode}
					</span>
					<span className={cn("rounded-full border px-2 py-0.5", dim.badge)}>{dim.label}</span>
					{player.onFire && (
						<span className="flex items-center gap-1 rounded-full border border-orange-500/30 bg-orange-500/10 px-2 py-0.5 text-orange-300">
							<Flame className="size-3" /> On fire
						</span>
					)}
					<span className="text-zinc-500">
						{player.online ? "Online now" : `Last seen ${formatRelativeTime(player.lastSeen)}`}
					</span>
				</div>
			</div>
		</div>
	)
}

function InventoryGrid({
	player,
	selected,
	onSelect,
}: {
	player: PlayerDetails
	selected: ItemStack | null
	onSelect: (item: ItemStack) => void
}) {
	return (
		<div className="flex flex-col gap-2">
			<SlotGrid
				items={player.inventory}
				slots={27}
				offset={9}
				onSelect={onSelect}
				selected={selected}
			/>
			<div className="h-px bg-zinc-800" />
			<SlotGrid
				items={player.inventory}
				slots={9}
				offset={0}
				onSelect={onSelect}
				selected={selected}
				highlightSlot={player.selectedSlot}
			/>
		</div>
	)
}

function SlotGrid({
	items,
	slots,
	offset,
	onSelect,
	selected,
	highlightSlot,
}: {
	items: ItemStack[]
	slots: number
	offset: number
	onSelect: (item: ItemStack) => void
	selected?: ItemStack | null
	highlightSlot?: number
}) {
	const bySlot = new Map(items.map((i) => [i.slot, i]))
	return (
		<div className="grid grid-cols-9 gap-1">
			{Array.from({ length: slots }, (_, i) => {
				const slot = offset + i
				const item = bySlot.get(slot)
				return (
					<ItemSlot
						key={slot}
						item={item}
						size="sm"
						selected={highlightSlot === slot || (selected != null && selected === item)}
						onSelect={onSelect}
					/>
				)
			})}
		</div>
	)
}

function ItemDetails({ item }: { item: ItemStack }) {
	return (
		<div className="flex items-center gap-3 rounded-xl border border-zinc-800 bg-zinc-900/60 p-3">
			<ItemIcon id={item.id} className="size-9" />
			<div className="min-w-0 flex-1 text-xs">
				<p
					className={cn(
						"truncate font-medium text-sm",
						item.enchanted ? "text-violet-300" : "text-zinc-100",
					)}
				>
					{item.customName ?? prettyId(item.id)}
				</p>
				<p className="truncate font-mono text-[11px] text-zinc-500">
					{item.id} × {item.count}
					{item.maxDamage ? ` · ${item.maxDamage - item.damage}/${item.maxDamage} durability` : ""}
				</p>
			</div>
			{item.enchanted && (
				<span className="rounded-full border border-violet-500/30 bg-violet-500/10 px-2 py-0.5 text-[10px] text-violet-300">
					Enchanted
				</span>
			)}
		</div>
	)
}

function EffectsList({ player }: { player: PlayerDetails }) {
	if (player.effects.length === 0) {
		return <EmptyState icon={Sparkles} title="No active effects" className="py-6" />
	}
	return (
		<div className="flex flex-col divide-y divide-zinc-800/70 rounded-xl border border-zinc-800">
			{player.effects.map((e) => (
				<div key={e.id} className="flex items-center justify-between px-3 py-2.5 text-xs">
					<span className="text-zinc-200">
						{prettyId(e.id)} {romanNumeral(e.amplifier + 1)}
					</span>
					<span className="font-mono text-zinc-500">{formatTicks(e.duration)}</span>
				</div>
			))}
		</div>
	)
}

function PlayerActions({ serverId, player }: { serverId: string; player: PlayerDetails }) {
	const ops = useAccessList(serverId, "ops")
	const whitelist = useAccessList(serverId, "whitelist")
	const [confirm, setConfirm] = useState<"kick" | "ban" | null>(null)
	const [busy, setBusy] = useState<string | null>(null)

	const isOp = ops.data?.some((e) => e.name.toLowerCase() === player.name.toLowerCase()) ?? false
	const isWhitelisted =
		whitelist.data?.some((e) => e.name.toLowerCase() === player.name.toLowerCase()) ?? false

	const run = async (key: string, command: string) => {
		setBusy(key)
		try {
			await serverService.sendCommand(serverId, command)
		} finally {
			setTimeout(() => setBusy(null), 400)
		}
	}

	return (
		<div className="flex flex-col gap-3 border-zinc-800/80 border-t bg-zinc-950/60 p-4 sm:px-5">
			<div className="grid grid-cols-4 gap-1 rounded-xl border border-zinc-800 bg-zinc-900/60 p-1">
				{GAMEMODES.map((mode) => (
					<button
						key={mode}
						type="button"
						onClick={() => run(`gm-${mode}`, `gamemode ${mode} ${player.name}`)}
						className={cn(
							"rounded-lg py-1.5 font-medium text-[11px] capitalize transition-colors",
							player.gamemode === mode
								? "bg-emerald-500/15 text-emerald-300"
								: "text-zinc-500 hover:text-zinc-200",
						)}
					>
						{mode}
					</button>
				))}
			</div>

			<div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
				<ActionButton
					icon={Heart}
					label="Heal"
					busy={busy === "heal"}
					onClick={() =>
						run("heal", `effect give ${player.name} minecraft:instant_health 1 10 true`)
					}
				/>
				<ActionButton
					icon={Apple}
					label="Feed"
					busy={busy === "feed"}
					onClick={() => run("feed", `effect give ${player.name} minecraft:saturation 1 10 true`)}
				/>
				<ActionButton
					icon={Crown}
					label={isOp ? "Remove OP" : "Make OP"}
					active={isOp}
					busy={ops.add.isPending || ops.remove.isPending}
					onClick={() => (isOp ? ops.remove.mutate(player.name) : ops.add.mutate(player.name))}
				/>
				<ActionButton
					icon={Shield}
					label={isWhitelisted ? "Whitelisted" : "Whitelist"}
					active={isWhitelisted}
					busy={whitelist.add.isPending || whitelist.remove.isPending}
					onClick={() =>
						isWhitelisted ? whitelist.remove.mutate(player.name) : whitelist.add.mutate(player.name)
					}
				/>
			</div>

			{confirm ? (
				<div className="flex items-center justify-between gap-2 rounded-xl border border-rose-500/25 bg-rose-500/10 p-2 pl-3">
					<span className="text-rose-200 text-xs">
						{confirm === "kick" ? "Kick" : "Ban"} {player.name}?
					</span>
					<div className="flex gap-1.5">
						<Button size="sm" variant="ghost" onClick={() => setConfirm(null)}>
							Cancel
						</Button>
						<Button
							size="sm"
							variant="destructive"
							onClick={() => {
								run(confirm, `${confirm} ${player.name}`)
								setConfirm(null)
							}}
						>
							Confirm
						</Button>
					</div>
				</div>
			) : (
				<div className="grid grid-cols-2 gap-2">
					<ActionButton icon={LogOut} label="Kick" danger onClick={() => setConfirm("kick")} />
					<ActionButton icon={Ban} label="Ban" danger onClick={() => setConfirm("ban")} />
				</div>
			)}
		</div>
	)
}

function ActionButton({
	icon: Icon,
	label,
	onClick,
	busy,
	active,
	danger,
}: {
	icon: typeof Heart
	label: string
	onClick: () => void
	busy?: boolean
	active?: boolean
	danger?: boolean
}) {
	return (
		<button
			type="button"
			onClick={onClick}
			disabled={busy}
			className={cn(
				"flex h-10 items-center justify-center gap-1.5 rounded-xl border font-medium text-xs transition-colors disabled:opacity-60",
				danger
					? "border-rose-500/20 bg-rose-500/5 text-rose-300 hover:bg-rose-500/15"
					: active
						? "border-emerald-500/30 bg-emerald-500/10 text-emerald-300 hover:bg-emerald-500/15"
						: "border-zinc-800 bg-zinc-900/60 text-zinc-300 hover:bg-zinc-800",
			)}
		>
			{busy ? <Loader2 className="size-3.5 animate-spin" /> : <Icon className="size-3.5" />}
			{label}
		</button>
	)
}
