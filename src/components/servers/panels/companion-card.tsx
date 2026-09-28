import { ArrowUpCircle, Download, Loader2, Trash2 } from "lucide-react"
import { useTranslation } from "react-i18next"
import type { ServerConfig } from "@/bindings"
import { Button } from "@/components/ui/button"
import { cn } from "@/lib/utils"
import { useCompanionStatus, usePluginActions } from "@/services/server-data"
import { Card } from "../shared/primitives"
import { addonKind } from "./plugins-panel"

/**
 * Ingot's own plugin (a mod on Fabric), shown apart from the user's. Never installed or updated
 * behind the user's back (only when a server is created); they can remove it and
 * install it again here.
 */
export function CompanionCard({
	server,
	onChanged,
}: {
	server: ServerConfig
	onChanged: (text: string, error?: boolean) => void
}) {
	const { data: status } = useCompanionStatus(server.id)
	const actions = usePluginActions(server.id)
	const { t } = useTranslation()
	const context = addonKind(server.core)?.kind ?? "plugin"
	if (!status?.supported) return null

	const installed = status.fileName !== null
	const outdated = installed && status.installedVersion !== status.bundledVersion
	const busy =
		actions.installCompanion.isPending || actions.remove.isPending || actions.toggle.isPending

	const install = async () => {
		try {
			await actions.installCompanion.mutateAsync()
			onChanged(
				installed ? t("companion.updated", { context }) : t("companion.installed", { context }),
			)
		} catch (e) {
			onChanged(String(e), true)
		}
	}
	const remove = async () => {
		if (!status.fileName) return
		try {
			await actions.remove.mutateAsync(status.fileName)
			onChanged(t("companion.removed", { context }))
		} catch (e) {
			onChanged(String(e), true)
		}
	}
	const enable = async () => {
		if (!status.fileName) return
		try {
			await actions.toggle.mutateAsync({ fileName: status.fileName, enabled: true })
			onChanged(t("companion.enabled", { context }))
		} catch (e) {
			onChanged(String(e), true)
		}
	}

	return (
		<Card className="flex items-center gap-3 px-3.5 py-3">
			<div className="flex size-10 shrink-0 items-center justify-center bg-zinc-900">
				<img
					src="/ingot.svg"
					alt=""
					className={cn("size-6", !installed && "opacity-40 grayscale")}
				/>
			</div>
			<div className="min-w-0 flex-1">
				<p className="flex items-baseline gap-1.5 truncate font-medium text-sm text-zinc-100">
					Ingot
					<span className="font-normal text-[10px] text-zinc-500 uppercase tracking-wider">
						{t("companion.system")}
					</span>
				</p>
				<p className="flex items-center gap-1.5 truncate text-[11px] text-zinc-500">
					<span
						className={cn(
							"size-1.5 shrink-0 rounded-full",
							installed && status.enabled ? "bg-emerald-400" : "bg-zinc-600",
						)}
					/>
					{installed
						? status.enabled
							? status.installedVersion
								? `${t("companion.liveMap")} · ${status.installedVersion}`
								: t("companion.liveMap")
							: t("companion.disabled")
						: t("companion.notInstalled")}
				</p>
			</div>

			{/* One main action, plus remove once installed */}
			{!installed ? (
				<ActionButton busy={actions.installCompanion.isPending} disabled={busy} onClick={install}>
					<Download className="size-3.5" /> {t("common.install")}
				</ActionButton>
			) : outdated ? (
				<ActionButton busy={actions.installCompanion.isPending} disabled={busy} onClick={install}>
					<ArrowUpCircle className="size-3.5" /> {t("common.update")}
				</ActionButton>
			) : (
				!status.enabled && (
					<ActionButton busy={actions.toggle.isPending} disabled={busy} onClick={enable}>
						{t("common.enable")}
					</ActionButton>
				)
			)}
			{installed && (
				<button
					type="button"
					onClick={remove}
					disabled={busy}
					title={t("companion.remove", { context })}
					aria-label={t("companion.remove", { context })}
					className="flex size-8 shrink-0 items-center justify-center text-zinc-500 transition-colors hover:bg-rose-500/10 hover:text-rose-300 disabled:opacity-50"
				>
					{actions.remove.isPending ? (
						<Loader2 className="size-4 animate-spin" />
					) : (
						<Trash2 className="size-4" />
					)}
				</button>
			)}
		</Card>
	)
}

function ActionButton({
	busy,
	disabled,
	onClick,
	children,
}: {
	busy: boolean
	disabled: boolean
	onClick: () => void
	children: React.ReactNode
}) {
	return (
		<Button
			size="sm"
			onClick={onClick}
			disabled={disabled}
			className="h-8 shrink-0 gap-1.5 bg-emerald-600 px-3 text-white hover:bg-emerald-500"
		>
			{busy ? <Loader2 className="size-3.5 animate-spin" /> : children}
		</Button>
	)
}
