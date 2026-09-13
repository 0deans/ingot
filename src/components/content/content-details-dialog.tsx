import { openUrl } from "@tauri-apps/plugin-opener"
import {
	ChevronLeft,
	ChevronRight,
	Download,
	ExternalLink,
	Flame,
	Globe,
	Loader2,
	Maximize2,
	X,
} from "lucide-react"
import { marked } from "marked"
import { memo, useEffect, useMemo, useRef, useState } from "react"
import { createPortal } from "react-dom"
import { Button } from "@/components/ui/button"
import { Dialog, DialogContent } from "@/components/ui/dialog"
import { ScrollArea } from "@/components/ui/scroll-area"
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select"
import {
	type ContentSource,
	contentService,
	type UnifiedContentDetails,
	type UnifiedContentItem,
	type UnifiedContentVersion,
} from "@/services/content-service"

marked.setOptions({
	gfm: true,
	breaks: true,
})

interface ContentDetailsDialogProps {
	open: boolean
	onOpenChange: (open: boolean) => void
	item: UnifiedContentItem | null
	onInstall: (item: UnifiedContentItem, specificVersion?: UnifiedContentVersion) => void
}

function formatDownloads(count: number): string {
	if (count >= 1_000_000_000) return `${(count / 1_000_000_000).toFixed(1)}B`
	if (count >= 1_000_000) return `${(count / 1_000_000).toFixed(1)}M`
	if (count >= 1_000) return `${(count / 1_000).toFixed(1)}K`
	return count.toString()
}

