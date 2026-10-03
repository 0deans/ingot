import { openUrl } from "@tauri-apps/plugin-opener"
import type { TFunction } from "i18next"
import i18n from "i18next"
import {
	ChevronLeft,
	ChevronRight,
	Download,
	ExternalLink,
	Flame,
	Globe,
	Maximize2,
	X,
} from "lucide-react"
import { marked } from "marked"
import { memo, useEffect, useMemo, useRef, useState } from "react"
import { createPortal } from "react-dom"
import { useTranslation } from "react-i18next"
import { ScrollArea } from "@/components/common/scroll-area"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Dialog, DialogContent } from "@/components/ui/dialog"
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select"
import { Spinner } from "@/components/ui/spinner"
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { formatBytes, formatCount, formatDate as formatLocalDate, formatNumber } from "@/lib/format"
import { sanitizeHtml } from "@/lib/sanitize-html"
import { cn } from "@/lib/utils"
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

const formatDownloads = formatCount

const VERSION_FILTER_LOADERS = [
	{ value: "fabric", label: "Fabric" },
	{ value: "forge", label: "Forge" },
	{ value: "neoforge", label: "NeoForge" },
	{ value: "quilt", label: "Quilt" },
]

/** "mod", "resourcepack"... in the user's language (unknown types as they are) */
function contentType(t: TFunction, type: string): string {
	return ["mod", "modpack", "resourcepack", "shader", "datapack", "plugin"].includes(type)
		? t(`contentTypes.${type as "mod"}`)
		: type
}

function releaseType(t: TFunction, type: string): string {
	return ["release", "beta", "alpha"].includes(type) ? t(`releaseTypes.${type as "release"}`) : type
}

function formatDate(dateStr: string): string {
	return dateStr ? formatLocalDate(dateStr) || dateStr : ""
}

