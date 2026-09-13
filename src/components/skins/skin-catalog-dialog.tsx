import {
	AlertCircle,
	Check,
	ChevronLeft,
	ChevronRight,
	Download,
	Eye,
	Heart,
	KeyRound,
	Loader2,
	Lock,
	RefreshCw,
	Search,
	Sparkles,
	Upload,
	Users,
	X,
} from "lucide-react"
import { useCallback, useEffect, useRef, useState } from "react"
import SkinViewer3D, { DEFAULT_STEVE_SKIN } from "@/components/accounts/skin-viewer-3d"
import { Button } from "@/components/ui/button"
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog"
import { Input } from "@/components/ui/input"
import { ScrollArea } from "@/components/ui/scroll-area"
import SkinAvatar from "@/components/ui/skin-avatar"
import { accountService, useAccounts } from "@/services/account-service"
import type { AccountProfile } from "@/types/account"
import type { ElySkinItem, SkinModelFilter, SkinSortOption } from "@/types/skin"

export interface SkinCatalogDialogProps {
	open: boolean
	onOpenChange: (open: boolean) => void
	initialAccount?: AccountProfile | null
}

function formatNumber(num?: number | null): string {
	if (num === undefined || num === null || Number.isNaN(num)) return "0"
	if (num >= 1_000_000) return `${(num / 1_000_000).toFixed(1)}M`
	if (num >= 1_000) return `${(num / 1_000).toFixed(1)}k`
	return String(num)
}

function normalizeSkinItem(raw: ElySkinItem | Record<string, unknown>): ElySkinItem {
	const r = raw as Record<string, unknown>
	return {
		id: Number(r.id),
		skinUrl: (r.skinUrl as string) || (r.skin_url as string) || "",
		isSlim: Boolean(r.isSlim ?? r.is_slim),
		countWearers: Number(r.countWearers ?? r.count_wearers ?? 0),
		countCubes: Number(r.countCubes ?? r.count_cubes ?? 0),
		countViews: Number(r.countViews ?? r.count_views ?? 0),
		tags: Array.isArray(r.tags) ? (r.tags as string[]) : [],
	}
}