function formatBytes(bytes: number): string {
	if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
	if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`
	return `${bytes} B`
}

function formatDate(dateStr: string): string {
	if (!dateStr) return ""
	try {
		const d = new Date(dateStr)
		if (Number.isNaN(d.getTime())) return dateStr
		return d.toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" })
	} catch {
		return dateStr
	}
}

function formatError(err: unknown): string {
	if (!err) return "Failed to load project details"
	if (typeof err === "string") return err
	if (err instanceof Error) return err.message
	if (
		typeof err === "object" &&
		err !== null &&
		"message" in err &&
		typeof (err as { message: unknown }).message === "string"
	) {
		return (err as { message: string }).message
	}
	return String(err)
}

function renderDescriptionHtml(body: string, isHtml: boolean): string {
	if (!body) return ""
	if (isHtml) return body

	try {
		const parsed = marked.parse(body)
		return typeof parsed === "string" ? parsed : body
	} catch (e) {
		console.error("Failed to parse markdown with marked:", e)
		return body
	}
}

export const ContentDetailsDialog = memo(
	({ open, onOpenChange, item, onInstall }: ContentDetailsDialogProps) => {
		const [activeTab, setActiveTab] = useState<"overview" | "versions">("overview")
		const [details, setDetails] = useState<UnifiedContentDetails | null>(null)
		const [isLoading, setIsLoading] = useState(false)
		const [error, setError] = useState<string | null>(null)

		// Version filtering
		const [versionLoaderFilter, setVersionLoaderFilter] = useState("")
		const [versionGameVerFilter, setVersionGameVerFilter] = useState("")

		// Active in-dialog preview photo and Fullscreen Lightbox index
		const [activePhotoIndex, setActivePhotoIndex] = useState(0)
		const [lightboxIndex, setLightboxIndex] = useState<number | null>(null)
		const galleryScrollRef = useRef<HTMLDivElement>(null)

		useEffect(() => {
			if (!open || !item) {
				setDetails(null)
				setError(null)
				setActiveTab("overview")
				setVersionLoaderFilter("")
				setVersionGameVerFilter("")
				setActivePhotoIndex(0)
				setLightboxIndex(null)
				return
			}

			let cancelled = false
			setIsLoading(true)
			setError(null)
			setActivePhotoIndex(0)

			const cleanId = item.id.replace(/^(mr|cf|modrinth|curseforge):/, "")

			contentService
				.getContentDetails(item.source as ContentSource, cleanId)
				.then((d) => {
					if (!cancelled) {
						setDetails(d)
					}
				})
				.catch((err) => {
					if (!cancelled) {
						setError(formatError(err))
					}
				})
				.finally(() => {
					if (!cancelled) {
						setIsLoading(false)
					}
				})

			return () => {
				cancelled = true
			}
		}, [open, item])

		// Keyboard controls for photo lightbox
		useEffect(() => {
			if (lightboxIndex === null || !details?.screenshots) return
			const handleKeyDown = (e: KeyboardEvent) => {
				if (e.key === "Escape") {
					setLightboxIndex(null)
				} else if (e.key === "ArrowLeft") {
					setLightboxIndex((prev) => (prev !== null && prev > 0 ? prev - 1 : prev))
				} else if (e.key === "ArrowRight") {
					setLightboxIndex((prev) =>
						prev !== null && prev < details.screenshots.length - 1 ? prev + 1 : prev,
					)
				}
			}
			window.addEventListener("keydown", handleKeyDown)
			return () => window.removeEventListener("keydown", handleKeyDown)
		}, [lightboxIndex, details?.screenshots])

		// Extract available game versions for dropdown
		const availableGameVersions = useMemo(() => {
			if (!details) return []
			const set = new Set<string>()
			for (const v of details.versions) {
				for (const gv of v.gameVersions) {
					if (gv) set.add(gv)
				}
			}
			return Array.from(set).sort((a, b) => b.localeCompare(a, undefined, { numeric: true }))
		}, [details])

		const filteredVersions = useMemo(() => {
			if (!details) return []
			return details.versions.filter((v) => {
				if (versionLoaderFilter && versionLoaderFilter !== "all") {
					const hasLoader = v.loaders.some(
						(l) => l.toLowerCase() === versionLoaderFilter.toLowerCase(),
					)
					if (!hasLoader) return false
				}
				if (versionGameVerFilter && versionGameVerFilter !== "all") {
					const query = versionGameVerFilter.toLowerCase().trim()
					const hasVer = v.gameVersions.some((gv) => gv.toLowerCase() === query)
					if (!hasVer) return false
				}
				return true
			})
		}, [details, versionLoaderFilter, versionGameVerFilter])

		const handleOpenUrl = async (url: string | null | undefined) => {
			if (!url) return
			try {
				await openUrl(url)
			} catch {
				window.open(url, "_blank")
			}
		}

		// Horizontal scroll buttons for gallery thumbnails
		const scrollGallery = (direction: "left" | "right") => {
			if (!galleryScrollRef.current) return
			const amount = 220
			galleryScrollRef.current.scrollBy({
				left: direction === "left" ? -amount : amount,
				behavior: "smooth",
			})
		}

		// Intercept mouse wheel events with non-passive listener to prevent vertical overscroll bubbling to upper ScrollArea
		useEffect(() => {
			if (activeTab !== "overview" || !details?.screenshots || details.screenshots.length <= 1)
				return
			const el = galleryScrollRef.current
			if (!el) return

			const handleWheel = (e: WheelEvent) => {
				e.preventDefault()
				e.stopPropagation()
				if (e.deltaY !== 0) {
					el.scrollLeft += e.deltaY
				} else if (e.deltaX !== 0) {
					el.scrollLeft += e.deltaX
				}
			}

			el.addEventListener("wheel", handleWheel, { passive: false })
			return () => {
				el.removeEventListener("wheel", handleWheel)
			}
		}, [activeTab, details])

		// Intercept link clicks in rich description
		const handleDescriptionClick = (e: React.MouseEvent<HTMLDivElement>) => {
			const target = (e.target as HTMLElement).closest("a")
			if (target?.href) {
				e.preventDefault()
				openUrl(target.href).catch(console.error)
			}
		}

		if (!item) return null

		return (
			<>
				<Dialog open={open} onOpenChange={onOpenChange}>
					<DialogContent
						showCloseButton={false}
						className="flex h-[88vh] max-h-[88vh] w-full max-w-4xl flex-col gap-0 overflow-hidden p-0 sm:max-w-4xl"
					>
						{/* Header bar */}
						<div className="flex items-center justify-between border-border/40 border-b bg-zinc-950/60 p-5">
							<div className="mr-4 flex min-w-0 flex-1 items-center gap-4">
								{item.iconUrl ? (
									<img
										src={item.iconUrl}
										alt={item.title}
										className="size-14 shrink-0 rounded-xl border border-border/40 bg-zinc-900 object-cover"
									/>
								) : (
									<div className="flex size-14 shrink-0 items-center justify-center rounded-xl border border-border/40 bg-zinc-900 font-bold text-lg text-muted-foreground">
										{item.title.charAt(0).toUpperCase()}
									</div>
								)}

								<div className="flex min-w-0 flex-1 flex-col gap-1">
									<div className="flex items-center gap-2">
										<span
											className={`inline-flex items-center gap-1 rounded px-1.5 py-0.5 font-medium text-[11px] ${
												item.source === "modrinth"
													? "border border-emerald-500/20 bg-emerald-500/15 text-emerald-400"
													: "border border-amber-500/20 bg-amber-500/15 text-amber-400"
											}`}
										>
											{item.source === "curseforge" && <Flame className="size-3 text-orange-400" />}
											{item.source === "modrinth" ? "Modrinth" : "CurseForge"}
										</span>

										<span className="rounded bg-zinc-800 px-1.5 py-0.5 text-[11px] text-muted-foreground capitalize">
											{item.projectType}
										</span>

										<span className="text-muted-foreground text-xs">by {item.author}</span>
									</div>

									<h2 className="truncate font-bold text-foreground text-xl tracking-tight">
										{item.title}
									</h2>

									<div className="flex items-center gap-3 text-muted-foreground text-xs">
										<div className="flex items-center gap-1">
											<Download className="size-3.5" />
											<span>{formatDownloads(item.downloads)} downloads</span>
										</div>
										{details?.follows != null && (
											<div>{details.follows.toLocaleString()} followers</div>
										)}
									</div>
								</div>
							</div>

							<div className="flex shrink-0 items-center gap-2">
								<Button
									size="sm"
									onClick={() => onInstall(item)}
									className="gap-1.5 font-semibold text-xs shadow-sm"
								>
									<Download className="size-3.5" />
									{item.projectType === "modpack" ? "Install Modpack" : "Add to Instance"}
								</Button>

								{item.websiteUrl && (
									<Button
										size="sm"
										variant="outline"
										onClick={() => handleOpenUrl(item.websiteUrl)}
										className="gap-1.5 text-xs"
									>
										<Globe className="size-3.5" />
										Website
									</Button>
								)}

								<button
									type="button"
									onClick={() => onOpenChange(false)}
									aria-label="Close dialog"
									className="rounded-lg p-2 text-muted-foreground transition-colors hover:bg-zinc-800 hover:text-foreground"
								>
									<X className="size-4" />
								</button>
							</div>
						</div>

						{/* Navigation Tabs */}
						<div className="flex items-center gap-2 border-border/40 border-b bg-zinc-950/40 px-5 pt-2">
							<button
								type="button"
								onClick={() => setActiveTab("overview")}
								className={`border-b-2 px-3 pb-2 font-medium text-xs transition-colors ${
									activeTab === "overview"
										? "border-primary text-primary"
										: "border-transparent text-muted-foreground hover:text-foreground"
								}`}
							>
								Overview
							</button>
							<button
								type="button"
								onClick={() => setActiveTab("versions")}
								className={`border-b-2 px-3 pb-2 font-medium text-xs transition-colors ${
									activeTab === "versions"
										? "border-primary text-primary"
										: "border-transparent text-muted-foreground hover:text-foreground"
								}`}
							>
								Versions & Files {details ? `(${details.versions.length})` : ""}
							</button>
						</div>

						{/* Tab Content */}
						<div className="flex min-h-0 flex-1 flex-col bg-background/50">
							{isLoading ? (
								<div className="flex flex-1 items-center justify-center gap-2 text-muted-foreground text-sm">
									<Loader2 className="size-5 animate-spin text-primary" />
									<span>Loading project details...</span>
								</div>
							) : error ? (
								<div className="flex flex-1 flex-col items-center justify-center gap-3 p-8 text-center">
									<p className="font-medium text-destructive text-sm">{error}</p>
									<Button
										size="sm"
										variant="outline"
										onClick={() => {
											setIsLoading(true)
											setError(null)
											const cleanId = item.id.replace(/^(mr|cf|modrinth|curseforge):/, "")
											contentService
												.getContentDetails(item.source as ContentSource, cleanId)
												.then(setDetails)
												.catch((err) => setError(formatError(err)))
												.finally(() => setIsLoading(false))
										}}
									>
										Try Again
									</Button>
								</div>
							) : activeTab === "overview" ? (
								<ScrollArea className="flex-1" scrollFade>
									<div className="flex flex-col gap-6 p-6">
										{/* Gallery: Featured Large Preview + Scrollable Thumbnail Strip */}
										{details?.screenshots && details.screenshots.length > 0 && (
											<div className="flex w-full min-w-0 max-w-full flex-col gap-3">
												<div className="flex items-center justify-between">
													<h3 className="font-semibold text-foreground text-sm">
														Gallery ({details.screenshots.length})
													</h3>
													<span className="text-[11px] text-muted-foreground">
														Click image to open in fullscreen
													</span>
												</div>

												{/* Featured Main Image Preview */}
												<div className="group relative aspect-video w-full overflow-hidden rounded-xl border border-border/40 bg-zinc-950 shadow-md">
													<img
														src={
															details.screenshots[
																activePhotoIndex < details.screenshots.length ? activePhotoIndex : 0
															].url
														}
														alt={
															details.screenshots[
																activePhotoIndex < details.screenshots.length ? activePhotoIndex : 0
															].title || "Featured screenshot"
														}
														className="size-full object-cover transition-transform duration-300 group-hover:scale-[1.01]"
													/>

													{/* Hover View Button Overlay */}
													<button
														type="button"
														onClick={() =>
															setLightboxIndex(
																activePhotoIndex < details.screenshots.length
																	? activePhotoIndex
																	: 0,
															)
														}
														className="absolute inset-0 flex items-center justify-center bg-black/30 opacity-0 transition-opacity duration-200 group-hover:opacity-100"
														aria-label="View screenshot in fullscreen"
													>
														<div className="flex items-center gap-2 rounded-lg bg-black/80 px-4 py-2 font-medium text-white text-xs shadow-xl backdrop-blur-xs transition-transform duration-200 hover:scale-105">
															<Maximize2 className="size-4" />
															<span>View Fullscreen</span>
														</div>
													</button>

													{/* Bottom Caption Overlay */}
													<div className="absolute inset-x-0 bottom-0 flex items-center justify-between bg-gradient-to-t from-black/80 via-black/40 to-transparent p-3 text-white">
														<span className="truncate font-medium text-xs">
															{details.screenshots[
																activePhotoIndex < details.screenshots.length ? activePhotoIndex : 0
															].title || item.title}
														</span>
														<span className="rounded bg-black/60 px-2 py-0.5 font-mono text-[11px] text-zinc-300">
															{activePhotoIndex + 1} / {details.screenshots.length}
														</span>
													</div>
												</div>

												{/* Horizontal Scrollable Thumbnails with Chevrons */}
												{details.screenshots.length > 1 && (
													<div className="relative flex w-full min-w-0 max-w-full items-center gap-1.5">
														<Button
															type="button"
															size="icon-sm"
															variant="outline"
															onClick={() => scrollGallery("left")}
															className="size-8 shrink-0 rounded-lg text-muted-foreground hover:text-foreground"
															aria-label="Scroll gallery left"
														>
															<ChevronLeft className="size-4" />
														</Button>

														<div
															ref={galleryScrollRef}
															className="flex min-w-0 flex-1 gap-2.5 overflow-x-auto overflow-y-hidden overscroll-contain py-1 [-ms-overflow-style:none] [scrollbar-width:none] [&::-webkit-scrollbar]:hidden"
														>
															{details.screenshots.map((s, idx) => (
																<button
																	// biome-ignore lint/suspicious/noArrayIndexKey: Screenshot index
																	key={idx}
																	type="button"
																	onClick={() => setActivePhotoIndex(idx)}
																	className={`group relative h-18 w-28 shrink-0 overflow-hidden rounded-lg border-2 transition-all focus:outline-hidden ${
																		idx === activePhotoIndex
																			? "border-primary opacity-100 shadow-md"
																			: "border-transparent opacity-60 hover:border-white/20 hover:opacity-90"
																	}`}
																>
																	<img
																		src={s.url}
																		alt={s.title || "Thumbnail"}
																		className="size-full object-cover"
																	/>
																	{s.title && (
																		<div className="absolute inset-x-0 bottom-0 truncate bg-gradient-to-t from-black/80 to-transparent p-1 text-left text-[9px] text-white">
																			{s.title}
																		</div>
																	)}
																</button>
															))}
														</div>

														<Button
															type="button"
															size="icon-sm"
															variant="outline"
															onClick={() => scrollGallery("right")}
															className="size-8 shrink-0 rounded-lg text-muted-foreground hover:text-foreground"
															aria-label="Scroll gallery right"
														>
															<ChevronRight className="size-4" />
														</Button>
													</div>
												)}
											</div>
										)}

										{/* External Links Bar */}
										{(details?.issuesUrl || details?.sourceUrl || details?.wikiUrl) && (
											<div className="flex flex-wrap items-center gap-2">
												{details.sourceUrl && (
													<Button
														size="sm"
														variant="outline"
														onClick={() => handleOpenUrl(details.sourceUrl)}
														className="h-7 gap-1.5 text-muted-foreground text-xs hover:text-foreground"
													>
														<ExternalLink className="size-3" />
														Source Code
													</Button>
												)}
												{details.issuesUrl && (
													<Button
														size="sm"
														variant="outline"
														onClick={() => handleOpenUrl(details.issuesUrl)}
														className="h-7 gap-1.5 text-muted-foreground text-xs hover:text-foreground"
													>
														<ExternalLink className="size-3" />
														Issue Tracker
													</Button>
												)}
												{details.wikiUrl && (
													<Button
														size="sm"
														variant="outline"
														onClick={() => handleOpenUrl(details.wikiUrl)}
														className="h-7 gap-1.5 text-muted-foreground text-xs hover:text-foreground"
													>
														<ExternalLink className="size-3" />
														Wiki
													</Button>
												)}
											</div>
										)}

										{/* Rich Description (Markdown & HTML Supported) */}
										<div className="flex flex-col gap-2">
											<h3 className="font-semibold text-foreground text-sm">Description</h3>
											{details?.body ? (
												<section
													className="prose prose-invert prose-sm max-w-none space-y-3 text-muted-foreground text-xs leading-relaxed [&_a]:text-primary [&_a]:underline hover:[&_a]:text-primary/80 [&_blockquote]:my-3 [&_blockquote]:rounded-r-lg [&_blockquote]:border-primary/70 [&_blockquote]:border-l-4 [&_blockquote]:bg-primary/5 [&_blockquote]:py-1 [&_blockquote]:pl-4 [&_code]:rounded [&_code]:bg-zinc-800/80 [&_code]:px-1.5 [&_code]:py-0.5 [&_code]:font-mono [&_code]:text-[11px] [&_code]:text-primary-foreground [&_details]:my-3 [&_details]:rounded-lg [&_details]:border [&_details]:border-border/40 [&_details]:bg-zinc-900/50 [&_details]:p-3 [&_h1]:mt-6 [&_h1]:mb-3 [&_h1]:font-bold [&_h1]:text-foreground [&_h1]:text-lg [&_h2]:mt-5 [&_h2]:mb-2 [&_h2]:font-bold [&_h2]:text-base [&_h2]:text-foreground [&_h3]:mt-4 [&_h3]:mb-1 [&_h3]:font-semibold [&_h3]:text-foreground [&_h3]:text-sm [&_img]:my-3 [&_img]:max-w-full [&_img]:rounded-lg [&_img]:border [&_img]:border-border/40 [&_li]:my-1 [&_p]:my-2 [&_pre]:my-3 [&_pre]:overflow-x-auto [&_pre]:rounded-lg [&_pre]:bg-zinc-900/90 [&_pre]:p-4 [&_summary]:cursor-pointer [&_summary]:font-semibold [&_summary]:text-foreground hover:[&_summary]:text-primary [&_table]:my-4 [&_table]:w-full [&_table]:border-collapse [&_td]:border [&_td]:border-border/60 [&_td]:p-2.5 [&_th]:border [&_th]:border-border [&_th]:bg-zinc-800/80 [&_th]:p-2.5 [&_th]:text-left [&_th]:font-semibold"
													onClick={handleDescriptionClick}
													onKeyDown={(e) => {
														if (e.key === "Enter") {
															const target = (e.target as HTMLElement).closest("a")
															if (target?.href) {
																e.preventDefault()
																openUrl(target.href).catch(console.error)
															}
														}
													}}
													aria-label="Project description"
													// biome-ignore lint/security/noDangerouslySetInnerHtml: Sanitized markdown & HTML parser
													dangerouslySetInnerHTML={{
														__html: renderDescriptionHtml(
															details.body,
															item.source === "curseforge" || details.body.trim().startsWith("<"),
														),
													}}
												/>
											) : (
												<p className="text-muted-foreground text-xs">
													{details?.description || "No description provided."}
												</p>
											)}
										</div>
									</div>
								</ScrollArea>
							) : (
								/* Versions & Files Tab */
								<div className="flex min-h-0 flex-1 flex-col">
									{/* Modern Version filters with Select components */}
									<div className="flex flex-wrap items-center gap-3 border-border/40 border-b bg-zinc-950/20 px-6 py-3">
										<div className="flex items-center gap-2">
											<span className="text-muted-foreground text-xs">Loader:</span>
											<Select
												value={versionLoaderFilter || "all"}
												onValueChange={(val) =>
													setVersionLoaderFilter(!val || val === "all" ? "" : val)
												}
											>
												<SelectTrigger className="h-8 w-32">
													<SelectValue placeholder="All Loaders" />
												</SelectTrigger>
												<SelectContent>
													<SelectItem value="all">All Loaders</SelectItem>
													<SelectItem value="fabric">Fabric</SelectItem>
													<SelectItem value="forge">Forge</SelectItem>
													<SelectItem value="neoforge">NeoForge</SelectItem>
													<SelectItem value="quilt">Quilt</SelectItem>
												</SelectContent>
											</Select>
										</div>

										<div className="flex items-center gap-2">
											<span className="text-muted-foreground text-xs">Game Version:</span>
											<Select
												value={versionGameVerFilter || "all"}
												onValueChange={(val) =>
													setVersionGameVerFilter(!val || val === "all" ? "" : val)
												}
											>
												<SelectTrigger className="h-8 w-36">
													<SelectValue placeholder="All Versions" />
												</SelectTrigger>
												<SelectContent>
													<SelectItem value="all">All Versions</SelectItem>
													{availableGameVersions.map((gv) => (
														<SelectItem key={gv} value={gv}>
															{gv}
														</SelectItem>
													))}
												</SelectContent>
											</Select>
										</div>

										{(versionLoaderFilter || versionGameVerFilter) && (
											<Button
												size="sm"
												variant="ghost"
												onClick={() => {
													setVersionLoaderFilter("")
													setVersionGameVerFilter("")
												}}
												className="h-8 gap-1 text-muted-foreground text-xs hover:text-foreground"
											>
												<X className="size-3" />
												Clear
											</Button>
										)}

										<span className="ml-auto text-muted-foreground text-xs">
											Showing {filteredVersions.length} of {details?.versions.length ?? 0} versions
										</span>
									</div>

									<ScrollArea className="flex-1" scrollFade>
										<div className="flex flex-col gap-3 p-6">
											{filteredVersions.length === 0 ? (
												<div className="py-12 text-center text-muted-foreground text-xs">
													No versions match your filter criteria.
												</div>
											) : (
												filteredVersions.map((ver) => (
													<div
														key={ver.id}
														className="flex flex-col gap-2 rounded-xl border border-border/40 bg-zinc-900/40 p-4 transition-all hover:border-border hover:bg-zinc-900/60"
													>
														<div className="flex items-start justify-between gap-3">
															<div className="flex min-w-0 flex-1 flex-col gap-1">
																<div className="flex items-center gap-2">
																	<span
																		className={`rounded px-1.5 py-0.5 font-medium text-[10px] uppercase ${
																			ver.versionType === "release"
																				? "border border-emerald-500/20 bg-emerald-500/15 text-emerald-400"
																				: ver.versionType === "beta"
																					? "border border-amber-500/20 bg-amber-500/15 text-amber-400"
																					: "border border-red-500/20 bg-red-500/15 text-red-400"
																		}`}
																	>
																		{ver.versionType}
																	</span>
																	<h4 className="truncate font-semibold text-foreground text-sm">
																		{ver.name}
																	</h4>
																</div>

																<div className="mt-1 flex flex-wrap items-center gap-1.5 text-muted-foreground text-xs">
																	<span className="font-mono text-zinc-400">{ver.filename}</span>
																	{ver.sizeBytes > 0 && (
																		<>
																			<span>•</span>
																			<span>{formatBytes(ver.sizeBytes)}</span>
																		</>
																	)}
																	{ver.datePublished && (
																		<>
																			<span>•</span>
																			<span>{formatDate(ver.datePublished)}</span>
																		</>
																	)}
																</div>
															</div>

															<Button
																size="sm"
																variant="outline"
																onClick={() => onInstall(item, ver)}
																className="shrink-0 gap-1.5 text-xs transition-colors hover:bg-primary hover:text-primary-foreground"
															>
																<Download className="size-3.5" />
																Install
															</Button>
														</div>

														<div className="mt-1 flex flex-wrap items-center gap-1.5 border-border/20 border-t pt-2">
															{ver.loaders.map((l) => (
																<span
																	key={l}
																	className="rounded bg-zinc-800 px-1.5 py-0.5 font-mono text-[10px] text-zinc-300 capitalize"
																>
																	{l}
																</span>
															))}
															{ver.gameVersions.map((gv) => (
																<span
																	key={gv}
																	className="rounded bg-primary/10 px-1.5 py-0.5 font-mono text-[10px] text-primary"
																>
																	{gv}
																</span>
															))}
														</div>
													</div>
												))
											)}
										</div>
									</ScrollArea>
								</div>
							)}
						</div>
					</DialogContent>
				</Dialog>

				{/* Fullscreen Lightbox Portaled Directly to document.body to stay on top of the modal without covering Titlebar */}
				{lightboxIndex !== null &&
					details?.screenshots?.[lightboxIndex] &&
					typeof document !== "undefined" &&
					createPortal(
						<div className="fade-in-0 fixed inset-x-0 top-10 bottom-0 z-60 flex animate-in flex-col justify-between bg-black/95 p-4 backdrop-blur-md duration-200">
							{/* Top Bar */}
							<div className="flex items-center justify-between text-white">
								<div className="flex items-center gap-3">
									<span className="rounded-md bg-white/10 px-2.5 py-1 font-medium text-xs">
										{lightboxIndex + 1} / {details.screenshots.length}
									</span>
									<span className="truncate font-medium text-sm text-zinc-300">
										{details.screenshots[lightboxIndex].title || item.title}
									</span>
								</div>
								<div className="flex items-center gap-2">
									<Button
										size="sm"
										variant="ghost"
										onClick={() => handleOpenUrl(details.screenshots[lightboxIndex].url)}
										className="h-8 gap-1.5 text-xs text-zinc-300 hover:text-white"
									>
										<ExternalLink className="size-3.5" />
										Open Original
									</Button>
									<button
										type="button"
										onClick={() => setLightboxIndex(null)}
										aria-label="Close image preview"
										className="rounded-lg p-1.5 text-zinc-400 transition-colors hover:bg-white/10 hover:text-white"
									>
										<X className="size-5" />
									</button>
								</div>
							</div>

							{/* Center Image with Previous/Next Controls */}
							<div className="relative flex flex-1 items-center justify-center py-4">
								{lightboxIndex > 0 && (
									<button
										type="button"
										onClick={() => setLightboxIndex(lightboxIndex - 1)}
										className="absolute top-1/2 left-4 -translate-y-1/2 rounded-full bg-black/60 p-3 text-white shadow-lg backdrop-blur-xs transition-all hover:bg-black/90"
										aria-label="Previous image"
									>
										<ChevronLeft className="size-6" />
									</button>
								)}

								<img
									src={details.screenshots[lightboxIndex].url}
									alt={details.screenshots[lightboxIndex].title || "Screenshot"}
									className="max-h-[72vh] max-w-[85vw] rounded-lg object-contain shadow-2xl transition-all"
								/>

								{lightboxIndex < details.screenshots.length - 1 && (
									<button
										type="button"
										onClick={() => setLightboxIndex(lightboxIndex + 1)}
										className="absolute top-1/2 right-4 -translate-y-1/2 rounded-full bg-black/60 p-3 text-white shadow-lg backdrop-blur-xs transition-all hover:bg-black/90"
										aria-label="Next image"
									>
										<ChevronRight className="size-6" />
									</button>
								)}
							</div>

							{/* Bottom Thumbnails Strip */}
							<div className="scrollbar-none flex justify-center gap-2 overflow-x-auto py-2">
								{details.screenshots.map((s, idx) => (
									<button
										// biome-ignore lint/suspicious/noArrayIndexKey: Thumbnail gallery index
										key={idx}
										type="button"
										onClick={() => setLightboxIndex(idx)}
										className={`relative h-14 w-24 shrink-0 overflow-hidden rounded-md border-2 transition-all focus:outline-hidden ${
											idx === lightboxIndex
												? "border-primary opacity-100 shadow-md"
												: "border-transparent opacity-50 hover:border-white/20 hover:opacity-80"
										}`}
									>
										<img src={s.url} alt="" className="size-full object-cover" />
									</button>
								))}
							</div>
						</div>,
						document.body,
					)}
			</>
		)
	},
)

ContentDetailsDialog.displayName = "ContentDetailsDialog"
