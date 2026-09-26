import { Check, Download, ExternalLink, Loader2, Package } from "lucide-react"
import { marked } from "marked"
import { useMemo, useState } from "react"
import type { PluginProject, PluginVersion } from "@/bindings"
import { Button } from "@/components/ui/button"
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog"
import { Switch } from "@/components/ui/switch"
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
				"flex shrink-0 items-center justify-center overflow-hidden rounded-xl border border-zinc-800 bg-zinc-900",
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
				<span className="font-bold text-sm text-zinc-500">{name.slice(0, 1).toUpperCase()}</span>
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
						<div className="flex flex-col gap-4 border-zinc-800/80 border-b p-4 pt-3 sm:p-5">
							<div className="flex items-start gap-3.5 pr-8">
								<PluginIcon url={shown.iconUrl} name={shown.title} className="size-14" />
								<div className="min-w-0 flex-1">
									<DialogTitle className="truncate font-bold text-lg text-zinc-50">
										{shown.title}
									</DialogTitle>
									<p className="text-xs text-zinc-500">
										by {shown.author} · {formatCount(shown.downloads)} downloads ·{" "}
										{shown.source === "hangar" ? "Hangar" : "Modrinth"}
									</p>
									<p className="mt-1.5 text-sm text-zinc-300 leading-relaxed">
										{shown.description}
									</p>
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
											? "bg-emerald-500/15 text-emerald-300"
											: "bg-emerald-600 text-white hover:bg-emerald-500",
									)}
								>
									{installing ? (
										<Loader2 className="size-4 animate-spin" />
									) : installed ? (
										<Check className="size-4" />
									) : (
										<Download className="size-4" />
									)}
									{installing ? "Installing..." : installed ? "Installed" : "Install"}
								</Button>
								<a
									href={shown.pageUrl}
									target="_blank"
									rel="noopener noreferrer"
									className="flex h-11 items-center gap-1.5 rounded-xl border border-zinc-800 px-4 font-medium text-sm text-zinc-300 hover:bg-zinc-900"
								>
									<ExternalLink className="size-4" /> Page
								</a>
							</div>
							{versions.isSuccess && !hasCompatible && (
								<p className="text-amber-300 text-xs">
									No version for Minecraft {gameVersion} yet. You can still pick an older one under
									Versions, but it may not work.
								</p>
							)}
						</div>

						{/* Pinned above the scrolling content */}
						<div className="shrink-0 px-4 pt-3 sm:px-5">
							<Segmented
								value={tab}
								onChange={setTab}
								options={[
									{ value: "about", label: "About" },
									{ value: "versions", label: "Versions" },
								]}
							/>
						</div>
						<FadeScroll className="flex min-h-0 flex-1 flex-col gap-3 p-4 sm:p-5">
							{tab === "about" &&
								(page.isLoading ? (
									<div className="flex justify-center py-10">
										<Loader2 className="size-5 animate-spin text-zinc-500" />
									</div>
								) : html ? (
									<div
										className="plugin-page text-sm text-zinc-300 leading-relaxed"
										// biome-ignore lint/security/noDangerouslySetInnerHtml: sanitized with sanitizeHtml
										dangerouslySetInnerHTML={{ __html: html }}
									/>
								) : (
									<p className="text-sm text-zinc-500">No description.</p>
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
	return (
		<div className="flex flex-col gap-2">
			<div className="flex items-center justify-between gap-3 px-1 text-xs text-zinc-400">
				Only versions for Minecraft {gameVersion}
				<Switch
					checked={compatibleOnly}
					onCheckedChange={onCompatibleChange}
					aria-label={`Only versions for Minecraft ${gameVersion}`}
				/>
			</div>
			{error ? <ErrorNote>{String(error)}</ErrorNote> : null}
			{loading ? (
				<div className="flex justify-center py-8">
					<Loader2 className="size-5 animate-spin text-zinc-500" />
				</div>
			) : versions.length === 0 ? (
				<EmptyState icon={Package} title="No versions" description="Try showing all versions." />
			) : (
				<ul className="divide-y divide-zinc-800/70 rounded-xl border border-zinc-800">
					{versions.slice(0, 40).map((v) => (
						<li key={v.id} className="flex items-center gap-3 px-3 py-2.5">
							<div className="min-w-0 flex-1">
								<p className="flex items-center gap-2 truncate font-medium text-sm text-zinc-100">
									{v.versionNumber}
									{v.channel !== "release" && (
										<span className="rounded-full border border-amber-500/30 bg-amber-500/10 px-1.5 text-[10px] text-amber-300">
											{v.channel}
										</span>
									)}
								</p>
								<p className="truncate text-[11px] text-zinc-500">
									{summarizeVersions(v.gameVersions)}
									{v.size > 0 && ` · ${formatBytes(v.size)}`}
									{v.date && ` · ${new Date(v.date).toLocaleDateString()}`}
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
									Install
								</Button>
							) : (
								v.externalUrl && (
									<a
										href={v.externalUrl}
										target="_blank"
										rel="noopener noreferrer"
										className="text-emerald-400 text-xs hover:underline"
									>
										Website
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
	if (list.length === 0) return "Any version"
	if (list.length <= 3) return list.join(", ")
	return `${list[0]} – ${list[list.length - 1]}`
}