export function SkinCatalogDialog({ open, onOpenChange, initialAccount }: SkinCatalogDialogProps) {
	const { accounts, activeAccount } = useAccounts()
	const targetAccount =
		initialAccount || activeAccount || accounts.find((a) => a.accountType === "ely") || null

	// Active tab: "catalog" | "upload"
	const [activeTab, setActiveTab] = useState<"catalog" | "upload">("catalog")

	// Catalog state
	const [skins, setSkins] = useState<ElySkinItem[]>([])
	const [selectedSkin, setSelectedSkin] = useState<ElySkinItem | null>(null)
	const [page, setPage] = useState(1)
	const [lastPage, setLastPage] = useState(1)
	const [totalItems, setTotalItems] = useState(0)
	const [searchQuery, setSearchQuery] = useState("")
	const [debouncedQuery, setDebouncedQuery] = useState("")
	const [sortOption, setSortOption] = useState<SkinSortOption>("wearers")
	const [modelFilter, setModelFilter] = useState<SkinModelFilter>("any")
	const [isLoadingCatalog, setIsLoadingCatalog] = useState(false)
	const [catalogError, setCatalogError] = useState<string | null>(null)

	// Action state
	const [isApplying, setIsApplying] = useState(false)
	const [applyStatusMessage, setApplyStatusMessage] = useState<{
		type: "success" | "error"
		text: string
	} | null>(null)

	// Password prompt modal state
	const [showPasswordDialog, setShowPasswordDialog] = useState(false)
	const [passwordInput, setPasswordInput] = useState("")
	const [passwordError, setPasswordError] = useState<string | null>(null)
	const [pendingAction, setPendingAction] = useState<
		{ type: "apply"; skinId: number } | { type: "upload"; dataUrl: string } | null
	>(null)

	// Upload state
	const [uploadedDataUrl, setUploadedDataUrl] = useState<string | null>(null)
	const [uploadedFileName, setUploadedFileName] = useState<string | null>(null)
	const [uploadedModel, setUploadedModel] = useState<"default" | "slim">("default")
	const [uploadValidationDetails, setUploadValidationDetails] = useState<{
		dimensions: string
		sizeKb: string
	} | null>(null)
	const [uploadError, setUploadError] = useState<string | null>(null)
	const [isDragging, setIsDragging] = useState(false)
	const fileInputRef = useRef<HTMLInputElement | null>(null)

	// Debounce search query
	useEffect(() => {
		const timer = setTimeout(() => {
			setDebouncedQuery(searchQuery.trim())
			setPage(1)
		}, 400)
		return () => clearTimeout(timer)
	}, [searchQuery])

	// Fetch catalog on filter / page changes
	const fetchCatalog = useCallback(async () => {
		if (!open) return
		setIsLoadingCatalog(true)
		setCatalogError(null)

		try {
			const res = await accountService.getElySkins(
				page,
				debouncedQuery || undefined,
				sortOption,
				modelFilter === "any" ? undefined : modelFilter,
			)
			const normalized = (res.items || []).map(normalizeSkinItem)
			if (sortOption === "views") {
				normalized.sort((a, b) => b.countViews - a.countViews)
			} else if (sortOption === "cubes") {
				normalized.sort((a, b) => b.countCubes - a.countCubes)
			}
			setSkins(normalized)
			setLastPage(res.lastPage || 1)
			setTotalItems(res.totalItems || 0)
			setSelectedSkin((prev) => {
				if (prev && normalized.some((item) => item.id === prev.id)) {
					return prev
				}
				return normalized[0] ?? null
			})
		} catch (err: unknown) {
			console.error("Failed to fetch Ely.by skins:", err)
			setCatalogError(err instanceof Error ? err.message : "Failed to load skins from Ely.by")
		} finally {
			setIsLoadingCatalog(false)
		}
	}, [open, page, debouncedQuery, sortOption, modelFilter])

	useEffect(() => {
		if (open && activeTab === "catalog") {
			fetchCatalog()
		}
	}, [open, activeTab, fetchCatalog])

	// Reset status message after delay
	useEffect(() => {
		if (!applyStatusMessage) return
		const timer = setTimeout(() => {
			setApplyStatusMessage(null)
		}, 4000)
		return () => clearTimeout(timer)
	}, [applyStatusMessage])

	// Apply skin to account handler
	const handleApplySkin = async (skinId: number, password?: string) => {
		if (!targetAccount) return

		if (targetAccount.accountType !== "ely") {
			setApplyStatusMessage({
				type: "error",
				text: "Skin synchronization requires an active Ely.by account.",
			})
			return
		}

		setIsApplying(true)
		setApplyStatusMessage(null)

		try {
			await accountService.applyElySkin(targetAccount.id, skinId, password)
			setApplyStatusMessage({
				type: "success",
				text: "Skin applied successfully to your Ely.by account!",
			})
			setShowPasswordDialog(false)
			setPasswordInput("")
			setPendingAction(null)
		} catch (err: unknown) {
			const errMsg = err instanceof Error ? err.message : String(err)
			if (errMsg.includes("PASSWORD_REQUIRED")) {
				setPendingAction({ type: "apply", skinId })
				setShowPasswordDialog(true)
			} else {
				setApplyStatusMessage({
					type: "error",
					text: errMsg || "Failed to apply skin",
				})
			}
		} finally {
			setIsApplying(false)
		}
	}

	// Upload skin to account handler
	const handleUploadSkin = async (dataUrl: string, password?: string) => {
		if (!targetAccount) return

		if (targetAccount.accountType !== "ely") {
			setApplyStatusMessage({
				type: "error",
				text: "Uploading requires an active Ely.by account.",
			})
			return
		}

		setIsApplying(true)
		setApplyStatusMessage(null)

		try {
			await accountService.uploadElySkin(targetAccount.id, dataUrl, password)
			setApplyStatusMessage({
				type: "success",
				text: "Custom skin uploaded and applied to your account!",
			})
			setShowPasswordDialog(false)
			setPasswordInput("")
			setPendingAction(null)
		} catch (err: unknown) {
			const errMsg = err instanceof Error ? err.message : String(err)
			if (errMsg.includes("PASSWORD_REQUIRED")) {
				setPendingAction({ type: "upload", dataUrl })
				setShowPasswordDialog(true)
			} else {
				setApplyStatusMessage({
					type: "error",
					text: errMsg || "Failed to upload skin",
				})
			}
		} finally {
			setIsApplying(false)
		}
	}

	// Handle password submission
	const handleConfirmPassword = async () => {
		if (!passwordInput.trim() || !pendingAction) {
			setPasswordError("Please enter your Ely.by account password")
			return
		}
		setPasswordError(null)

		if (pendingAction.type === "apply") {
			await handleApplySkin(pendingAction.skinId, passwordInput.trim())
		} else if (pendingAction.type === "upload") {
			await handleUploadSkin(pendingAction.dataUrl, passwordInput.trim())
		}
	}

	// Validate and read dropped or selected file
	const processFile = (file: File) => {
		setUploadError(null)

		if (!file.type.includes("png") && !file.name.toLowerCase().endsWith(".png")) {
			setUploadError("Invalid file type: Minecraft skins must be in PNG format (.png).")
			return
		}

		if (file.size > 196608) {
			setUploadError("File too large: Ely.by skins cannot exceed 192 KB.")
			return
		}

		const reader = new FileReader()
		reader.onload = (e) => {
			const result = e.target?.result as string
			if (!result) return

			// Validate image dimensions using Image element
			const img = new Image()
			img.onload = () => {
				const is64x64 = img.width === 64 && img.height === 64
				const is64x32 = img.width === 64 && img.height === 32
				const is128x128 = img.width === 128 && img.height === 128

				if (!is64x64 && !is64x32 && !is128x128) {
					setUploadError(`Invalid dimensions: ${img.width}x${img.height}. Expected 64x64 or 64x32.`)
					return
				}

				setUploadedDataUrl(result)
				setUploadedFileName(file.name)
				setUploadValidationDetails({
					dimensions: `${img.width}x${img.height}`,
					sizeKb: `${(file.size / 1024).toFixed(1)} KB`,
				})
			}
			img.onerror = () => {
				setUploadError("Could not decode image file.")
			}
			img.src = result
		}
		reader.readAsDataURL(file)
	}

	// Download skin locally
	const handleDownloadSkin = async (skinUrl: string, name: string) => {
		try {
			await accountService.saveSkinToDownloads(name, skinUrl)
			setApplyStatusMessage({
				type: "success",
				text: "Skin saved to your Downloads folder!",
			})
		} catch {
			try {
				const link = document.createElement("a")
				link.href = skinUrl
				link.download = `${name}-skin.png`
				link.target = "_blank"
				document.body.appendChild(link)
				link.click()
				document.body.removeChild(link)
			} catch (err) {
				console.error("Failed to download skin:", err)
			}
		}
	}

	return (
		<>
			<Dialog open={open} onOpenChange={onOpenChange}>
				<DialogContent className="flex h-[640px] max-h-[90vh] w-full flex-col gap-0 overflow-hidden border-zinc-800 bg-zinc-950 p-0 sm:max-w-4xl">
					{/* Header with Title and Tabs */}
					<DialogHeader className="border-zinc-800/80 border-b bg-zinc-900/40 p-4 pb-3 sm:p-5">
						<div className="flex flex-col justify-between gap-3 sm:flex-row sm:items-center">
							<div className="flex items-center gap-3">
								<div className="flex size-9 items-center justify-center rounded-md border border-emerald-500/20 bg-emerald-500/10 text-emerald-400">
									<Sparkles className="size-4" />
								</div>
								<div>
									<DialogTitle className="flex items-center gap-2 font-semibold text-base text-zinc-100">
										Ely.by Skin Manager
										{targetAccount && (
											<span className="font-normal text-xs text-zinc-400">
												(Account:{" "}
												<span className="font-medium text-zinc-200">{targetAccount.username}</span>)
											</span>
										)}
									</DialogTitle>
									<DialogDescription className="text-xs text-zinc-400">
										Browse community skins from Ely.by, upload your own, or apply directly to your
										account.
									</DialogDescription>
								</div>
							</div>

							{/* Tab selector */}
							<div className="flex items-center gap-1 self-start rounded-lg border border-zinc-800 bg-zinc-900 p-0.5 sm:self-auto">
								<Button
									type="button"
									variant="ghost"
									size="sm"
									className={`h-7 rounded-md px-3 font-medium text-xs transition-colors ${
										activeTab === "catalog"
											? "bg-zinc-800 text-zinc-100 shadow-xs"
											: "text-zinc-400 hover:text-zinc-200"
									}`}
									onClick={() => setActiveTab("catalog")}
								>
									<Search className="mr-1.5 size-3.5" />
									Browse Catalog
								</Button>
								<Button
									type="button"
									variant="ghost"
									size="sm"
									className={`h-7 rounded-md px-3 font-medium text-xs transition-colors ${
										activeTab === "upload"
											? "bg-zinc-800 text-zinc-100 shadow-xs"
											: "text-zinc-400 hover:text-zinc-200"
									}`}
									onClick={() => setActiveTab("upload")}
								>
									<Upload className="mr-1.5 size-3.5" />
									Upload Skin
								</Button>
							</div>
						</div>
					</DialogHeader>

					{/* Notification banner */}
					{applyStatusMessage && (
						<div
							className={`flex items-center gap-2 border-b px-4 py-2 text-xs ${
								applyStatusMessage.type === "success"
									? "border-emerald-500/20 bg-emerald-500/10 text-emerald-300"
									: "border-rose-500/20 bg-rose-500/10 text-rose-300"
							}`}
						>
							{applyStatusMessage.type === "success" ? (
								<Check className="size-3.5 shrink-0" />
							) : (
								<AlertCircle className="size-3.5 shrink-0" />
							)}
							<span>{applyStatusMessage.text}</span>
						</div>
					)}

					{/* Tab 1: Catalog */}
					{activeTab === "catalog" && (
						<div className="flex min-h-0 flex-1 flex-col">
							{/* Filter Toolbar */}
							<div className="flex flex-wrap items-center gap-2.5 border-zinc-800/60 border-b bg-zinc-900/20 p-3 sm:px-5">
								{/* Search Bar */}
								<div className="relative min-w-[180px] flex-1">
									<Search className="absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-zinc-500" />
									<Input
										type="text"
										placeholder="Search skins by tag or keyword (e.g. knight, anime)..."
										value={searchQuery}
										onChange={(e) => setSearchQuery(e.target.value)}
										className="h-8 border-zinc-800 bg-zinc-900/80 pr-7 pl-8 text-xs text-zinc-200 placeholder:text-zinc-500"
									/>
									{searchQuery && (
										<button
											type="button"
											onClick={() => setSearchQuery("")}
											className="absolute top-1/2 right-2 -translate-y-1/2 text-zinc-500 hover:text-zinc-300"
										>
											<X className="size-3" />
										</button>
									)}
								</div>

								{/* Model Filter */}
								<div className="flex items-center gap-1 rounded-md border border-zinc-800 bg-zinc-900 p-0.5">
									{(["any", "steve", "slim"] as const).map((m) => (
										<button
											key={m}
											type="button"
											className={`rounded px-2 py-1 font-medium text-[11px] transition-colors ${
												modelFilter === m
													? "bg-zinc-800 text-zinc-100"
													: "text-zinc-400 hover:text-zinc-200"
											}`}
											onClick={() => {
												setModelFilter(m)
												setPage(1)
											}}
										>
											{m === "any" ? "All Models" : m === "steve" ? "Classic" : "Slim"}
										</button>
									))}
								</div>

								{/* Sort Filter */}
								<div className="flex items-center gap-1 rounded-md border border-zinc-800 bg-zinc-900 p-0.5">
									{(
										[
											{ id: "wearers", label: "Popular" },
											{ id: "views", label: "Views" },
											{ id: "cubes", label: "Likes" },
											{ id: "latest", label: "Latest" },
										] as const
									).map((s) => (
										<button
											key={s.id}
											type="button"
											className={`rounded px-2 py-1 font-medium text-[11px] transition-colors ${
												sortOption === s.id
													? "bg-zinc-800 text-zinc-100"
													: "text-zinc-400 hover:text-zinc-200"
											}`}
											onClick={() => {
												setSortOption(s.id)
												setPage(1)
											}}
										>
											{s.label}
										</button>
									))}
								</div>
							</div>

							{/* Main Content: Left Grid + Right 3D Preview */}
							<div className="flex min-h-0 flex-1 overflow-hidden">
								{/* Left Grid */}
								<div className="relative flex min-w-0 flex-1 flex-col border-zinc-800/60 border-r">
									{isLoadingCatalog && skins.length > 0 && (
										<div className="absolute top-2 right-3 z-10 flex items-center gap-1.5 rounded-full border border-zinc-700/80 bg-zinc-900/95 px-2.5 py-1 text-[11px] text-zinc-300 shadow-md backdrop-blur-xs">
											<Loader2 className="size-3 animate-spin text-emerald-400" />
											<span>Updating...</span>
										</div>
									)}
									<ScrollArea className="flex-1 p-3 sm:p-4">
										{isLoadingCatalog && skins.length === 0 ? (
											<div className="flex h-full min-h-[400px] flex-col items-center justify-center gap-2 text-zinc-400">
												<Loader2 className="size-6 animate-spin text-emerald-400" />
												<span className="text-xs">Loading skins from Ely.by...</span>
											</div>
										) : catalogError ? (
											<div className="flex h-full min-h-[400px] flex-col items-center justify-center gap-3 p-4 text-center">
												<AlertCircle className="size-8 text-rose-400" />
												<p className="max-w-sm text-rose-300 text-xs">{catalogError}</p>
												<Button
													size="sm"
													variant="outline"
													onClick={fetchCatalog}
													className="h-7 text-xs"
												>
													<RefreshCw className="mr-1.5 size-3" />
													Retry
												</Button>
											</div>
										) : skins.length === 0 ? (
											<div className="flex h-full min-h-[400px] flex-col items-center justify-center gap-2 text-center text-zinc-500">
												<Search className="size-8 opacity-40" />
												<p className="font-medium text-xs text-zinc-400">No skins found</p>
												<p className="text-[11px] text-zinc-500">
													Try a different search query or filter.
												</p>
											</div>
										) : (
											<div
												className={`grid grid-cols-2 gap-2.5 transition-opacity duration-150 sm:grid-cols-3 md:grid-cols-4 ${
													isLoadingCatalog ? "pointer-events-none opacity-50" : "opacity-100"
												}`}
											>
												{skins.map((skin) => {
													const isSelected = selectedSkin?.id === skin.id
													return (
														<button
															key={skin.id}
															type="button"
															onClick={() => setSelectedSkin(skin)}
															className={`group relative flex flex-col rounded-lg border p-2 text-left transition-all ${
																isSelected
																	? "border-emerald-500/50 bg-emerald-500/10 shadow-emerald-500/10 shadow-sm ring-1 ring-emerald-500/30"
																	: "border-zinc-800/80 bg-zinc-900/60 hover:border-zinc-700 hover:bg-zinc-900"
															}`}
														>
															{/* Avatar / Thumbnail */}
															<div className="relative mb-2 flex aspect-square items-center justify-center overflow-hidden rounded-md border border-zinc-800/60 bg-zinc-950/80">
																<SkinAvatar
																	username={`skin_${skin.id}`}
																	skinUrl={skin.skinUrl}
																	size={64}
																	className="transition-transform group-hover:scale-105"
																/>
																{skin.isSlim && (
																	<span className="absolute right-1 bottom-1 rounded-xs border border-zinc-700 bg-zinc-900/90 px-1 py-0.2 font-mono text-[9px] text-zinc-300">
																		Slim
																	</span>
																)}
															</div>

															{/* Badges / Stats */}
															<div className="flex items-center justify-between gap-1 text-[10px] text-zinc-400">
																<span className="flex items-center gap-1">
																	<Users className="size-2.5 text-zinc-500" />
																	{formatNumber(skin.countWearers)}
																</span>
																<span className="flex items-center gap-1 text-zinc-500">
																	<Heart className="size-2.5 text-rose-500/70" />
																	{formatNumber(skin.countCubes)}
																</span>
															</div>

															{/* Tags */}
															{skin.tags.length > 0 && (
																<div className="mt-1.5 flex h-4 flex-wrap gap-1 overflow-hidden">
																	{skin.tags.slice(0, 2).map((t) => (
																		<span
																			key={t}
																			className="max-w-[70px] truncate rounded bg-zinc-800 px-1 py-0.1 text-[9px] text-zinc-400"
																		>
																			{t}
																		</span>
																	))}
																</div>
															)}
														</button>
													)
												})}
											</div>
										)}
									</ScrollArea>

									{/* Pagination Footer */}
									<div className="flex items-center justify-between border-zinc-800/60 border-t bg-zinc-900/40 p-2.5 px-4 text-xs text-zinc-400">
										<span className="text-[11px] text-zinc-500">
											{totalItems > 0
												? `Showing page ${page} of ${lastPage} (${totalItems} skins)`
												: `Page ${page} of ${lastPage}`}
										</span>
										<div className="flex items-center gap-1.5">
											<Button
												size="sm"
												variant="ghost"
												disabled={page <= 1 || isLoadingCatalog}
												onClick={() => setPage((p) => Math.max(1, p - 1))}
												className="h-7 px-2 text-xs"
											>
												<ChevronLeft className="mr-0.5 size-3.5" />
												Previous
											</Button>
											<Button
												size="sm"
												variant="ghost"
												disabled={page >= lastPage || isLoadingCatalog}
												onClick={() => setPage((p) => Math.min(lastPage, p + 1))}
												className="h-7 px-2 text-xs"
											>
												Next
												<ChevronRight className="ml-0.5 size-3.5" />
											</Button>
										</div>
									</div>
								</div>

								{/* Right Side: 3D Preview Panel */}
								<div className="flex w-[280px] shrink-0 flex-col gap-3 overflow-y-auto bg-zinc-950/60 p-3 sm:w-[320px] sm:p-4">
									{selectedSkin ? (
										<>
											<div className="flex items-center justify-between font-semibold text-xs text-zinc-300">
												<span>Skin Preview</span>
												<span className="font-normal text-[11px] text-zinc-500">
													#{selectedSkin.id}
												</span>
											</div>

											{/* Interactive 3D Viewer */}
											<div className="relative flex items-center justify-center overflow-hidden rounded-xl border border-zinc-800 bg-zinc-900/60 shadow-inner">
												<SkinViewer3D
													skinUrl={selectedSkin.skinUrl}
													username={`skin_${selectedSkin.id}`}
													width={260}
													height={280}
													model={selectedSkin.isSlim ? "slim" : "default"}
													className="rounded-lg"
												/>
											</div>

											{/* Stats Row */}
											<div className="grid grid-cols-3 gap-1.5 text-center">
												<div className="rounded-lg border border-zinc-800/80 bg-zinc-900/60 p-1.5">
													<div className="text-[10px] text-zinc-500">Wearers</div>
													<div className="font-semibold text-xs text-zinc-200">
														{formatNumber(selectedSkin.countWearers)}
													</div>
												</div>
												<div className="rounded-lg border border-zinc-800/80 bg-zinc-900/60 p-1.5">
													<div className="text-[10px] text-zinc-500">Views</div>
													<div className="font-semibold text-xs text-zinc-200">
														{formatNumber(selectedSkin.countViews)}
													</div>
												</div>
												<div className="rounded-lg border border-zinc-800/80 bg-zinc-900/60 p-1.5">
													<div className="text-[10px] text-zinc-500">Likes</div>
													<div className="font-semibold text-rose-400 text-xs">
														{formatNumber(selectedSkin.countCubes)}
													</div>
												</div>
											</div>

											{/* Tags */}
											{selectedSkin.tags.length > 0 && (
												<div className="flex flex-wrap gap-1">
													{selectedSkin.tags.map((t) => (
														<button
															key={t}
															type="button"
															onClick={() => setSearchQuery(t)}
															className="rounded-md border border-zinc-800 bg-zinc-900 px-2 py-0.5 text-[10px] text-zinc-400 transition-colors hover:border-zinc-700 hover:text-zinc-200"
														>
															#{t}
														</button>
													))}
												</div>
											)}

											{/* Action Buttons */}
											<div className="mt-auto flex flex-col gap-2 pt-2">
												<Button
													type="button"
													disabled={
														isApplying || !targetAccount || targetAccount.accountType !== "ely"
													}
													onClick={() => handleApplySkin(selectedSkin.id)}
													className="h-8 w-full bg-emerald-600 font-semibold text-white text-xs hover:bg-emerald-500"
												>
													{isApplying ? (
														<>
															<Loader2 className="mr-1.5 size-3.5 animate-spin" />
															Applying...
														</>
													) : targetAccount?.accountType === "ely" ? (
														<>
															<Check className="mr-1.5 size-3.5" />
															Apply to Ely.by Account
														</>
													) : (
														<>Requires Ely.by Account</>
													)}
												</Button>

												<Button
													type="button"
													variant="outline"
													size="sm"
													onClick={() =>
														handleDownloadSkin(selectedSkin.skinUrl, `ely_skin_${selectedSkin.id}`)
													}
													className="h-7 w-full border-zinc-800 text-xs text-zinc-300 hover:bg-zinc-900"
												>
													<Download className="mr-1.5 size-3.5" />
													Download Skin (.png)
												</Button>
											</div>
										</>
									) : (
										<div className="flex flex-1 flex-col items-center justify-center gap-2 text-center text-zinc-500">
											<Eye className="size-8 opacity-30" />
											<span className="text-xs">Select a skin to view in 3D</span>
										</div>
									)}
								</div>
							</div>
						</div>
					)}

					{/* Tab 2: Upload Custom Skin */}
					{activeTab === "upload" && (
						<div className="flex flex-1 flex-col gap-6 overflow-y-auto p-5 md:flex-row">
							{/* Left: Drag and Drop & Options */}
							<div className="flex flex-1 flex-col gap-4">
								<div>
									<h3 className="font-semibold text-sm text-zinc-100">
										Upload Custom Minecraft Skin
									</h3>
									<p className="mt-0.5 text-xs text-zinc-400">
										Choose a standard Minecraft skin texture (.png) from your computer.
									</p>
								</div>

								{/* Dropzone */}
								<button
									type="button"
									onDragOver={(e) => {
										e.preventDefault()
										setIsDragging(true)
									}}
									onDragLeave={() => setIsDragging(false)}
									onDrop={(e) => {
										e.preventDefault()
										setIsDragging(false)
										const files = e.dataTransfer.files
										if (files.length > 0) {
											processFile(files[0])
										}
									}}
									onClick={() => fileInputRef.current?.click()}
									className={`flex w-full cursor-pointer flex-col items-center justify-center gap-2 rounded-xl border-2 border-dashed p-6 text-center transition-all ${
										isDragging
											? "border-emerald-500 bg-emerald-500/5 text-emerald-400"
											: "border-zinc-800 bg-zinc-900/40 text-zinc-400 hover:border-zinc-700 hover:bg-zinc-900"
									}`}
								>
									<input
										ref={fileInputRef}
										type="file"
										accept="image/png"
										className="hidden"
										onChange={(e) => {
											const file = e.target.files?.[0]
											if (file) processFile(file)
										}}
									/>
									<div className="mb-1 flex size-10 items-center justify-center rounded-full bg-zinc-800/80 text-zinc-300">
										<Upload className="size-5" />
									</div>
									<div className="font-medium text-xs text-zinc-200">
										Drop your skin .png file here or{" "}
										<span className="text-emerald-400 underline">browse</span>
									</div>
									<div className="text-[11px] text-zinc-500">
										Supports 64x64 or 64x32 PNG (max 192 KB)
									</div>
								</button>

								{/* Upload Error Banner */}
								{uploadError && (
									<div className="flex items-center gap-2 rounded-lg border border-rose-500/20 bg-rose-500/10 p-3 text-rose-300 text-xs">
										<AlertCircle className="size-4 shrink-0" />
										<span>{uploadError}</span>
									</div>
								)}

								{/* Model selection */}
								<div className="flex flex-col gap-2 rounded-xl border border-zinc-800 bg-zinc-900/30 p-3.5">
									<div className="font-semibold text-xs text-zinc-200">Arm Model / Variant</div>
									<div className="grid grid-cols-2 gap-2">
										<button
											type="button"
											onClick={() => setUploadedModel("default")}
											className={`rounded-lg border p-2.5 text-left transition-all ${
												uploadedModel === "default"
													? "border-zinc-600 bg-zinc-800 text-zinc-100"
													: "border-zinc-800/80 bg-zinc-900 text-zinc-400 hover:text-zinc-200"
											}`}
										>
											<div className="font-medium text-xs">Classic (Steve)</div>
											<div className="mt-0.5 text-[10px] text-zinc-500">4-pixel arm width</div>
										</button>
										<button
											type="button"
											onClick={() => setUploadedModel("slim")}
											className={`rounded-lg border p-2.5 text-left transition-all ${
												uploadedModel === "slim"
													? "border-zinc-600 bg-zinc-800 text-zinc-100"
													: "border-zinc-800/80 bg-zinc-900 text-zinc-400 hover:text-zinc-200"
											}`}
										>
											<div className="font-medium text-xs">Slim (Alex)</div>
											<div className="mt-0.5 text-[10px] text-zinc-500">3-pixel arm width</div>
										</button>
									</div>
								</div>

								{/* File Metadata if selected */}
								{uploadValidationDetails && (
									<div className="flex items-center justify-between rounded-lg border border-zinc-800 bg-zinc-900/40 px-3 py-2 text-xs text-zinc-400">
										<span className="max-w-[160px] truncate font-medium text-zinc-300">
											{uploadedFileName}
										</span>
										<span className="text-[11px] text-zinc-500">
											{uploadValidationDetails.dimensions} • {uploadValidationDetails.sizeKb}
										</span>
									</div>
								)}

								{/* Upload Action Button */}
								<div className="mt-auto pt-2">
									<Button
										type="button"
										disabled={
											!uploadedDataUrl ||
											isApplying ||
											!targetAccount ||
											targetAccount.accountType !== "ely"
										}
										onClick={() => {
											if (uploadedDataUrl) {
												handleUploadSkin(uploadedDataUrl)
											}
										}}
										className="h-9 w-full bg-emerald-600 font-semibold text-white text-xs hover:bg-emerald-500"
									>
										{isApplying ? (
											<>
												<Loader2 className="mr-1.5 size-3.5 animate-spin" />
												Uploading & Applying...
											</>
										) : targetAccount?.accountType === "ely" ? (
											<>
												<Upload className="mr-1.5 size-3.5" />
												Upload & Apply to Ely.by Account
											</>
										) : (
											<>Requires Ely.by Account</>
										)}
									</Button>
								</div>
							</div>

							{/* Right: Live 3D Preview of Dropped Skin */}
							<div className="flex w-full flex-col items-center gap-3 md:w-[300px]">
								<div className="self-start font-semibold text-xs text-zinc-300">
									Live 3D Preview
								</div>
								<div className="relative flex aspect-[3/4] w-full items-center justify-center overflow-hidden rounded-xl border border-zinc-800 bg-zinc-900/60 shadow-inner">
									<SkinViewer3D
										skinUrl={uploadedDataUrl || DEFAULT_STEVE_SKIN}
										username={targetAccount?.username || "Steve"}
										width={280}
										height={340}
										model={uploadedModel}
										className="rounded-lg"
									/>
									{!uploadedDataUrl && (
										<div className="absolute inset-0 flex items-center justify-center bg-zinc-950/40 p-4 text-center backdrop-blur-[1px]">
											<span className="rounded-lg border border-zinc-800 bg-zinc-900/90 px-3 py-1.5 font-medium text-xs text-zinc-400">
												Drop a skin to preview
											</span>
										</div>
									)}
								</div>
							</div>
						</div>
					)}
				</DialogContent>
			</Dialog>

			{/* Sub-Dialog: Password prompt for initial Ely.by web authorization */}
			<Dialog open={showPasswordDialog} onOpenChange={setShowPasswordDialog}>
				<DialogContent className="border-zinc-800 bg-zinc-950 sm:max-w-md">
					<DialogHeader>
						<div className="mb-2 flex size-9 items-center justify-center rounded-md border border-amber-500/20 bg-amber-500/10 text-amber-400">
							<KeyRound className="size-4" />
						</div>
						<DialogTitle className="font-semibold text-sm text-zinc-100">
							Ely.by Password Authorization
						</DialogTitle>
						<DialogDescription className="text-xs text-zinc-400 leading-relaxed">
							Ely.by requires authentication to change account skins. Enter your password for{" "}
							<span className="font-medium text-zinc-200">{targetAccount?.username}</span>. It will
							be encrypted and securely stored in your native OS Credential Vault so you won't be
							asked again.
						</DialogDescription>
					</DialogHeader>

					<div className="flex flex-col gap-2 py-3">
						<div className="relative">
							<Lock className="absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-zinc-500" />
							<Input
								type="password"
								placeholder="Ely.by password"
								value={passwordInput}
								onChange={(e) => setPasswordInput(e.target.value)}
								onKeyDown={(e) => {
									if (e.key === "Enter") {
										handleConfirmPassword()
									}
								}}
								className="h-9 border-zinc-800 bg-zinc-900 pl-8 text-xs text-zinc-200"
								autoFocus
							/>
						</div>
						{passwordError && <span className="text-[11px] text-rose-400">{passwordError}</span>}
					</div>

					<DialogFooter className="gap-2 sm:gap-0">
						<Button
							type="button"
							variant="ghost"
							size="sm"
							onClick={() => {
								setShowPasswordDialog(false)
								setPasswordInput("")
							}}
							className="text-xs text-zinc-400"
						>
							Cancel
						</Button>
						<Button
							type="button"
							size="sm"
							disabled={isApplying || !passwordInput.trim()}
							onClick={handleConfirmPassword}
							className="bg-emerald-600 text-white text-xs hover:bg-emerald-500"
						>
							{isApplying ? (
								<>
									<Loader2 className="mr-1 size-3 animate-spin" />
									Authorizing...
								</>
							) : (
								"Authorize & Apply"
							)}
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
		</>
	)
}
export default SkinCatalogDialog
