import i18n from "i18next"
import { Check, Download, ExternalLink, Package } from "lucide-react"
import { marked } from "marked"
import { useMemo, useState } from "react"
import { useTranslation } from "react-i18next"
import type { PluginProject, PluginVersion } from "@/bindings"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog"
import { Spinner } from "@/components/ui/spinner"
import { Switch } from "@/components/ui/switch"
import { formatDate } from "@/lib/format"
import { formatBytes, formatCount } from "@/lib/minecraft"
import { sanitizeHtml } from "@/lib/sanitize-html"
import { cn } from "@/lib/utils"
import { usePluginPage, usePluginVersions } from "@/services/server-data"
import { EmptyState, ErrorNote, FadeScroll, Segmented, useSticky } from "../shared/primitives"

export function PluginIcon({
	url,
	name,
	className,
}: {
	url?: string | null
	name: string
	className?: string
}) {
	const [failed, setFailed] = useState(false)
	return (
		<div
			className={cn(
				"flex shrink-0 items-center justify-center overflow-hidden rounded-xl border border-border bg-card",
				className,
			)}
		>
			{url && !failed ? (
				<img
					src={url}
					alt=""
					loading="lazy"
					onError={() => setFailed(true)}
					className="size-full object-cover"
				/>
			) : (
				<span className="font-bold text-muted-foreground text-sm">
					{name.slice(0, 1).toUpperCase()}
				</span>
			)}
		</div>
	)
}

export function PluginDetailsSheet({
	serverId,
	gameVersion,
	project,
	installed,
	installing,
	onInstall,
	onClose,
}: {
	serverId: string
	gameVersion: string
	project: PluginProject | null
	installed: boolean
	installing: boolean
	onInstall: (project: PluginProject, version?: PluginVersion) => void
	onClose: () => void
}) {
	const { t } = useTranslation()
	const [tab, setTab] = useState<"about" | "versions">("about")
	const [compatibleOnly, setCompatibleOnly] = useState(true)
	const open = Boolean(project)
	const shown = useSticky(project)
	const source = shown?.source ?? "modrinth"
	const page = usePluginPage(source, shown?.id ?? null)
	const versions = usePluginVersions(serverId, source, shown?.id ?? null, compatibleOnly)

	const html = useMemo(() => {
		if (!page.data) return ""
		const parsed = marked.parse(page.data, { gfm: true, breaks: true, async: false })
		return sanitizeHtml(typeof parsed === "string" ? parsed : "")
	}, [page.data])

	const hasCompatible = (versions.data ?? []).some((v) => v.downloadUrl)

	return (
		<Dialog open={open} onOpenChange={(o) => !o && onClose()}>
			<DialogContent className="flex max-h-[90vh] flex-col gap-0 overflow-hidden p-0 sm:max-w-2xl">
				{shown && (
					<>
						<div className="flex flex-col gap-4 border-border/80 border-b p-4 pt-3 sm:p-5">
							<div className="flex items-start gap-3.5 pr-8">
								<PluginIcon url={shown.iconUrl} name={shown.title} className="size-14" />
								<div className="min-w-0 flex-1">
									<DialogTitle className="truncate font-bold text-foreground text-lg">
										{shown.title}
									</DialogTitle>
									<p className="text-muted-foreground text-xs">
										{t("pluginDetails.byline", {
											author: shown.author,
											downloads: formatCount(shown.downloads),
										})}{" "}
										· {shown.source === "hangar" ? "Hangar" : "Modrinth"}
									</p>
									<p className="mt-1.5 text-foreground/80 text-sm leading-relaxed">
										{shown.description}
									</p>
									{shown.playersNeedIt && (
										<p className="mt-2 rounded-xl bg-warning/10 px-3 py-2 text-warning text-xs leading-relaxed">
											{t("pluginDetails.playersNeedIt")}
										</p>
									)}
								</div>
							</div>
							<div className="flex gap-2">
								<Button
									disabled={
										installed ||
										installing ||
										(versions.isSuccess && !hasCompatible && compatibleOnly)
									}
									onClick={() => onInstall(shown)}
									className={cn(
										"h-11 flex-1 gap-2 rounded-xl font-semibold",
										installed
											? "bg-primary/15 text-primary"
											: "bg-primary text-primary-foreground hover:bg-primary",
									)}
								>
									{installing ? (
										<Spinner className="size-4" />
									) : installed ? (
										<Check className="size-4" />
									) : (
										<Download className="size-4" />
									)}
									{installing
										? t("pluginDetails.installing")
										: installed
											? t("common.installed")
											: t("common.install")}
								</Button>
								<a
									href={shown.pageUrl}
									target="_blank"
									rel="noopener noreferrer"
									className="flex h-11 items-center gap-1.5 rounded-xl border border-border px-4 font-medium text-foreground/80 text-sm hover:bg-card"
								>
									<ExternalLink className="size-4" /> {t("pluginDetails.page")}
								</a>
							</div>
							{versions.isSuccess && !hasCompatible && (
								<p className="text-warning text-xs">
									{t("pluginDetails.noCompatible", { version: gameVersion })}
								</p>
							)}
						</div>

						{/* Pinned above the scrolling content */}
						<div className="shrink-0 px-4 pt-3 sm:px-5">
							<Segmented
								value={tab}
								onChange={setTab}
								options={[
									{ value: "about", label: t("pluginDetails.about") },
									{ value: "versions", label: t("pluginDetails.versions") },
								]}
							/>
						</div>
						<FadeScroll className="flex min-h-0 flex-1 flex-col gap-3 p-4 sm:p-5">
							{tab === "about" &&
								(page.isLoading ? (
									<div className="flex justify-center py-10">
										<Spinner className="size-5 text-muted-foreground" />
									</div>
								) : html ? (
									<div
										className="plugin-page text-foreground/80 text-sm leading-relaxed"
										// biome-ignore lint/security/noDangerouslySetInnerHtml: sanitized with sanitizeHtml
										dangerouslySetInnerHTML={{ __html: html }}
									/>
								) : (
									<p className="text-muted-foreground text-sm">
										{t("pluginDetails.noDescription")}
									</p>
								))}
							{tab === "versions" && (
								<VersionList
									versions={versions.data ?? []}
									loading={versions.isLoading}
									error={versions.error}
									gameVersion={gameVersion}
									compatibleOnly={compatibleOnly}
									onCompatibleChange={setCompatibleOnly}
									installing={installing}
									onInstall={(v) => onInstall(shown, v)}
								/>
							)}
						</FadeScroll>
					</>
				)}
			</DialogContent>
		</Dialog>
	)
}