function formatError(err: unknown): string {
	if (!err) return i18n.t("contentDetails.loadFailed")
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

/**
 * Description HTML from Modrinth/CurseForge, safe to render. CurseForge sends HTML and
 * Markdown may embed it, so every path goes through the sanitizer: inside the app's
 * WebView, script in a description could reach the Tauri backend.
 */
function renderDescriptionHtml(body: string, isHtml: boolean): string {
	if (!body) return ""
	if (isHtml) return sanitizeHtml(body)

	try {
		const parsed = marked.parse(body)
		return sanitizeHtml(typeof parsed === "string" ? parsed : body)
	} catch (e) {
		console.error("Failed to parse markdown with marked:", e)
		return sanitizeHtml(body)
	}
}

export const ContentDetailsDialog = memo(
	({ open, onOpenChange, item, onInstall }: ContentDetailsDialogProps) => {
		const { t } = useTranslation()
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
		// Sanitizing parses the whole description; do it once per description, not per render
		const descriptionHtml = useMemo(
			() =>
				details?.body
					? renderDescriptionHtml(
							details.body,
							item?.source === "curseforge" || details.body.trim().startsWith("<"),
						)
					: "",
			[details?.body, item?.source],
		)

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

		// Select items give each filter's trigger its label ("All loaders", not "all")
		const loaderFilterItems = [
			{ value: "all", label: t("modpacks.filters.allLoaders") },
			...VERSION_FILTER_LOADERS,
		]
		const gameVersionFilterItems = [
			{ value: "all", label: t("modpacks.filters.allVersions") },
			...availableGameVersions.map((gv) => ({ value: gv, label: gv })),
		]

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
						<div className="flex items-center justify-between border-border/40 border-b bg-background/60 p-5">
							<div className="mr-4 flex min-w-0 flex-1 items-center gap-4">
								{item.iconUrl ? (
									<img
										src={item.iconUrl}
										alt={item.title}
										className="size-14 shrink-0 rounded-xl border border-border/40 bg-card object-cover"
									/>
								) : (
									<div className="flex size-14 shrink-0 items-center justify-center rounded-xl border border-border/40 bg-card font-bold text-lg text-muted-foreground">
										{item.title.charAt(0).toUpperCase()}
									</div>
								)}

								<div className="flex min-w-0 flex-1 flex-col gap-1">
									<div className="flex items-center gap-2">
										<span
											className={`inline-flex items-center gap-1 rounded px-1.5 py-0.5 font-medium text-2xs ${
												item.source === "modrinth"
													? "border border-primary/20 bg-primary/15 text-primary"
													: "border border-warning/20 bg-warning/15 text-warning"
											}`}
										>
											{item.source === "curseforge" && <Flame className="size-3 text-orange-400" />}
											{item.source === "modrinth" ? "Modrinth" : "CurseForge"}
										</span>

										<span className="rounded bg-muted px-1.5 py-0.5 text-2xs text-muted-foreground capitalize">
											{contentType(t, item.projectType)}
										</span>

										<span className="text-muted-foreground text-xs">
											{t("modpacks.byAuthor", { author: item.author })}
										</span>
									</div>

									<h2 className="truncate font-bold text-foreground text-xl tracking-tight">
										{item.title}
									</h2>

									<div className="flex items-center gap-3 text-muted-foreground text-xs">
										<div className="flex items-center gap-1">
											<Download className="size-3.5" />
											<span>
												{t("contentDetails.downloads", {
													count: item.downloads,
													formatted: formatDownloads(item.downloads),
												})}
											</span>
										</div>
										{details?.follows != null && (
											<div>
												{t("contentDetails.followers", {
													count: details.follows,
													formatted: formatNumber(details.follows),
												})}
											</div>
										)}
									</div>
								</div>
							</div>

							<div className="flex shrink-0 items-center gap-2">
								<Button
									size="sm"
									onClick={() => onInstall(item)}
									className="gap-1.5 font-semibold text-xs"
								>
									<Download className="size-3.5" />
									{item.projectType === "modpack"
										? t("install.modpackTitle")
										: t("contentDetails.addToInstance")}
								</Button>

								{item.websiteUrl && (
									<Button
										size="sm"
										variant="outline"
										onClick={() => handleOpenUrl(item.websiteUrl)}
										className="gap-1.5 text-xs"
									>
										<Globe className="size-3.5" />
										{t("pluginDetails.website")}
									</Button>
								)}

								<Button
									variant="ghost"
									size="icon"
									onClick={() => onOpenChange(false)}
									aria-label={t("common.close")}
								>
									<X />
								</Button>
							</div>
						</div>

						{/* Navigation Tabs */}
						<Tabs
							value={activeTab}
							onValueChange={(tab) => setActiveTab(tab as typeof activeTab)}
							className="border-border/40 border-b px-5 pt-2"
						>
							<TabsList variant="line">
								<TabsTrigger value="overview">{t("serverTabs.overview")}</TabsTrigger>
								<TabsTrigger value="versions">
									{details
										? t("contentDetails.versionsCount", { count: details.versions.length })
										: t("contentDetails.versions")}
								</TabsTrigger>
							</TabsList>
						</Tabs>

						{/* Tab Content */}
						<div className="flex min-h-0 flex-1 flex-col bg-background/50">
							{isLoading ? (
								<div className="flex flex-1 items-center justify-center gap-2 text-muted-foreground text-sm">
									<Spinner className="size-5 text-primary" />
									<span>{t("contentDetails.loading")}</span>
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
										{t("common.tryAgain")}
									</Button>
								</div>
							) : activeTab === "overview" ? (
								<ScrollArea className="flex-1">
									<div className="flex flex-col gap-6 p-6">
										{/* Gallery: Featured Large Preview + Scrollable Thumbnail Strip */}
										{details?.screenshots && details.screenshots.length > 0 && (
											<div className="flex w-full min-w-0 max-w-full flex-col gap-3">
												<div className="flex items-center justify-between">
													<h3 className="font-semibold text-foreground text-sm">
														{t("contentDetails.gallery", { count: details.screenshots.length })}
													</h3>
													<span className="text-2xs text-muted-foreground">
														{t("contentDetails.clickToOpen")}
													</span>
												</div>

												{/* Featured Main Image Preview */}
												<div className="group relative aspect-video w-full overflow-hidden rounded-xl border border-border/40 bg-background shadow-md">
													<img
														src={
															details.screenshots[
																activePhotoIndex < details.screenshots.length ? activePhotoIndex : 0
															].url
														}
														alt={
															details.screenshots[
																activePhotoIndex < details.screenshots.length ? activePhotoIndex : 0
															].title || ""
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
														aria-label={t("contentDetails.viewFullscreen")}
													>
														<div className="flex items-center gap-2 rounded-lg bg-black/80 px-4 py-2 font-medium text-white text-xs shadow-xl backdrop-blur-xs transition-transform duration-200 hover:scale-105">
															<Maximize2 className="size-4" />
															<span>{t("contentDetails.viewFullscreen")}</span>
														</div>
													</button>

													{/* Bottom Caption Overlay */}
													<div className="absolute inset-x-0 bottom-0 flex items-center justify-between bg-linear-to-t from-black/80 via-black/40 to-transparent p-3 text-white">
														<span className="truncate font-medium text-xs">
															{details.screenshots[
																activePhotoIndex < details.screenshots.length ? activePhotoIndex : 0
															].title || item.title}
														</span>
														<span className="dark rounded bg-black/60 px-2 py-0.5 font-mono text-2xs text-foreground/80">
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
															className="size-8 shrink-0 rounded-lg"
															aria-label={t("contentDetails.scrollLeft")}
														>
															<ChevronLeft className="size-4" />
														</Button>

														<div
															ref={galleryScrollRef}
															className="scrollbar-none flex min-w-0 flex-1 gap-2.5 overflow-x-auto overflow-y-hidden overscroll-contain py-1 [-ms-overflow-style:none] [&::-webkit-scrollbar]:hidden"
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
																			: "border-transparent opacity-60 hover:border-border/20 hover:opacity-90"
																	}`}
																>
																	<img
																		src={s.url}
																		alt={s.title || ""}
																		className="size-full object-cover"
																	/>
																	{s.title && (
																		<div className="absolute inset-x-0 bottom-0 truncate bg-linear-to-t from-black/80 to-transparent p-1 text-left text-3xs text-white">
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
															className="size-8 shrink-0 rounded-lg"
															aria-label={t("contentDetails.scrollRight")}
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
														className="h-7 gap-1.5 text-xs"
													>
														<ExternalLink className="size-3" />
														{t("contentDetails.source")}
													</Button>
												)}
												{details.issuesUrl && (
													<Button
														size="sm"
														variant="outline"
														onClick={() => handleOpenUrl(details.issuesUrl)}
														className="h-7 gap-1.5 text-xs"
													>
														<ExternalLink className="size-3" />
														{t("contentDetails.issues")}
													</Button>
												)}
												{details.wikiUrl && (
													<Button
														size="sm"
														variant="outline"
														onClick={() => handleOpenUrl(details.wikiUrl)}
														className="h-7 gap-1.5 text-xs"
													>
														<ExternalLink className="size-3" />
														{t("contentDetails.wiki")}
													</Button>
												)}
											</div>
										)}

										{/* Rich Description (Markdown & HTML Supported) */}
										<div className="flex flex-col gap-2">
											<h3 className="font-semibold text-foreground text-sm">
												{t("contentDetails.description")}
											</h3>
											{details?.body ? (
												<section
													className="prose prose-invert prose-sm max-w-none space-y-3 text-muted-foreground text-xs leading-relaxed [&_a]:text-primary [&_a]:underline hover:[&_a]:text-primary/80 [&_blockquote]:my-3 [&_blockquote]:rounded-r-lg [&_blockquote]:border-primary/70 [&_blockquote]:border-l-4 [&_blockquote]:bg-primary/5 [&_blockquote]:py-1 [&_blockquote]:pl-4 [&_code]:rounded [&_code]:bg-muted/80 [&_code]:px-1.5 [&_code]:py-0.5 [&_code]:font-mono [&_code]:text-2xs [&_code]:text-primary-foreground [&_details]:my-3 [&_details]:rounded-lg [&_details]:border [&_details]:border-border/40 [&_details]:bg-card/50 [&_details]:p-3 [&_h1]:mt-6 [&_h1]:mb-3 [&_h1]:font-bold [&_h1]:text-foreground [&_h1]:text-lg [&_h2]:mt-5 [&_h2]:mb-2 [&_h2]:font-bold [&_h2]:text-base [&_h2]:text-foreground [&_h3]:mt-4 [&_h3]:mb-1 [&_h3]:font-semibold [&_h3]:text-foreground [&_h3]:text-sm [&_img]:my-3 [&_img]:max-w-full [&_img]:rounded-lg [&_img]:border [&_img]:border-border/40 [&_li]:my-1 [&_p]:my-2 [&_pre]:my-3 [&_pre]:overflow-x-auto [&_pre]:rounded-lg [&_pre]:bg-card/90 [&_pre]:p-4 [&_summary]:cursor-pointer [&_summary]:font-semibold [&_summary]:text-foreground hover:[&_summary]:text-primary [&_table]:my-4 [&_table]:w-full [&_table]:border-collapse [&_td]:border [&_td]:border-border/60 [&_td]:p-2.5 [&_th]:border [&_th]:border-border [&_th]:bg-muted/80 [&_th]:p-2.5 [&_th]:text-left [&_th]:font-semibold"
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
													aria-label={t("contentDetails.description")}
													// biome-ignore lint/security/noDangerouslySetInnerHtml: sanitized in renderDescriptionHtml
													dangerouslySetInnerHTML={{ __html: descriptionHtml }}
												/>
											) : (
												<p className="text-muted-foreground text-xs">
													{details?.description || t("modpacks.noDescription")}
												</p>
											)}
										</div>
									</div>
								</ScrollArea>
							) : (
								/* Versions & Files Tab */
								<div className="flex min-h-0 flex-1 flex-col">
									{/* Modern Version filters with Select components */}
									<div className="flex flex-wrap items-center gap-3 border-border/40 border-b bg-background/20 px-6 py-3">
										<div className="flex items-center gap-2">
											<span className="text-muted-foreground text-xs">
												{t("modpacks.filters.loader")}
											</span>
											<Select
												items={loaderFilterItems}
												value={versionLoaderFilter || "all"}
												onValueChange={(val) =>
													setVersionLoaderFilter(!val || val === "all" ? "" : val)
												}
											>
												<SelectTrigger className="h-8 w-32">
													<SelectValue />
												</SelectTrigger>
												<SelectContent>
													{loaderFilterItems.map((item) => (
														<SelectItem key={item.value} value={item.value}>
															{item.label}
														</SelectItem>
													))}
												</SelectContent>
											</Select>
										</div>

										<div className="flex items-center gap-2">
											<span className="text-muted-foreground text-xs">
												{t("contentDetails.gameVersion")}
											</span>
											<Select
												items={gameVersionFilterItems}
												value={versionGameVerFilter || "all"}
												onValueChange={(val) =>
													setVersionGameVerFilter(!val || val === "all" ? "" : val)
												}
											>
												<SelectTrigger className="h-8 w-36">
													<SelectValue />
												</SelectTrigger>
												<SelectContent>
													{gameVersionFilterItems.map((item) => (
														<SelectItem key={item.value} value={item.value}>
															{item.label}
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
												className="h-8 gap-1 text-muted-foreground text-xs"
											>
												<X className="size-3" />
												{t("console.clear")}
											</Button>
										)}

										<span className="ml-auto text-muted-foreground text-xs">
											{t("contentDetails.showing", {
												shown: filteredVersions.length,
												count: details?.versions.length ?? 0,
											})}
										</span>
									</div>

									<ScrollArea className="flex-1">
										<div className="flex flex-col gap-3 p-6">
											{filteredVersions.length === 0 ? (
												<div className="py-12 text-center text-muted-foreground text-xs">
													{t("contentDetails.noMatch")}
												</div>
											) : (
												filteredVersions.map((ver) => (
													<div
														key={ver.id}
														className="flex flex-col gap-2 rounded-xl border border-border/40 bg-card/40 p-4 transition-all hover:border-border hover:bg-card/60"
													>
														<div className="flex items-start justify-between gap-3">
															<div className="flex min-w-0 flex-1 flex-col gap-1">
																<div className="flex items-center gap-2">
																	<Badge
																		variant="outline"
																		className={cn(
																			"uppercase",
																			ver.versionType === "release"
																				? "border-primary/30 text-primary"
																				: ver.versionType === "beta"
																					? "border-warning/30 text-warning"
																					: "border-destructive/30 text-destructive",
																		)}
																	>
																		{releaseType(t, ver.versionType)}
																	</Badge>
																	<h4 className="truncate font-semibold text-foreground text-sm">
																		{ver.name}
																	</h4>
																</div>

																<div className="mt-1 flex flex-wrap items-center gap-1.5 text-muted-foreground text-xs">
																	<span className="font-mono text-muted-foreground">
																		{ver.filename}
																	</span>
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
																{t("common.install")}
															</Button>
														</div>

														<div className="mt-1 flex flex-wrap items-center gap-1.5 border-border/20 border-t pt-2">
															{ver.loaders.map((l) => (
																<Badge key={l} variant="secondary" className="font-mono capitalize">
																	{l}
																</Badge>
															))}
															{ver.gameVersions.map((gv) => (
																<Badge key={gv} variant="outline" className="font-mono">
																	{gv}
																</Badge>
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
						<div className="dark fade-in-0 fixed inset-x-0 top-10 bottom-0 z-60 flex animate-in flex-col justify-between bg-black/95 p-4 backdrop-blur-md duration-200">
							{/* Top Bar */}
							<div className="flex items-center justify-between text-foreground">
								<div className="flex items-center gap-3">
									<span className="rounded-md bg-foreground/10 px-2.5 py-1 font-medium text-xs">
										{lightboxIndex + 1} / {details.screenshots.length}
									</span>
									<span className="truncate font-medium text-foreground/80 text-sm">
										{details.screenshots[lightboxIndex].title || item.title}
									</span>
								</div>
								<div className="flex items-center gap-2">
									<Button
										size="sm"
										variant="ghost"
										onClick={() => handleOpenUrl(details.screenshots[lightboxIndex].url)}
										className="h-8 gap-1.5 text-foreground/80 text-xs"
									>
										<ExternalLink className="size-3.5" />
										{t("contentDetails.openOriginal")}
									</Button>
									<Button
										variant="ghost"
										size="icon"
										onClick={() => setLightboxIndex(null)}
										aria-label={t("screenshots.lightbox.close")}
									>
										<X />
									</Button>
								</div>
							</div>

							{/* Center Image with Previous/Next Controls */}
							<div className="relative flex flex-1 items-center justify-center py-4">
								{lightboxIndex > 0 && (
									<button
										type="button"
										onClick={() => setLightboxIndex(lightboxIndex - 1)}
										className="absolute top-1/2 left-4 -translate-y-1/2 rounded-full bg-black/60 p-3 text-white shadow-lg backdrop-blur-xs transition-all hover:bg-black/90"
										aria-label={t("contentDetails.previous")}
									>
										<ChevronLeft className="size-6" />
									</button>
								)}

								<img
									src={details.screenshots[lightboxIndex].url}
									alt={details.screenshots[lightboxIndex].title || ""}
									className="max-h-[72vh] max-w-[85vw] rounded-lg object-contain shadow-2xl transition-all"
								/>

								{lightboxIndex < details.screenshots.length - 1 && (
									<button
										type="button"
										onClick={() => setLightboxIndex(lightboxIndex + 1)}
										className="absolute top-1/2 right-4 -translate-y-1/2 rounded-full bg-black/60 p-3 text-white shadow-lg backdrop-blur-xs transition-all hover:bg-black/90"
										aria-label={t("contentDetails.next")}
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
												: "border-transparent opacity-50 hover:border-border/20 hover:opacity-80"
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
