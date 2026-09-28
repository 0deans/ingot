import { Ban, Crown, Loader2, Plus, Shield, X } from "lucide-react"
import { type FormEvent, useState } from "react"
import { useTranslation } from "react-i18next"
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

/** Texts are under access.<kind>.* in the locale files */
const LISTS: Record<AccessListKind, { icon: typeof Shield }> = {
	whitelist: { icon: Shield },
	ops: { icon: Crown },
	bans: { icon: Ban },
}

export function AccessPanel({ server }: { server: ServerConfig }) {
	const { t } = useTranslation()
	const [kind, setKind] = useState<AccessListKind>("whitelist")
	return (
		<div className="flex flex-col gap-3">
			<Segmented
				value={kind}
				onChange={setKind}
				options={[
					{ value: "whitelist", label: t("access.tabs.whitelist"), icon: Shield },
					{ value: "ops", label: t("access.tabs.ops"), icon: Crown },
					{ value: "bans", label: t("access.tabs.bans"), icon: Ban },
				]}
			/>
			{kind === "whitelist" && <WhitelistToggle server={server} />}
			<AccessList key={kind} serverId={server.id} kind={kind} />
		</div>
	)
}

function WhitelistToggle({ server }: { server: ServerConfig }) {
	const { t } = useTranslation()
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
				<p className="font-medium text-sm text-zinc-100">
					{enabled ? t("access.whitelistOn") : t("access.whitelistOff")}
				</p>
				<p className="text-xs text-zinc-500">
					{enabled ? t("access.onlyListed") : t("access.anyone")}
				</p>
			</div>
			<Switch checked={enabled} disabled={save.isPending} onCheckedChange={toggle} />
		</Card>
	)
}

function AccessList({ serverId, kind }: { serverId: string; kind: AccessListKind }) {
	const { t } = useTranslation()
	const meta = LISTS[kind]
	const { data = [], isLoading, add, remove } = useAccessList(serverId, kind)
	const [name, setName] = useState("")
	const [error, setError] = useState<string | null>(null)

	const submit = async (e: FormEvent) => {
		e.preventDefault()
		const trimmed = name.trim()
		if (!isValidPlayerName(trimmed)) {
			setError(t("access.invalidName"))
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
			<CardHeader
				icon={meta.icon}
				title={t(`access.${kind}.title`)}
				description={t(`access.${kind}.description`)}
			/>
			<form onSubmit={submit} className="flex gap-2 px-4 pb-3">
				<Input
					value={name}
					onChange={(e) => setName(e.target.value)}
					placeholder={t("access.playerName")}
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
					{t(`access.${kind}.add`)}
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
				<EmptyState icon={meta.icon} title={t(`access.${kind}.empty`)} className="pt-4" />
			) : (
				<ul className="divide-y divide-zinc-800/70 border-zinc-800/70 border-t">
					{data.map((entry) => (
						<li key={entry.uuid || entry.name} className="flex items-center gap-3 px-4 py-2.5">
							<PlayerAvatar name={entry.name} size={32} />
							<div className="min-w-0 flex-1">
								<p className="truncate font-medium text-sm text-zinc-100">{entry.name}</p>
								<p className="truncate text-[11px] text-zinc-500">
									{kind === "ops"
										? t("access.permissionLevel", { level: entry.level ?? 4 })
										: kind === "bans"
											? (entry.reason ?? t("access.banned"))
											: entry.uuid || t("access.noUuid")}
								</p>
							</div>
							<Button
								size="icon-sm"
								variant="ghost"
								disabled={remove.isPending && remove.variables === entry.name}
								onClick={() => remove.mutate(entry.name)}
								aria-label={t("access.remove", { name: entry.name })}
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
