import { keepPreviousData, queryOptions, useQuery, useQueryClient } from "@tanstack/react-query"
import { getRouteApi } from "@tanstack/react-router"
import i18n from "i18next"
import {
	AlertCircle,
	ArrowUpDown,
	Check,
	ChevronDown,
	ChevronLeft,
	ChevronRight,
	Copy,
	Download,
	Eye,
	Grid,
	KeyRound,
	Loader2,
	Lock,
	RefreshCw,
	Search,
	Sparkles,
	Trash2,
	Upload,
	User,
	Users,
	X,
} from "lucide-react"
import { memo, useCallback, useEffect, useMemo, useRef, useState } from "react"
import { Trans, useTranslation } from "react-i18next"
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
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { Input } from "@/components/ui/input"
import { ScrollArea } from "@/components/ui/scroll-area"
import SkinAvatar from "@/components/ui/skin-avatar"
import { formatBytes, formatCount, formatDate } from "@/lib/format"
import { cn } from "@/lib/utils"
import { accountService, skinStorageService, useAccounts } from "@/services/account-service"
import type { AccountProfile } from "@/types/account"
import type { ElySkinItem, SkinSortOption } from "@/types/skin"
import SkinPreviewCanvas from "./skin-preview-canvas"

const routeApi = getRouteApi("/skins")

/** Labels are skins.sorts.<id> in the locale files */
const SORT_OPTIONS: { id: SkinSortOption }[] = [
	{ id: "wearers" },
	{ id: "latest" },
	{ id: "views" },
	{ id: "cubes" },
]

const MODEL_OPTIONS: readonly ("any" | "steve" | "slim")[] = ["any", "steve", "slim"]

let lastKnownSkin: ElySkinItem | null = null

function formatNumber(num?: number | null): string {
	if (num === undefined || num === null || Number.isNaN(num)) return formatCount(0)
	return formatCount(num)
}

function normalizeSkinItem(raw: ElySkinItem | Record<string, unknown>): ElySkinItem {
	const id = "id" in raw && typeof raw.id === "number" ? raw.id : 0
	const skinUrl =
		"skinUrl" in raw && typeof raw.skinUrl === "string"
			? raw.skinUrl
			: "skin_url" in raw && typeof raw.skin_url === "string"
				? raw.skin_url
				: ""
	const isSlim =
		"isSlim" in raw && typeof raw.isSlim === "boolean"
			? raw.isSlim
			: "is_slim" in raw
				? Boolean(raw.is_slim)
				: false
	const countWearers =
		"countWearers" in raw && typeof raw.countWearers === "number"
			? raw.countWearers
			: "count_wearers" in raw && typeof raw.count_wearers === "number"
				? raw.count_wearers
				: 0
	const countCubes =
		"countCubes" in raw && typeof raw.countCubes === "number"
			? raw.countCubes
			: "count_cubes" in raw && typeof raw.count_cubes === "number"
				? raw.count_cubes
				: 0
	const countViews =
		"countViews" in raw && typeof raw.countViews === "number"
			? raw.countViews
			: "count_views" in raw && typeof raw.count_views === "number"
				? raw.count_views
				: 0
	const tags =
		"tags" in raw && Array.isArray(raw.tags)
			? raw.tags.filter((t): t is string => typeof t === "string")
			: []
	return {
		id,
		skinUrl,
		isSlim,
		countWearers,
		countCubes,
		countViews,
		tags,
	}
}

export interface SkinsQueryParams {
	tab: "catalog" | "my-skins" | "upload"
	page: number
	searchQuery: string
	sort: SkinSortOption
	model: "any" | "steve" | "slim"
	accountId?: string
	accountUsername?: string
	accountSkinUrl?: string | null
}

export const skinsQueryOptions = (params: SkinsQueryParams) =>
	queryOptions({
		queryKey: [
			"skins",
			params.tab,
			{
				page: params.page,
				q: params.searchQuery,
				sort: params.sort,
				model: params.model,
				account: params.tab === "my-skins" ? params.accountId : undefined,
				// Applying a skin gives the account a new cache-busted skin URL; keying on it
				// rebuilds the "current skin" entry instead of reusing the stale one
				accountSkin: params.tab === "my-skins" ? params.accountSkinUrl : undefined,
			},
		],
		queryFn: async () => {
			if (params.tab === "upload") {
				return { items: [], lastPage: 1, totalItems: 0 }
			}

			if (params.tab === "my-skins") {
				const localUploads = params.accountId
					? skinStorageService.getUploadedSkins(params.accountId)
					: []

				const localSkinItems: ElySkinItem[] = localUploads.map((u, idx) => ({
					id: -1000 - idx,
					skinUrl: u.dataUrl,
					dataUrl: u.dataUrl,
					isSlim: u.isSlim,
					countWearers: 1,
					countCubes: 0,
					countViews: 0,
					tags: [u.name || i18n.t("skinsPage.customSkin")],
					isCustom: true,
					name: u.name,
					uploadedAt: u.uploadedAt,
				}))

				let remoteItems: ElySkinItem[] = []
				if (params.accountUsername) {
					try {
						const res = await accountService.getElySkins(
							params.page,
							params.searchQuery || undefined,
							params.sort,
							params.model === "any" ? undefined : params.model,
							params.accountUsername,
						)
						remoteItems = (res.items || []).map(normalizeSkinItem)
					} catch (e) {
						console.warn("Could not fetch remote uploader skins:", e)
					}
				}

				const currentSkinItem: ElySkinItem[] = []
				if (
					params.accountSkinUrl &&
					!localSkinItems.some((s) => s.skinUrl === params.accountSkinUrl) &&
					!remoteItems.some((s) => s.skinUrl === params.accountSkinUrl)
				) {
					currentSkinItem.push({
						id: 0,
						skinUrl: params.accountSkinUrl,
						isSlim: false,
						countWearers: 1,
						countCubes: 0,
						countViews: 0,
						tags: [i18n.t("skinsPage.currentActiveSkin"), params.accountUsername || ""],
						name: i18n.t("skinsPage.activeAccountSkin"),
					})
				}

				let combined = [...localSkinItems, ...currentSkinItem, ...remoteItems]

				if (params.model === "slim") {
					combined = combined.filter((s) => s.isSlim)
				} else if (params.model === "steve") {
					combined = combined.filter((s) => !s.isSlim)
				}

				if (params.searchQuery) {
					const q = params.searchQuery.toLowerCase()
					combined = combined.filter(
						(s) =>
							s.tags.some((t) => t.toLowerCase().includes(q)) || s.name?.toLowerCase().includes(q),
					)
				}

				return {
					items: combined,
					lastPage: 1,
					totalItems: combined.length,
				}
			}

			// Public catalog
			const res = await accountService.getElySkins(
				params.page,
				params.searchQuery || undefined,
				params.sort,
				params.model === "any" ? undefined : params.model,
			)
			const normalized = (res.items || []).map(normalizeSkinItem)
			if (params.sort === "views") {
				normalized.sort((a, b) => b.countViews - a.countViews)
			} else if (params.sort === "cubes") {
				normalized.sort((a, b) => b.countCubes - a.countCubes)
			}

			return {
				items: normalized,
				lastPage: res.lastPage || 1,
				totalItems: res.totalItems || 0,
			}
		},
		staleTime: 1000 * 60 * 5,
		gcTime: 1000 * 60 * 30,
		placeholderData: keepPreviousData,
	})