function VersionList({
	versions,
	loading,
	error,
	gameVersion,
	compatibleOnly,
	onCompatibleChange,
	installing,
	onInstall,
}: {
	versions: PluginVersion[]
	loading: boolean
	error: unknown
	gameVersion: string
	compatibleOnly: boolean
	onCompatibleChange: (value: boolean) => void
	installing: boolean
	onInstall: (version: PluginVersion) => void
}) {
	const { t } = useTranslation()
	return (
		<div className="flex flex-col gap-2">
			<div className="flex items-center justify-between gap-3 px-1 text-muted-foreground text-xs">
				{t("pluginDetails.onlyFor", { version: gameVersion })}
				<Switch
					checked={compatibleOnly}
					onCheckedChange={onCompatibleChange}
					aria-label={t("pluginDetails.onlyFor", { version: gameVersion })}
				/>
			</div>
			{error ? <ErrorNote>{String(error)}</ErrorNote> : null}
			{loading ? (
				<div className="flex justify-center py-8">
					<Spinner className="size-5 text-muted-foreground" />
				</div>
			) : versions.length === 0 ? (
				<EmptyState
					icon={Package}
					title={t("pluginDetails.noVersions")}
					description={t("pluginDetails.noVersionsHint")}
				/>
			) : (
				<ul className="divide-y divide-border/70 rounded-xl border border-border">
					{versions.slice(0, 40).map((v) => (
						<li key={v.id} className="flex items-center gap-3 px-3 py-2.5">
							<div className="min-w-0 flex-1">
								<p className="flex items-center gap-2 truncate font-medium text-foreground text-sm">
									{v.versionNumber}
									{v.channel !== "release" && (
										<Badge variant="outline" className="border-warning/30 text-warning">
											{v.channel}
										</Badge>
									)}
								</p>
								<p className="truncate text-[11px] text-muted-foreground">
									{summarizeVersions(v.gameVersions)}
									{v.size > 0 && ` · ${formatBytes(v.size)}`}
									{v.date && ` · ${formatDate(v.date)}`}
								</p>
							</div>
							{v.downloadUrl ? (
								<Button
									size="sm"
									variant="outline"
									disabled={installing}
									onClick={() => onInstall(v)}
									className="h-8 rounded-lg"
								>
									{t("common.install")}
								</Button>
							) : (
								v.externalUrl && (
									<a
										href={v.externalUrl}
										target="_blank"
										rel="noopener noreferrer"
										className="text-primary text-xs hover:underline"
									>
										{t("pluginDetails.website")}
									</a>
								)
							)}
						</li>
					))}
				</ul>
			)}
		</div>
	)
}

/** ["1.20", ..., "26.2"] -> "1.20 – 26.2" */
function summarizeVersions(list: string[]): string {
	if (list.length === 0) return i18n.t("pluginDetails.anyVersion")
	if (list.length <= 3) return list.join(", ")
	return `${list[0]} – ${list[list.length - 1]}`
}
