import { Ban, Crown, Loader2, Plus, Shield, X } from "lucide-react"
import { type FormEvent, useState } from "react"
import type { AccessListKind, ServerConfig } from "@/bindings"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Switch } from "@/components/ui/switch"
import { isValidPlayerName } from "@/lib/minecraft"
import { useAccessList, useServerPropertiesAll, useServerStatus } from "@/services/server-data"
import { serverService } from "@/services/server-service"
import {
	Card,
	CardHeader,
	EmptyState,
	ErrorNote,
	PlayerAvatar,
	Segmented,
} from "../shared/primitives"

const LISTS: Record<
	AccessListKind,
	{ title: string; icon: typeof Shield; empty: string; description: string; addLabel: string }
> = {
	whitelist: {
		title: "Whitelist",
		icon: Shield,
		empty: "Nobody is whitelisted yet",
		description: "When enabled, only these players can join.",
		addLabel: "Add",
	},
	ops: {
		title: "Operators",
		icon: Crown,
		empty: "No operators",
		description: "Operators can use every command, including /stop and /op.",
		addLabel: "Make OP",
	},
	bans: {
		title: "Banned players",
		icon: Ban,
		empty: "Nobody is banned",
		description: "Banned players can't join until pardoned.",
		addLabel: "Ban",
	},
}

export function AccessPanel({ server }: { server: ServerConfig }) {
	const [kind, setKind] = useState<AccessListKind>("whitelist")
	return (
		<div className="flex flex-col gap-3">
			<Segmented
				value={kind}
				onChange={setKind}
				options={[
					{ value: "whitelist", label: "Whitelist", icon: Shield },
					{ value: "ops", label: "Operators", icon: Crown },
					{ value: "bans", label: "Bans", icon: Ban },
				]}
			/>
			{kind === "whitelist" && <WhitelistToggle server={server} />}
			<AccessList key={kind} serverId={server.id} kind={kind} />
		</div>
	)
}

function WhitelistToggle({ server }: { server: ServerConfig }) {
	const { isRunning } = useServerStatus(server.id)
	const { values, save } = useServerPropertiesAll(server.id)
	const enabled = values.get("white-list") === "true"

	const toggle = async (next: boolean) => {
		await save.mutateAsync([
			{ key: "white-list", value: String(next) },
			// Also kick players who aren't on the list when it's turned on
			{ key: "enforce-whitelist", value: String(next) },
		])
		if (isRunning) {
			await serverService.sendCommand(server.id, next ? "whitelist on" : "whitelist off")
		}
	}

	return (
		<Card className="flex items-center justify-between gap-3 px-4 py-3.5">
			<div>
				<p className="font-medium text-sm text-zinc-100">Whitelist {enabled ? "on" : "off"}</p>
				<p className="text-xs text-zinc-500">
					{enabled ? "Only listed players can join." : "Anyone can join this server."}
				</p>
			</div>
			<Switch checked={enabled} disabled={save.isPending} onCheckedChange={toggle} />
		</Card>
	)
}

function AccessList({ serverId, kind }: { serverId: string; kind: AccessListKind }) {
	const meta = LISTS[kind]
	const { data = [], isLoading, add, remove } = useAccessList(serverId, kind)
	const [name, setName] = useState("")
	const [error, setError] = useState<string | null>(null)

	const submit = async (e: FormEvent) => {
		e.preventDefault()
		const trimmed = name.trim()
		if (!isValidPlayerName(trimmed)) {
			setError("Usernames can only contain letters, numbers and _")
			return
		}
		setError(null)
		try {
			await add.mutateAsync(trimmed)
			setName("")
		} catch (err) {
			setError(String(err))
		}
	}

	return (
		<Card>
			<CardHeader icon={meta.icon} title={meta.title} description={meta.description} />
			<form onSubmit={submit} className="flex gap-2 px-4 pb-3">
				<Input
					value={name}
					onChange={(e) => setName(e.target.value)}
					placeholder="Player name"
					autoCapitalize="off"
					autoCorrect="off"
					spellCheck={false}
					className="h-10 flex-1 rounded-xl border-zinc-800 bg-zinc-950/60 text-sm"
				/>
				<Button
					type="submit"
					disabled={!name.trim() || add.isPending}
					className="h-10 gap-1.5 rounded-xl bg-emerald-600 px-4 text-white hover:bg-emerald-500"
				>
					{add.isPending ? (
						<Loader2 className="size-4 animate-spin" />
					) : (
						<Plus className="size-4" />
					)}
					{meta.addLabel}
				</Button>
			</form>
			{error && (
				<div className="px-4 pb-3">
					<ErrorNote>{error}</ErrorNote>
				</div>
			)}

			{isLoading ? (
				<div className="flex justify-center py-8">
					<Loader2 className="size-4 animate-spin text-zinc-500" />
				</div>
			) : data.length === 0 ? (
				<EmptyState icon={meta.icon} title={meta.empty} className="pt-4" />
			) : (
				<ul className="divide-y divide-zinc-800/70 border-zinc-800/70 border-t">
					{data.map((entry) => (
						<li key={entry.uuid || entry.name} className="flex items-center gap-3 px-4 py-2.5">
							<PlayerAvatar name={entry.name} size={32} />
							<div className="min-w-0 flex-1">
								<p className="truncate font-medium text-sm text-zinc-100">{entry.name}</p>
								<p className="truncate text-[11px] text-zinc-500">
									{kind === "ops"
										? `Permission level ${entry.level ?? 4}`
										: kind === "bans"
											? (entry.reason ?? "Banned")
											: entry.uuid || "No UUID"}
								</p>
							</div>
							<Button
								size="icon-sm"
								variant="ghost"
								disabled={remove.isPending && remove.variables === entry.name}
								onClick={() => remove.mutate(entry.name)}
								aria-label={`Remove ${entry.name}`}
								className="text-zinc-500 hover:bg-rose-500/10 hover:text-rose-300"
							>
								{remove.isPending && remove.variables === entry.name ? (
									<Loader2 className="size-3.5 animate-spin" />
								) : (
									<X className="size-3.5" />
								)}
							</Button>
						</li>
					))}
				</ul>
			)}
		</Card>
	)
}