export function SkinCatalogView({ initialAccount }: { initialAccount?: AccountProfile | null }) {
	const { t } = useTranslation()
	const search = routeApi.useSearch()
	const navigate = routeApi.useNavigate()

	const activeTab = search.tab ?? "catalog"
	const sortOption = search.sort ?? "wearers"
	const modelFilter = search.model ?? "any"
	const page = search.page ?? 1
	const searchQuery = search.q ?? ""
	const selectedSkinId = search.skinId

	const [searchInput, setSearchInput] = useState(searchQuery)

	const { accounts, activeAccount } = useAccounts()
	const targetAccount =
		initialAccount || activeAccount || accounts.find((a) => a.accountType === "ely") || null

	// Mobile view mode when activeTab is not "upload": "catalog" | "preview"
	const [mobileView, setMobileView] = useState<"catalog" | "preview">("catalog")

	// Query client & catalog query
	const queryClient = useQueryClient()
	const [selectedSkin, setSelectedSkin] = useState<ElySkinItem | null>(null)

	const { data, isLoading, isFetching, error, refetch } = useQuery(
		skinsQueryOptions({
			tab: activeTab,
			page,
			searchQuery,
			sort: sortOption,
			model: modelFilter,
			accountId: targetAccount?.id,
			accountUsername: targetAccount?.username,
			accountSkinUrl: targetAccount?.skinUrl,
		}),
	)

	const skins = data?.items ?? []
	const totalItems = data?.totalItems ?? 0
	const lastPage = data?.lastPage ?? 1
	const catalogError = error
		? error instanceof Error
			? error.message
			: t("skinsPage.loadFailed")
		: null
	const isLoadingCatalog = isLoading

	// Keep input synced if URL search param changes
	useEffect(() => {
		setSearchInput(searchQuery)
	}, [searchQuery])

	// Debounce search input to URL query
	useEffect(() => {
		const timer = setTimeout(() => {
			if (searchInput !== searchQuery) {
				navigate({
					search: (prev) => ({ ...prev, q: searchInput, page: 1 }),
					replace: true,
				})
			}
		}, 400)
		return () => clearTimeout(timer)
	}, [searchInput, searchQuery, navigate])

	// Fallback skin (active account skin or Steve) to guarantee a skin is ALWAYS visible in 3D
	const fallbackSkin: ElySkinItem = useMemo(
		() => ({
			id: 0,
			skinUrl: targetAccount?.skinUrl || DEFAULT_STEVE_SKIN,
			isSlim: false,
			countWearers: 0,
			countCubes: 0,
			countViews: 0,
			tags: targetAccount ? [targetAccount.username] : [t("skinsPage.defaultSteve")],
		}),
		[targetAccount, t],
	)

	const activeSkin = useMemo(() => {
		if (selectedSkinId !== undefined) {
			const match = skins.find((item) => item.id === selectedSkinId)
			if (match) {
				lastKnownSkin = match
				return match
			}
		}
		if (selectedSkin) {
			const match = skins.find(
				(item) =>
					item.skinUrl === selectedSkin.skinUrl || (item.id !== 0 && item.id === selectedSkin.id),
			)
			if (match) {
				lastKnownSkin = match
				return match
			}
		}
		if (skins[0]) {
			lastKnownSkin = skins[0]
			return skins[0]
		}
		return lastKnownSkin || fallbackSkin
	}, [selectedSkinId, selectedSkin, skins, fallbackSkin])

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

	const handleTabChange = useCallback(
		(tab: "catalog" | "my-skins" | "upload") => {
			navigate({
				search: (prev) => ({
					...prev,
					tab,
					page: 1,
					q: "",
					skinId: undefined,
				}),
			})
			setSelectedSkin(null)
			lastKnownSkin = null
		},
		[navigate],
	)

	// Reset status message after delay
	useEffect(() => {
		if (!applyStatusMessage) return
		const timer = setTimeout(() => {
			setApplyStatusMessage(null)
		}, 6000)
		return () => clearTimeout(timer)
	}, [applyStatusMessage])

	// Handle applying selected skin to active Ely.by account
	const handleApplySkin = async (skin: ElySkinItem) => {
		if (!targetAccount) {
			setApplyStatusMessage({
				type: "error",
				text: t("skinsPage.addAccountFirst"),
			})
			return
		}

		if (targetAccount.accountType !== "ely") {
			setApplyStatusMessage({
				type: "error",
				text: t("skinsPage.elyOnlySync"),
			})
			return
		}

		setIsApplying(true)
		setApplyStatusMessage(null)

		try {
			const hasCreds = await accountService.hasElyWebCredentials(targetAccount.id)
			if (!hasCreds) {
				if (skin.dataUrl) {
					setPendingAction({ type: "upload", dataUrl: skin.dataUrl })
				} else if (skin.id > 0) {
					setPendingAction({ type: "apply", skinId: skin.id })
				}
				setPasswordError(null)
				setPasswordInput("")
				setShowPasswordDialog(true)
				setIsApplying(false)
				return
			}

			if (skin.dataUrl) {
				await accountService.uploadElySkin(targetAccount.id, skin.dataUrl)
				setApplyStatusMessage({
					type: "success",
					text: t("skinsPage.customApplied", { name: targetAccount.username }),
				})
			} else if (skin.id > 0) {
				await accountService.applyElySkin(targetAccount.id, skin.id)
				setApplyStatusMessage({
					type: "success",
					text: t("skinsPage.skinApplied", { id: skin.id, name: targetAccount.username }),
				})
			}
			await queryClient.invalidateQueries({ queryKey: ["skins"] })
		} catch (err: unknown) {
			const msg = err instanceof Error ? err.message : String(err)
			if (msg.includes("password") || msg.includes("auth") || msg.includes("credentials")) {
				if (skin.dataUrl) {
					setPendingAction({ type: "upload", dataUrl: skin.dataUrl })
				} else if (skin.id > 0) {
					setPendingAction({ type: "apply", skinId: skin.id })
				}
				setPasswordError(msg)
				setPasswordInput("")
				setShowPasswordDialog(true)
			} else {
				setApplyStatusMessage({
					type: "error",
					text: msg || t("skinsPage.applyFailed"),
				})
			}
		} finally {
			setIsApplying(false)
		}
	}

	// Handle executing action with provided password
	const handleProcessPendingAction = async () => {
		if (!pendingAction || !targetAccount) return
		if (!passwordInput.trim()) {
			setPasswordError(t("skinsPage.enterPassword"))
			return
		}

		setIsApplying(true)
		setPasswordError(null)

		try {
			if (pendingAction.type === "apply") {
				await accountService.applyElySkin(targetAccount.id, pendingAction.skinId, passwordInput)
				setApplyStatusMessage({
					type: "success",
					text: t("skinsPage.skinApplied", {
						id: pendingAction.skinId,
						name: targetAccount.username,
					}),
				})
			} else if (pendingAction.type === "upload") {
				skinStorageService.saveUploadedSkin(targetAccount.id, {
					id: `custom_${Date.now()}`,
					name: uploadedFileName || t("skinsPage.skinFromDate", { date: formatDate(Date.now()) }),
					dataUrl: pendingAction.dataUrl,
					isSlim: uploadedModel === "slim",
					uploadedAt: Date.now(),
				})
				await accountService.uploadElySkin(targetAccount.id, pendingAction.dataUrl, passwordInput)
				setApplyStatusMessage({
					type: "success",
					text: t("skinsPage.uploadedApplied", { name: targetAccount.username }),
				})
				handleTabChange("my-skins")
			}
			setShowPasswordDialog(false)
			setPendingAction(null)
			setPasswordInput("")
			await queryClient.invalidateQueries({ queryKey: ["skins"] })
		} catch (err: unknown) {
			setPasswordError(err instanceof Error ? err.message : String(err))
		} finally {
			setIsApplying(false)
		}
	}

	const handleRemoveCustomSkin = (skin: ElySkinItem, e: React.MouseEvent) => {
		e.stopPropagation()
		if (!targetAccount) return
		const localUploads = skinStorageService.getUploadedSkins(targetAccount.id)
		const match = localUploads.find((u) => u.dataUrl === skin.skinUrl || u.id === String(skin.id))
		if (match) {
			skinStorageService.removeUploadedSkin(targetAccount.id, match.id)
			queryClient.invalidateQueries({ queryKey: ["skins"] })
		}
	}

	// Handle local file validation & upload preview
	const handleFileSelected = (file: File) => {
		setUploadError(null)
		setUploadValidationDetails(null)

		if (!file.type.includes("png")) {
			setUploadError(t("skinsPage.pngOnly"))
			return
		}

		if (file.size > 2 * 1024 * 1024) {
			setUploadError(t("skinsPage.tooLarge"))
			return
		}

		const reader = new FileReader()
		reader.onload = (e) => {
			const dataUrl = typeof e.target?.result === "string" ? e.target.result : null
			if (!dataUrl) return

			const img = new Image()
			img.onload = () => {
				const w = img.naturalWidth
				const h = img.naturalHeight

				const isValidMinecraftSkin =
					(w === 64 && h === 64) ||
					(w === 64 && h === 32) ||
					(w === 128 && h === 128) ||
					(w === 128 && h === 64)

				if (!isValidMinecraftSkin) {
					setUploadError(t("skinsPage.badSize", { size: `${w}x${h}` }))
					return
				}

				setUploadedDataUrl(dataUrl)
				setUploadedFileName(file.name)
				setUploadValidationDetails({
					dimensions: `${w}x${h} px`,
					sizeKb: formatBytes(file.size),
				})
			}
			img.onerror = () => {
				setUploadError(t("skinsPage.badImage"))
			}
			img.src = dataUrl
		}
		reader.readAsDataURL(file)
	}

	const handleDrop = (e: React.DragEvent) => {
		e.preventDefault()
		setIsDragging(false)
		if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
			handleFileSelected(e.dataTransfer.files[0])
		}
	}

	const handleDragOver = (e: React.DragEvent) => {
		e.preventDefault()
		setIsDragging(true)
	}

	const handleDragLeave = () => {
		setIsDragging(false)
	}

	const handleUploadSkin = async () => {
		if (!uploadedDataUrl || !targetAccount) return
		if (targetAccount.accountType !== "ely") {
			setApplyStatusMessage({
				type: "error",
				text: t("skinsPage.elyOnlyUpload"),
			})
			return
		}

		setIsApplying(true)
		try {
			const hasCreds = await accountService.hasElyWebCredentials(targetAccount.id)
			if (!hasCreds) {
				setPendingAction({ type: "upload", dataUrl: uploadedDataUrl })
				setPasswordError(null)
				setPasswordInput("")
				setShowPasswordDialog(true)
				setIsApplying(false)
				return
			}

			skinStorageService.saveUploadedSkin(targetAccount.id, {
				id: `custom_${Date.now()}`,
				name: uploadedFileName || t("skinsPage.skinFromDate", { date: formatDate(Date.now()) }),
				dataUrl: uploadedDataUrl,
				isSlim: uploadedModel === "slim",
				uploadedAt: Date.now(),
			})
			await accountService.uploadElySkin(targetAccount.id, uploadedDataUrl)
			setApplyStatusMessage({
				type: "success",
				text: t("skinsPage.uploadedApplied", { name: targetAccount.username }),
			})
			handleTabChange("my-skins")
			await queryClient.invalidateQueries({ queryKey: ["skins"] })
		} catch (err: unknown) {
			const msg = err instanceof Error ? err.message : String(err)
			if (msg.includes("password") || msg.includes("auth") || msg.includes("credentials")) {
				setPendingAction({ type: "upload", dataUrl: uploadedDataUrl })
				setPasswordError(msg)
				setPasswordInput("")
				setShowPasswordDialog(true)
			} else {
				setApplyStatusMessage({
					type: "error",
					text: msg || t("skinsPage.uploadFailed"),
				})
			}
		} finally {
			setIsApplying(false)
		}
	}

	const handleDownloadSkin = async (skin: ElySkinItem) => {
		if (!skin.skinUrl) return
		try {
			const savedPath = await accountService.saveSkinToDownloads(`skin_${skin.id}`, skin.skinUrl)
			if (!savedPath) return // User cancelled file dialog
			setApplyStatusMessage({
				type: "success",
				text: t("skinsPage.saved", { id: skin.id }),
			})
		} catch (err: unknown) {
			setApplyStatusMessage({
				type: "error",
				text: err instanceof Error ? err.message : t("skinsPage.downloadFailed"),
			})
		}
	}

	const handleCopyUrl = (url: string) => {
		navigator.clipboard.writeText(url)
		setApplyStatusMessage({
			type: "success",
			text: t("skinsPage.urlCopied"),
		})
	}

	return (
		<div className="flex size-full min-h-0 flex-1 flex-col gap-4 overflow-hidden p-4 sm:p-5 lg:p-6">
			{/* Page Header */}
			<div className="flex shrink-0 flex-col justify-between gap-2 sm:flex-row sm:items-center">
				<div>
					<h1 className="font-bold text-foreground text-xl tracking-tight">{t("skins.title")}</h1>
					<p className="text-muted-foreground text-xs">{t("skins.subtitle")}</p>
				</div>

				<div className="flex flex-wrap items-center gap-2">
					{/* Target Account Badge */}
					{targetAccount ? (
						<div className="flex items-center gap-2 rounded-lg border border-border/40 bg-zinc-900/60 px-2.5 py-1.5 text-xs text-zinc-300">
							<SkinAvatar
								username={targetAccount.username}
								skinUrl={targetAccount.skinUrl}
								size={20}
							/>
							<span className="max-w-[120px] truncate font-medium">{targetAccount.username}</span>
							<span
								className="size-1.5 rounded-full bg-emerald-500"
								title={t("skinsPage.activeAccount")}
							/>
						</div>
					) : (
						<div className="flex items-center gap-1.5 rounded-lg border border-amber-500/20 bg-amber-500/5 px-2.5 py-1.5 text-amber-400 text-xs">
							<AlertCircle className="size-3.5" />
							<span>{t("skinsPage.noActiveAccount")}</span>
						</div>
					)}

					{/* Tab Buttons */}
					<div className="flex rounded-lg border border-border/40 bg-zinc-900/60 p-0.5">
						<Button
							type="button"
							variant="ghost"
							size="sm"
							className={cn(
								"h-7 rounded-md px-3 font-medium text-xs transition-colors",
								activeTab === "catalog"
									? "bg-zinc-800 text-zinc-100 shadow-xs"
									: "text-zinc-400 hover:text-zinc-200",
							)}
							onClick={() => handleTabChange("catalog")}
						>
							<Search className="mr-1.5 size-3.5" />
							{t("skins.tabs.catalog")}
						</Button>
						<Button
							type="button"
							variant="ghost"
							size="sm"
							className={cn(
								"h-7 rounded-md px-3 font-medium text-xs transition-colors",
								activeTab === "my-skins"
									? "bg-zinc-800 text-zinc-100 shadow-xs"
									: "text-zinc-400 hover:text-zinc-200",
							)}
							onClick={() => handleTabChange("my-skins")}
						>
							<User className="mr-1.5 size-3.5" />
							{t("skins.tabs.mySkins")}
						</Button>
						<Button
							type="button"
							variant="ghost"
							size="sm"
							className={cn(
								"h-7 rounded-md px-3 font-medium text-xs transition-colors",
								activeTab === "upload"
									? "bg-zinc-800 text-zinc-100 shadow-xs"
									: "text-zinc-400 hover:text-zinc-200",
							)}
							onClick={() => handleTabChange("upload")}
						>
							<Upload className="mr-1.5 size-3.5" />
							{t("skins.tabs.upload")}
						</Button>
					</div>
				</div>
			</div>

			{/* Status Feedback Banner */}
			{applyStatusMessage && (
				<div
					className={cn(
						"flex shrink-0 items-center justify-between rounded-lg border px-4 py-2 text-xs transition-all",
						applyStatusMessage.type === "success"
							? "border-emerald-500/30 bg-emerald-500/10 text-emerald-300"
							: "border-rose-500/30 bg-rose-500/10 text-rose-300",
					)}
				>
					<div className="flex items-center gap-2">
						{applyStatusMessage.type === "success" ? (
							<Check className="size-4 shrink-0 text-emerald-400" />
						) : (
							<AlertCircle className="size-4 shrink-0 text-rose-400" />
						)}
						<span>{applyStatusMessage.text}</span>
					</div>
					<button
						type="button"
						onClick={() => setApplyStatusMessage(null)}
						className="opacity-70 hover:opacity-100"
					>
						<X className="size-3.5" />
					</button>
				</div>
			)}

			{/* Main Studio View Card */}
			<div className="flex size-full min-h-0 flex-1 flex-col overflow-hidden rounded-2xl border border-border/40 bg-zinc-900/40 shadow-2xl backdrop-blur-md">
				{/* Tab 1 & 2: Catalog & My Uploads */}
				{activeTab !== "upload" && (
					<div className="flex size-full min-h-0 flex-1 flex-col overflow-hidden md:flex-row">
						{/* Mobile Sub-Navigation Toggle: Browse vs 3D Preview (Visible only on < md) */}
						<div className="flex shrink-0 border-border/30 border-b bg-zinc-950/60 p-1.5 md:hidden">
							<button
								type="button"
								className={cn(
									"flex flex-1 items-center justify-center gap-1.5 rounded-md py-1.5 font-medium text-xs transition-colors",
									mobileView === "catalog"
										? "bg-zinc-800 text-zinc-100 shadow-xs"
										: "text-zinc-400 hover:text-zinc-200",
								)}
								onClick={() => setMobileView("catalog")}
							>
								<Grid className="size-3.5" />
								<span>
									{activeTab === "my-skins" ? t("skins.tabs.mySkins") : t("skinsPage.browse")}{" "}
									{totalItems > 0 ? `(${formatNumber(totalItems)})` : ""}
								</span>
							</button>
							<button
								type="button"
								className={cn(
									"flex flex-1 items-center justify-center gap-1.5 rounded-md py-1.5 font-medium text-xs transition-colors",
									mobileView === "preview"
										? "bg-zinc-800 text-zinc-100 shadow-xs"
										: "text-zinc-400 hover:text-zinc-200",
								)}
								onClick={() => setMobileView("preview")}
							>
								<Sparkles className="size-3.5 text-emerald-400" />
								<span>{t("skinsPage.preview3d")}</span>
							</button>
						</div>

						{/* Left Showcase: Character Studio & Actions */}
						<div
							className={cn(
								"relative flex h-full shrink-0 flex-col justify-between overflow-hidden border-border/30 bg-gradient-to-b from-zinc-950 via-zinc-900/60 to-zinc-950 md:w-[320px] md:border-r md:border-b-0 lg:w-[350px]",
								mobileView === "preview" ? "flex w-full" : "hidden md:flex",
							)}
						>
							{/* Background Studio Ambience */}
							<div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_50%_40%,rgba(16,185,129,0.08),transparent_70%)]" />
							<div className="pointer-events-none absolute bottom-24 left-1/2 h-10 w-48 -translate-x-1/2 rounded-[100%] bg-emerald-500/10 blur-md" />

							{/* Full-bleed 3D Viewer Stage - Fills entire container behind overlays */}
							<div className="absolute inset-0 z-0 size-full">
								<SkinViewer3D
									skinUrl={activeSkin.skinUrl}
									username={`skin_${activeSkin.id}`}
									showToolbar={false}
									model={activeSkin.isSlim ? "slim" : "default"}
									autoResize={true}
									borderless={true}
									zoom={0.56}
									floatingControlsClassName="top-14 right-3"
									className="size-full"
								/>
							</div>

							{/* Showcase Header (Floating Overlay) */}
							<div className="pointer-events-none relative z-10 flex shrink-0 items-center justify-between bg-gradient-to-b from-zinc-950/90 via-zinc-950/50 to-transparent p-4 lg:p-5">
								<div className="pointer-events-auto flex items-center gap-1.5">
									<Button
										variant="ghost"
										size="xs"
										className="h-6 px-1.5 text-xs text-zinc-400 hover:text-zinc-200 md:hidden"
										onClick={() => setMobileView("catalog")}
									>
										<ChevronLeft className="mr-0.5 size-3.5" />
										{t("skins.tabs.catalog")}
									</Button>
									<span className="font-semibold text-xs text-zinc-200">
										{activeSkin.name
											? activeSkin.name
											: activeSkin.id === 0
												? t("skinsPage.currentSkin")
												: t("skinsPage.skinNumber", { id: activeSkin.id })}
									</span>
								</div>
								<span className="pointer-events-auto rounded-full border border-zinc-800/80 bg-zinc-900/90 px-2.5 py-0.5 font-medium text-[10px] text-zinc-300 backdrop-blur-xs">
									{activeSkin.isSlim ? t("skinsPage.alexSlim3") : t("skinsPage.steveClassic4")}
								</span>
							</div>

							{/* Transparent Interaction Spacer - lets clicks fall through to 3D canvas */}
							<div className="pointer-events-none min-h-0 flex-1" />

							{/* Primary Apply Button & Secondary Actions (Floating Overlay) */}
							<div className="pointer-events-none relative z-10 flex shrink-0 flex-col gap-2 bg-gradient-to-t from-zinc-950/95 via-zinc-950/60 to-transparent p-4 lg:p-5">
								<Button
									size="default"
									className="pointer-events-auto h-11 w-full gap-2 rounded-xl bg-gradient-to-r from-emerald-600 to-emerald-500 font-semibold text-sm text-white shadow-emerald-950/50 shadow-lg transition-all hover:from-emerald-500 hover:to-emerald-400 hover:shadow-emerald-900/60 active:scale-[0.99] disabled:opacity-50"
									disabled={
										isApplying || !targetAccount || (activeSkin.id === 0 && !activeSkin.dataUrl)
									}
									onClick={() =>
										(activeSkin.id !== 0 || !!activeSkin.dataUrl) && handleApplySkin(activeSkin)
									}
								>
									{isApplying ? (
										<Loader2 className="size-4 animate-spin" />
									) : (
										<Sparkles className="size-4" />
									)}
									{activeSkin.id === 0 && !activeSkin.dataUrl
										? t("skinsPage.currentSkinOf", { name: targetAccount?.username ?? "" })
										: targetAccount
											? t("skinsPage.applyTo", { name: targetAccount.username })
											: t("skinsPage.applyToAccount")}
								</Button>

								<div className="pointer-events-auto grid grid-cols-2 gap-2">
									<Button
										variant="outline"
										size="xs"
										className="h-8 gap-1.5 rounded-lg border-zinc-800/80 bg-zinc-900/80 text-xs text-zinc-300 backdrop-blur-xs hover:border-zinc-700 hover:bg-zinc-850"
										onClick={() => handleDownloadSkin(activeSkin)}
									>
										<Download className="size-3.5" />
										{t("skinsPage.downloadPng")}
									</Button>
									<Button
										variant="outline"
										size="xs"
										className="h-8 gap-1.5 rounded-lg border-zinc-800/80 bg-zinc-900/80 text-xs text-zinc-300 backdrop-blur-xs hover:border-zinc-700 hover:bg-zinc-850"
										onClick={() => handleCopyUrl(activeSkin.skinUrl)}
									>
										<Copy className="size-3.5" />
										{t("skinPreview.copyUrl")}
									</Button>
								</div>
							</div>
						</div>

						{/* Right Column: Catalog Browser */}
						<div
							className={cn(
								"flex size-full min-w-0 flex-1 flex-col overflow-hidden",
								mobileView === "preview" ? "hidden md:flex" : "flex",
							)}
						>
							{/* Filter Toolbar - Clean Single Row Layout */}
							<div className="flex shrink-0 flex-wrap items-center justify-between gap-3 border-border/30 border-b bg-zinc-950/40 px-4 py-2.5">
								{/* Search Bar */}
								<div className="relative min-w-[180px] max-w-xs flex-1">
									<Search className="absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-zinc-500" />
									<Input
										type="text"
										placeholder={t("skins.searchPlaceholder")}
										value={searchInput}
										onChange={(e) => setSearchInput(e.target.value)}
										className="h-8 rounded-lg border-zinc-800 bg-zinc-900/80 pr-7 pl-8 text-xs text-zinc-200 placeholder:text-zinc-500 focus-visible:ring-emerald-500/50"
									/>
									{searchInput && (
										<button
											type="button"
											onClick={() => {
												setSearchInput("")
												navigate({
													search: (prev) => ({ ...prev, q: "", page: 1 }),
													replace: true,
												})
											}}
											className="absolute top-1/2 right-2 -translate-y-1/2 text-zinc-500 hover:text-zinc-300"
										>
											<X className="size-3" />
										</button>
									)}
								</div>

								{/* Filter Controls: Model & Sort */}
								<div className="flex items-center gap-2">
									{/* Model Filter */}
									<div className="flex items-center gap-0.5 rounded-lg border border-zinc-800 bg-zinc-900/80 p-0.5">
										{MODEL_OPTIONS.map((m) => (
											<button
												key={m}
												type="button"
												className={cn(
													"rounded-md px-2.5 py-1 font-medium text-[11px] transition-colors",
													modelFilter === m
														? "bg-zinc-800 text-zinc-100 shadow-xs"
														: "text-zinc-400 hover:text-zinc-200",
												)}
												onClick={() => {
													navigate({
														search: (prev) => ({ ...prev, model: m, page: 1 }),
													})
												}}
											>
												{t(`skins.models.${m}`)}
											</button>
										))}
									</div>

									{/* Sort Dropdown */}
									<DropdownMenu>
										<DropdownMenuTrigger
											render={
												<Button
													variant="outline"
													size="xs"
													className="h-8 gap-1.5 rounded-lg border-zinc-800 bg-zinc-900/80 text-[11px] text-zinc-300 hover:bg-zinc-800"
												>
													<ArrowUpDown className="size-3 text-zinc-400" />
													<span>{t(`skins.sorts.${sortOption}`)}</span>
													<ChevronDown className="size-3 opacity-60" />
												</Button>
											}
										/>
										<DropdownMenuContent
											align="end"
											className="border-zinc-800 bg-zinc-950 p-1 text-zinc-200"
										>
											{SORT_OPTIONS.map((s) => (
												<DropdownMenuItem
													key={s.id}
													onClick={() => {
														navigate({
															search: (prev) => ({ ...prev, sort: s.id, page: 1 }),
														})
													}}
													className={cn(
														"flex cursor-pointer items-center justify-between gap-2 rounded-md px-2 py-1.5 text-xs hover:bg-zinc-900",
														sortOption === s.id && "bg-zinc-900 font-medium text-emerald-400",
													)}
												>
													<span>{t(`skins.sorts.${s.id}`)}</span>
													{sortOption === s.id && <Check className="size-3 text-emerald-400" />}
												</DropdownMenuItem>
											))}
										</DropdownMenuContent>
									</DropdownMenu>

									<Button
										variant="outline"
										size="xs"
										onClick={() => refetch()}
										disabled={isFetching}
										className="h-8 gap-1.5 rounded-lg border-zinc-800 bg-zinc-900/80 px-2 text-[11px] text-zinc-300 hover:bg-zinc-800"
										title={t("common.refresh")}
									>
										<RefreshCw
											className={cn(
												"size-3 text-zinc-400",
												isFetching && "animate-spin text-emerald-400",
											)}
										/>
										<span className="hidden sm:inline">{t("common.refresh")}</span>
									</Button>

									{totalItems > 0 && (
										<span className="hidden font-mono text-[11px] text-zinc-500 xl:inline">
											{t("skinsPage.count", {
												count: totalItems,
												formatted: formatNumber(totalItems),
											})}
										</span>
									)}
								</div>
							</div>

							{/* Skins Grid Area */}
							<div className="relative flex min-h-0 flex-1 flex-col overflow-hidden">
								{isFetching && skins.length > 0 && (
									<div className="absolute top-2 right-3 z-10 flex items-center gap-1.5 rounded-full border border-zinc-700/80 bg-zinc-900/95 px-2.5 py-1 text-[11px] text-zinc-300 shadow-md backdrop-blur-xs">
										<Loader2 className="size-3 animate-spin text-emerald-400" />
										<span>{t("skinsPage.updating")}</span>
									</div>
								)}

								<ScrollArea className="flex-1" viewportClassName="p-4">
									{isLoading && skins.length === 0 ? (
										<div className="flex h-full min-h-[300px] flex-col items-center justify-center gap-2 text-zinc-400">
											<Loader2 className="size-6 animate-spin text-emerald-400" />
											<span className="text-xs">
												{activeTab === "my-skins"
													? t("skinsPage.loadingMine")
													: t("skinsPage.loading")}
											</span>
										</div>
									) : catalogError ? (
										<div className="flex h-full min-h-[300px] flex-col items-center justify-center gap-3 p-4 text-center">
											<AlertCircle className="size-8 text-rose-400" />
											<p className="max-w-sm text-rose-300 text-xs">{catalogError}</p>
											<Button
												size="sm"
												variant="outline"
												onClick={() => refetch()}
												className="h-7 text-xs"
											>
												<RefreshCw className="mr-1.5 size-3" />
												{t("install.retry")}
											</Button>
										</div>
									) : skins.length === 0 ? (
										<div className="flex h-full min-h-[300px] flex-col items-center justify-center gap-3 text-center text-zinc-500">
											{activeTab === "my-skins" ? (
												<>
													<div className="flex size-12 items-center justify-center rounded-full border border-zinc-800 bg-zinc-900/80">
														<User className="size-6 text-zinc-400" />
													</div>
													<p className="font-medium text-xs text-zinc-300">
														{searchQuery
															? t("skinsPage.noMatchingUploads")
															: targetAccount
																? t("skinsPage.noUploadsFor", { name: targetAccount.username })
																: t("skinsPage.noActiveAccount")}
													</p>
													<p className="max-w-xs text-[11px] text-zinc-500">
														{searchQuery
															? t("skinsPage.tryDifferent")
															: targetAccount
																? t("skinsPage.noUploadsHint")
																: t("skinsPage.selectAccountHint")}
													</p>
													{!searchQuery && targetAccount && (
														<Button
															type="button"
															size="sm"
															className="mt-1 h-8 gap-1.5 rounded-lg bg-emerald-600 px-3 text-white text-xs hover:bg-emerald-500"
															onClick={() => handleTabChange("upload")}
														>
															<Upload className="size-3.5" />
															{t("skinsPage.uploadASkin")}
														</Button>
													)}
												</>
											) : (
												<>
													<Search className="size-8 opacity-40" />
													<p className="font-medium text-xs text-zinc-400">
														{t("skinsPage.noSkins")}
													</p>
													<p className="text-[11px] text-zinc-500">{t("skinsPage.tryDifferent")}</p>
												</>
											)}
										</div>
									) : (
										<div className="grid grid-cols-[repeat(auto-fill,minmax(140px,1fr))] gap-3">
											{skins.map((skin) => {
												const isSelected =
													activeSkin.skinUrl === skin.skinUrl ||
													(activeSkin.id !== 0 && activeSkin.id === skin.id)
												const title =
													skin.name ||
													(skin.tags.length > 0
														? skin.tags[0].replace(/_/g, " ")
														: t("skinsPage.skinNumber", { id: skin.id }))

												return (
													<button
														key={skin.id || skin.skinUrl}
														type="button"
														onClick={() => {
															lastKnownSkin = skin
															setSelectedSkin(skin)
															navigate({
																search: (prev) => ({ ...prev, skinId: skin.id }),
															})
														}}
														className={cn(
															"group relative flex flex-col rounded-xl border p-2.5 text-left transition-all duration-150 hover:shadow-md",
															isSelected
																? "border-emerald-500/80 bg-emerald-500/10 shadow-emerald-950/40 ring-1 ring-emerald-500/60"
																: "border-zinc-800/80 bg-zinc-900/50 hover:border-zinc-700/80 hover:bg-zinc-850/60",
														)}
													>
														{/* Active Selection Badge */}
														{isSelected && (
															<div className="absolute top-2 right-2 z-10 flex size-4 items-center justify-center rounded-full bg-emerald-500 text-zinc-950 shadow-xs">
																<Check className="size-2.5 stroke-[3]" />
															</div>
														)}

														{/* Custom Skin Remove Button */}
														{skin.isCustom && (
															<button
																type="button"
																onClick={(e) => {
																	e.stopPropagation()
																	handleRemoveCustomSkin(skin, e)
																}}
																className="absolute top-2 right-2 z-10 flex size-6 items-center justify-center rounded-lg bg-zinc-900/90 text-zinc-400 any-pointer-coarse:opacity-100 opacity-0 transition-all hover:bg-rose-500/20 hover:text-rose-400 focus-visible:opacity-100 group-hover:opacity-100"
																title={t("skinsPage.deleteCustom")}
															>
																<Trash2 className="size-3" />
															</button>
														)}

														{/* 2D Skin Body Preview */}
														<div className="flex h-36 w-full items-center justify-center overflow-hidden rounded-lg bg-zinc-950/60 p-2">
															<SkinPreviewCanvas
																skinUrl={skin.dataUrl || skin.skinUrl}
																isSlim={skin.isSlim}
																height={128}
																className="drop-shadow-md transition-transform duration-200 group-hover:scale-105"
															/>
														</div>

														{/* Skin Metadata Info */}
														<div className="mt-2 flex flex-col gap-0.5">
															<span
																className="truncate font-medium text-xs text-zinc-200 group-hover:text-emerald-400"
																title={title}
															>
																{title}
															</span>
															<div className="flex items-center justify-between text-[10px] text-zinc-500">
																<span>
																	{skin.isSlim ? t("skinsPage.slim") : t("skinsPage.classic")}
																</span>
																{skin.countWearers > 0 && (
																	<span className="flex items-center gap-0.5 font-mono">
																		<Users className="size-2.5" />
																		{formatNumber(skin.countWearers)}
																	</span>
																)}
															</div>
														</div>
													</button>
												)
											})}
										</div>
									)}
								</ScrollArea>

								{/* Pagination Bar */}
								<div className="flex shrink-0 flex-col gap-2 border-border/30 border-t bg-zinc-950/40 px-4 py-2 sm:flex-row sm:items-center sm:justify-between">
									<span className="text-[11px] text-zinc-400">
										<Trans
											i18nKey="skinsPage.pageOf"
											values={{ page, total: lastPage }}
											components={{ b: <strong className="text-zinc-200" /> }}
										/>
										{totalItems > 0 && (
											<span className="ml-1 text-zinc-500">
												{t("skinsPage.total", { count: formatNumber(totalItems) })}
											</span>
										)}
									</span>
									<div className="flex items-center gap-1.5">
										<Button
											size="sm"
											variant="ghost"
											disabled={page <= 1 || isLoadingCatalog}
											onClick={() =>
												navigate({
													search: (prev) => ({ ...prev, page: Math.max(1, page - 1) }),
												})
											}
											className="h-7 px-2 text-xs"
										>
											<ChevronLeft className="mr-0.5 size-3.5" />
											{t("skinsPage.previous")}
										</Button>
										<Button
											size="sm"
											variant="ghost"
											disabled={page >= lastPage || isLoadingCatalog}
											onClick={() =>
												navigate({
													search: (prev) => ({ ...prev, page: Math.min(lastPage, page + 1) }),
												})
											}
											className="h-7 px-2 text-xs"
										>
											{t("skinsPage.next")}
											<ChevronRight className="ml-0.5 size-3.5" />
										</Button>
									</div>
								</div>

								{/* Mobile Sticky Action Bar */}
								<div className="flex shrink-0 items-center justify-between gap-3 border-border/30 border-t bg-zinc-950/95 px-3 py-2 backdrop-blur-md md:hidden">
									<div className="flex min-w-0 items-center gap-2">
										<SkinAvatar
											username={`skin_${activeSkin.id}`}
											skinUrl={activeSkin.skinUrl}
											size={26}
										/>
										<div className="truncate">
											<p className="truncate font-semibold text-xs text-zinc-200">
												{activeSkin.name
													? activeSkin.name
													: activeSkin.id === 0
														? t("skinsPage.currentSkin")
														: t("skinsPage.skinNumber", { id: activeSkin.id })}
											</p>
											<span className="text-[10px] text-zinc-400">
												{activeSkin.isSlim ? t("skinsPage.alexSlim") : t("skinsPage.steveClassic")}
											</span>
										</div>
									</div>
									<div className="flex shrink-0 items-center gap-1.5">
										<Button
											variant="outline"
											size="xs"
											className="h-7 gap-1 border-zinc-800 text-xs text-zinc-300 hover:bg-zinc-900"
											onClick={() => setMobileView("preview")}
										>
											<Eye className="size-3" />
											{t("skinsPage.view3d")}
										</Button>
										<Button
											size="xs"
											className="h-7 gap-1 bg-emerald-600 font-medium text-white text-xs hover:bg-emerald-500"
											disabled={
												isApplying || !targetAccount || (activeSkin.id === 0 && !activeSkin.dataUrl)
											}
											onClick={() =>
												(activeSkin.id !== 0 || !!activeSkin.dataUrl) && handleApplySkin(activeSkin)
											}
										>
											{isApplying ? (
												<Loader2 className="size-3 animate-spin" />
											) : (
												<Sparkles className="size-3" />
											)}
											{t("skinsPage.apply")}
										</Button>
									</div>
								</div>
							</div>
						</div>
					</div>
				)}

				{/* Tab 2: Upload Custom Skin */}
				{activeTab === "upload" && (
					<div className="flex size-full min-h-0 flex-1 flex-col overflow-hidden md:flex-row">
						{/* Left Showcase: Live 3D Preview Stage */}
						<div className="relative flex h-full shrink-0 flex-col justify-between overflow-hidden border-border/30 bg-gradient-to-b from-zinc-950 via-zinc-900/60 to-zinc-950 md:w-[320px] md:border-r md:border-b-0 lg:w-[350px]">
							{/* Background Studio Ambience */}
							<div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_50%_40%,rgba(16,185,129,0.08),transparent_70%)]" />
							<div className="pointer-events-none absolute bottom-24 left-1/2 h-10 w-48 -translate-x-1/2 rounded-[100%] bg-emerald-500/10 blur-md" />

							{/* Full-bleed 3D Viewer Stage */}
							<div className="absolute inset-0 z-0 size-full">
								<SkinViewer3D
									skinUrl={uploadedDataUrl || DEFAULT_STEVE_SKIN}
									username={targetAccount?.username || "CustomSkin"}
									showToolbar={false}
									model={uploadedModel}
									autoResize={true}
									borderless={true}
									zoom={0.56}
									floatingControlsClassName="top-4 right-3"
									className="size-full"
								/>
							</div>

							{/* Top Showcase Header (Floating Overlay) */}
							<div className="pointer-events-none relative z-10 flex shrink-0 items-center justify-between bg-gradient-to-b from-zinc-950/90 via-zinc-950/50 to-transparent p-4 lg:p-5">
								<span className="font-semibold text-xs text-zinc-200">
									{t("skinsPage.livePreview")}
								</span>
								<span className="pointer-events-auto rounded-full border border-zinc-800/80 bg-zinc-900/90 px-2.5 py-0.5 font-medium text-[10px] text-zinc-300 backdrop-blur-xs">
									{uploadedModel === "default"
										? t("skinsPage.steveClassic4")
										: t("skinsPage.alexSlim3")}
								</span>
							</div>

							{/* Transparent Interaction Spacer */}
							<div className="pointer-events-none min-h-0 flex-1" />

							{/* Primary Apply Button (Floating Overlay at Bottom of 3D stage) */}
							<div className="pointer-events-none relative z-10 flex shrink-0 flex-col gap-2 bg-gradient-to-t from-zinc-950/95 via-zinc-950/60 to-transparent p-4 lg:p-5">
								<Button
									size="default"
									className="pointer-events-auto h-11 w-full gap-2 rounded-xl bg-gradient-to-r from-emerald-600 to-emerald-500 font-semibold text-sm text-white shadow-emerald-950/50 shadow-lg transition-all hover:from-emerald-500 hover:to-emerald-400 hover:shadow-emerald-900/60 active:scale-[0.99] disabled:opacity-50"
									disabled={isApplying || !targetAccount || !uploadedDataUrl}
									onClick={handleUploadSkin}
								>
									{isApplying ? (
										<>
											<Loader2 className="size-4 animate-spin" />
											{t("skinsPage.uploading")}
										</>
									) : (
										<>
											<Upload className="size-4" />
											{t("skinsPage.uploadApply")}
										</>
									)}
								</Button>
							</div>
						</div>

						{/* Right Content Panel: Upload & Configuration */}
						<ScrollArea scrollFade className="flex-1" viewportClassName="p-5 lg:p-8">
							<div className="mx-auto flex w-full max-w-xl flex-col gap-5">
								<div>
									<h2 className="font-semibold text-base text-zinc-100">
										{t("skinsPage.uploadTitle")}
									</h2>
									<p className="mt-0.5 text-xs text-zinc-400">{t("skinsPage.uploadDescription")}</p>
								</div>

								{/* Drag & Drop Upload Zone */}
								<label
									onDragOver={handleDragOver}
									onDragLeave={handleDragLeave}
									onDrop={handleDrop}
									className={cn(
										"relative flex cursor-pointer flex-col items-center justify-center rounded-2xl border-2 border-dashed p-6 text-center transition-all sm:p-8",
										isDragging
											? "border-emerald-500 bg-emerald-500/10"
											: uploadedDataUrl
												? "border-emerald-500/40 bg-zinc-900/40 hover:border-emerald-500/60"
												: "border-zinc-800 bg-zinc-900/20 hover:border-zinc-700 hover:bg-zinc-900/40",
									)}
								>
									<input
										ref={fileInputRef}
										type="file"
										accept="image/png"
										className="hidden"
										onChange={(e) => {
											if (e.target.files?.[0]) {
												handleFileSelected(e.target.files[0])
											}
										}}
									/>

									<div className="mb-2.5 flex size-11 items-center justify-center rounded-full border border-zinc-800 bg-zinc-900 text-zinc-300">
										<Upload className="size-5 text-emerald-400" />
									</div>
									<p className="font-medium text-xs text-zinc-200 sm:text-sm">
										{uploadedFileName || t("skinsPage.dropHint")}
									</p>
									<p className="mt-1 text-[11px] text-zinc-500">{t("skinsPage.supports")}</p>

									{uploadValidationDetails && (
										<div className="mt-3 flex items-center gap-2.5 rounded-md border border-emerald-500/30 bg-emerald-500/10 px-3 py-1 font-mono text-[11px] text-emerald-300">
											<span>
												{t("skinsPage.dimensions", { value: uploadValidationDetails.dimensions })}
											</span>
											<span>•</span>
											<span>{t("skinsPage.size", { value: uploadValidationDetails.sizeKb })}</span>
										</div>
									)}
								</label>

								{uploadError && (
									<div className="flex items-center gap-2 rounded-lg border border-rose-500/20 bg-rose-500/10 p-3 text-rose-300 text-xs">
										<AlertCircle className="size-4 shrink-0 text-rose-400" />
										<span>{uploadError}</span>
									</div>
								)}

								{/* Skin Arm Model Choice */}
								<div className="flex flex-col gap-2 rounded-xl border border-zinc-800/80 bg-zinc-900/30 p-4">
									<div>
										<span className="font-medium text-xs text-zinc-300">
											{t("skinsPage.armModel")}
										</span>
										<p className="mt-0.5 text-[11px] text-zinc-500">
											{t("skinsPage.armModelHint")}
										</p>
									</div>
									<div className="grid grid-cols-2 gap-2.5 pt-1">
										<button
											type="button"
											className={cn(
												"flex flex-col rounded-lg border p-3 text-left transition-all",
												uploadedModel === "default"
													? "border-emerald-500/60 bg-emerald-500/10 text-zinc-100 ring-1 ring-emerald-500/30"
													: "border-zinc-800 bg-zinc-900/60 text-zinc-400 hover:border-zinc-700",
											)}
											onClick={() => setUploadedModel("default")}
										>
											<span className="font-semibold text-xs">Steve</span>
											<span className="text-[10px] text-zinc-500">
												{t("skinsPage.classicArms")}
											</span>
										</button>
										<button
											type="button"
											className={cn(
												"flex flex-col rounded-lg border p-3 text-left transition-all",
												uploadedModel === "slim"
													? "border-emerald-500/60 bg-emerald-500/10 text-zinc-100 ring-1 ring-emerald-500/30"
													: "border-zinc-800 bg-zinc-900/60 text-zinc-400 hover:border-zinc-700",
											)}
											onClick={() => setUploadedModel("slim")}
										>
											<span className="font-semibold text-xs">Alex</span>
											<span className="text-[10px] text-zinc-500">{t("skinsPage.slimArms")}</span>
										</button>
									</div>
								</div>

								{/* Target Ely.by Account */}
								<div className="flex items-center justify-between rounded-xl border border-zinc-800/80 bg-zinc-900/30 p-3.5 text-xs">
									<div>
										<div className="text-[11px] text-zinc-400">{t("skinsPage.targetAccount")}</div>
										<div className="mt-0.5 font-semibold text-xs text-zinc-200">
											{targetAccount ? targetAccount.username : t("skinsPage.noAccount")}
										</div>
									</div>
									{targetAccount && (
										<span className="rounded-full bg-emerald-500/10 px-2 py-0.5 font-medium text-[10px] text-emerald-400">
											{t("skinsPage.readyToApply")}
										</span>
									)}
								</div>
							</div>
						</ScrollArea>
					</div>
				)}
			</div>

			{/* Password Authentication Modal */}
			<Dialog open={showPasswordDialog} onOpenChange={setShowPasswordDialog}>
				<DialogContent className="max-w-md border-zinc-800 bg-zinc-950 p-6 text-zinc-100">
					<DialogHeader>
						<div className="mb-2 flex size-10 items-center justify-center rounded-full border border-zinc-800 bg-zinc-900">
							<KeyRound className="size-5 text-emerald-400" />
						</div>
						<DialogTitle className="text-base text-zinc-100">
							{t("skinsPage.passwordTitle")}
						</DialogTitle>
						<DialogDescription className="text-xs text-zinc-400">
							<Trans
								i18nKey="skinsPage.passwordDescription"
								values={{ name: targetAccount?.username ?? "" }}
								components={{ b: <strong /> }}
							/>
						</DialogDescription>
					</DialogHeader>

					<div className="space-y-3 py-2">
						<div className="relative">
							<Lock className="absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-zinc-500" />
							<Input
								type="password"
								placeholder={t("skinsPage.passwordPlaceholder")}
								value={passwordInput}
								onChange={(e) => setPasswordInput(e.target.value)}
								onKeyDown={(e) => {
									if (e.key === "Enter") {
										handleProcessPendingAction()
									}
								}}
								className="border-zinc-800 bg-zinc-900/80 pl-8 text-xs text-zinc-100"
							/>
						</div>
						{passwordError && <p className="text-rose-400 text-xs">{passwordError}</p>}
					</div>

					<DialogFooter className="gap-2 sm:gap-0">
						<Button
							variant="ghost"
							size="sm"
							onClick={() => setShowPasswordDialog(false)}
							className="text-xs text-zinc-400 hover:text-zinc-200"
						>
							{t("common.cancel")}
						</Button>
						<Button
							size="sm"
							onClick={handleProcessPendingAction}
							disabled={isApplying || !passwordInput.trim()}
							className="bg-emerald-600 text-white text-xs hover:bg-emerald-500"
						>
							{isApplying ? (
								<Loader2 className="size-3.5 animate-spin" />
							) : (
								t("skinsPage.authorize")
							)}
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
		</div>
	)
}

SkinCatalogView.displayName = "SkinCatalogView"

export default memo(SkinCatalogView)
