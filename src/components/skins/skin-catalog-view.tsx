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
import { toast } from "sonner"
import SkinViewer3D, { DEFAULT_STEVE_SKIN } from "@/components/accounts/skin-viewer-3d"
import { alertTone } from "@/components/common/alert-tones"
import { ScrollArea } from "@/components/common/scroll-area"
import SkinAvatar from "@/components/common/skin-avatar"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Badge } from "@/components/ui/badge"
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
import {
	Field,
	FieldContent,
	FieldDescription,
	FieldLabel,
	FieldTitle,
} from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import {
	InputGroup,
	InputGroupAddon,
	InputGroupButton,
	InputGroupInput,
} from "@/components/ui/input-group"
import { Label } from "@/components/ui/label"
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group"
import { Spinner } from "@/components/ui/spinner"
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
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
	accountType?: string
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
				// Uploads on Ely.by belong to Ely.by accounts only; another account type with the
				// same name would otherwise list a stranger's skins
				if (params.accountType === "ely" && params.accountUsername) {
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
			accountType: targetAccount?.accountType,
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

	// Microsoft accounts change skins through Minecraft services: no password prompt needed
	const runMicrosoftSkinChange = async (change: () => Promise<void>, successText: string) => {
		setIsApplying(true)
		try {
			await change()
			toast.success(successText)
			await queryClient.invalidateQueries({ queryKey: ["skins"] })
		} catch (err: unknown) {
			const msg = err instanceof Error ? err.message : String(err)
			toast.error(msg || t("skinsPage.applyFailed"))
		} finally {
			setIsApplying(false)
		}
	}

	// Handle applying selected skin to the Ely.by or Microsoft account
	const handleApplySkin = async (skin: ElySkinItem) => {
		if (!targetAccount) {
			toast.error(t("skinsPage.addAccountFirst"))
			return
		}

		if (targetAccount.accountType === "microsoft") {
			const { id: accountId, username: name } = targetAccount
			const { dataUrl } = skin
			await runMicrosoftSkinChange(
				dataUrl
					? () => accountService.uploadMicrosoftSkin(accountId, dataUrl, skin.isSlim)
					: () => accountService.applyMicrosoftSkin(accountId, skin.skinUrl, skin.isSlim),
				dataUrl
					? t("skinsPage.customApplied", { name })
					: t("skinsPage.skinApplied", { id: skin.id, name }),
			)
			return
		}

		if (targetAccount.accountType !== "ely") {
			toast.error(t("skinsPage.offlineNoSync"))
			return
		}

		setIsApplying(true)

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
				toast.success(t("skinsPage.customApplied", { name: targetAccount.username }))
			} else if (skin.id > 0) {
				await accountService.applyElySkin(targetAccount.id, skin.id)
				toast.success(t("skinsPage.skinApplied", { id: skin.id, name: targetAccount.username }))
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
				toast.error(msg || t("skinsPage.applyFailed"))
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
				toast.success(
					t("skinsPage.skinApplied", {
						id: pendingAction.skinId,
						name: targetAccount.username,
					}),
				)
			} else if (pendingAction.type === "upload") {
				skinStorageService.saveUploadedSkin(targetAccount.id, {
					id: `custom_${Date.now()}`,
					name: uploadedFileName || t("skinsPage.skinFromDate", { date: formatDate(Date.now()) }),
					dataUrl: pendingAction.dataUrl,
					isSlim: uploadedModel === "slim",
					uploadedAt: Date.now(),
				})
				await accountService.uploadElySkin(targetAccount.id, pendingAction.dataUrl, passwordInput)
				toast.success(t("skinsPage.uploadedApplied", { name: targetAccount.username }))
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

		if (targetAccount.accountType === "microsoft") {
			const { id: accountId, username: name } = targetAccount
			const isSlim = uploadedModel === "slim"
			await runMicrosoftSkinChange(
				async () => {
					await accountService.uploadMicrosoftSkin(accountId, uploadedDataUrl, isSlim)
					skinStorageService.saveUploadedSkin(accountId, {
						id: `custom_${Date.now()}`,
						name: uploadedFileName || t("skinsPage.skinFromDate", { date: formatDate(Date.now()) }),
						dataUrl: uploadedDataUrl,
						isSlim,
						uploadedAt: Date.now(),
					})
					handleTabChange("my-skins")
				},
				t("skinsPage.uploadedApplied", { name }),
			)
			return
		}

		if (targetAccount.accountType !== "ely") {
			toast.error(t("skinsPage.offlineNoUpload"))
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
			toast.success(t("skinsPage.uploadedApplied", { name: targetAccount.username }))
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
				toast.error(msg || t("skinsPage.uploadFailed"))
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
			toast.success(t("skinsPage.saved", { id: skin.id }))
		} catch (err: unknown) {
			toast.error(err instanceof Error ? err.message : t("skinsPage.downloadFailed"))
		}
	}

	const handleCopyUrl = (url: string) => {
		navigator.clipboard.writeText(url)
		toast.success(t("skinsPage.urlCopied"))
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
						<div className="flex items-center gap-2 rounded-lg border border-border/40 bg-card/60 px-2.5 py-1.5 text-foreground/80 text-xs">
							<SkinAvatar
								username={targetAccount.username}
								skinUrl={targetAccount.skinUrl}
								size={20}
							/>
							<span className="max-w-[120px] truncate font-medium">{targetAccount.username}</span>
							<span
								className="size-1.5 rounded-full bg-primary"
								title={t("skinsPage.activeAccount")}
							/>
						</div>
					) : (
						<Alert className={alertTone.warning}>
							<AlertCircle />
							<AlertDescription>{t("skinsPage.noActiveAccount")}</AlertDescription>
						</Alert>
					)}

					{/* Tab Buttons */}
					<Tabs
						value={activeTab}
						onValueChange={(tab) => handleTabChange(tab as SkinsQueryParams["tab"])}
					>
						<TabsList>
							<TabsTrigger value="catalog">
								<Search />
								{t("skins.tabs.catalog")}
							</TabsTrigger>
							<TabsTrigger value="my-skins">
								<User />
								{t("skins.tabs.mySkins")}
							</TabsTrigger>
							<TabsTrigger value="upload">
								<Upload />
								{t("skins.tabs.upload")}
							</TabsTrigger>
						</TabsList>
					</Tabs>
				</div>
			</div>

			{/* Main Studio View Card */}
			<div className="flex size-full min-h-0 flex-1 flex-col overflow-hidden rounded-2xl border border-border/40 bg-card/40 shadow-2xl backdrop-blur-md">
				{/* Tab 1 & 2: Catalog & My Uploads */}
				{activeTab !== "upload" && (
					<div className="flex size-full min-h-0 flex-1 flex-col overflow-hidden md:flex-row">
						{/* Mobile Sub-Navigation Toggle: Browse vs 3D Preview (Visible only on < md) */}
						<Tabs
							value={mobileView}
							onValueChange={(view) => setMobileView(view as typeof mobileView)}
							className="shrink-0 border-border/30 border-b p-1.5 md:hidden"
						>
							<TabsList className="w-full">
								<TabsTrigger value="catalog">
									<Grid />
									{activeTab === "my-skins" ? t("skins.tabs.mySkins") : t("skinsPage.browse")}{" "}
									{totalItems > 0 ? `(${formatNumber(totalItems)})` : ""}
								</TabsTrigger>
								<TabsTrigger value="preview">
									<Sparkles className="text-primary" />
									{t("skinsPage.preview3d")}
								</TabsTrigger>
							</TabsList>
						</Tabs>

						{/* Left Showcase: Character Studio & Actions */}
						<div
							className={cn(
								"relative flex h-full shrink-0 flex-col justify-between overflow-hidden border-border/30 bg-gradient-to-b from-background via-card/60 to-background md:w-[320px] md:border-r md:border-b-0 lg:w-[350px]",
								mobileView === "preview" ? "flex w-full" : "hidden md:flex",
							)}
						>
							{/* Background Studio Ambience */}
							<div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_50%_40%,color-mix(in_oklab,var(--primary)_8%,transparent),transparent_70%)]" />
							<div className="pointer-events-none absolute bottom-24 left-1/2 h-10 w-48 -translate-x-1/2 rounded-[100%] bg-primary/10 blur-md" />

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
							<div className="pointer-events-none relative z-10 flex shrink-0 items-center justify-between bg-gradient-to-b from-background/90 via-background/50 to-transparent p-4 lg:p-5">
								<div className="pointer-events-auto flex items-center gap-1.5">
									<Button
										variant="ghost"
										size="xs"
										className="h-6 px-1.5 text-muted-foreground text-xs md:hidden"
										onClick={() => setMobileView("catalog")}
									>
										<ChevronLeft className="mr-0.5 size-3.5" />
										{t("skins.tabs.catalog")}
									</Button>
									<span className="font-semibold text-foreground text-xs">
										{activeSkin.name
											? activeSkin.name
											: activeSkin.id === 0
												? t("skinsPage.currentSkin")
												: t("skinsPage.skinNumber", { id: activeSkin.id })}
									</span>
								</div>
								<Badge variant="secondary" className="pointer-events-auto backdrop-blur-xs">
									{activeSkin.isSlim ? t("skinsPage.alexSlim3") : t("skinsPage.steveClassic4")}
								</Badge>
							</div>

							{/* Transparent Interaction Spacer - lets clicks fall through to 3D canvas */}
							<div className="pointer-events-none min-h-0 flex-1" />

							{/* Primary Apply Button & Secondary Actions (Floating Overlay) */}
							<div className="pointer-events-none relative z-10 flex shrink-0 flex-col gap-2 bg-gradient-to-t from-background/95 via-background/60 to-transparent p-4 lg:p-5">
								<Button
									size="default"
									className="pointer-events-auto h-11 w-full gap-2 rounded-xl font-semibold text-sm transition-all active:scale-[0.99] disabled:opacity-50"
									disabled={
										isApplying || !targetAccount || (activeSkin.id === 0 && !activeSkin.dataUrl)
									}
									onClick={() =>
										(activeSkin.id !== 0 || !!activeSkin.dataUrl) && handleApplySkin(activeSkin)
									}
								>
									{isApplying ? <Spinner className="size-4" /> : <Sparkles className="size-4" />}
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
										className="h-8 gap-1.5 rounded-lg text-xs backdrop-blur-xs"
										onClick={() => handleDownloadSkin(activeSkin)}
									>
										<Download className="size-3.5" />
										{t("skinsPage.downloadPng")}
									</Button>
									<Button
										variant="outline"
										size="xs"
										className="h-8 gap-1.5 rounded-lg text-xs backdrop-blur-xs"
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
							<div className="flex shrink-0 flex-wrap items-center justify-between gap-3 border-border/30 border-b bg-background/40 px-4 py-2.5">
								{/* Search Bar */}
								<InputGroup className="min-w-[180px] max-w-xs flex-1">
									<InputGroupAddon>
										<Search />
									</InputGroupAddon>
									<InputGroupInput
										type="text"
										placeholder={t("skins.searchPlaceholder")}
										value={searchInput}
										onChange={(e) => setSearchInput(e.target.value)}
										className="text-xs"
									/>
									{searchInput && (
										<InputGroupAddon align="inline-end">
											<InputGroupButton
												size="icon-xs"
												aria-label={t("common.clearSearch")}
												onClick={() => {
													setSearchInput("")
													navigate({
														search: (prev) => ({ ...prev, q: "", page: 1 }),
														replace: true,
													})
												}}
											>
												<X />
											</InputGroupButton>
										</InputGroupAddon>
									)}
								</InputGroup>

								{/* Filter Controls: Model & Sort */}
								<div className="flex items-center gap-2">
									{/* Model Filter */}
									<ToggleGroup
										variant="outline"
										size="sm"
										spacing={0}
										value={[modelFilter]}
										onValueChange={(value) => {
											const model = MODEL_OPTIONS.find((m) => m === value[0])
											if (model) navigate({ search: (prev) => ({ ...prev, model, page: 1 }) })
										}}
									>
										{MODEL_OPTIONS.map((m) => (
											<ToggleGroupItem key={m} value={m}>
												{t(`skins.models.${m}`)}
											</ToggleGroupItem>
										))}
									</ToggleGroup>

									{/* Sort Dropdown */}
									<DropdownMenu>
										<DropdownMenuTrigger
											render={
												<Button
													variant="outline"
													size="xs"
													className="h-8 gap-1.5 rounded-lg text-2xs"
												>
													<ArrowUpDown className="size-3 text-muted-foreground" />
													<span>{t(`skins.sorts.${sortOption}`)}</span>
													<ChevronDown className="size-3 opacity-60" />
												</Button>
											}
										/>
										<DropdownMenuContent
											align="end"
											className="border-border bg-background p-1 text-foreground"
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
														"flex cursor-pointer items-center justify-between gap-2 rounded-md px-2 py-1.5 text-xs hover:bg-card",
														sortOption === s.id && "bg-card font-medium text-primary",
													)}
												>
													<span>{t(`skins.sorts.${s.id}`)}</span>
													{sortOption === s.id && <Check className="size-3 text-primary" />}
												</DropdownMenuItem>
											))}
										</DropdownMenuContent>
									</DropdownMenu>

									<Tooltip>
										<TooltipTrigger
											render={
												<Button
													variant="outline"
													size="xs"
													onClick={() => refetch()}
													disabled={isFetching}
													className="h-8 gap-1.5 rounded-lg px-2 text-2xs"
												/>
											}
										>
											<RefreshCw
												className={cn(
													"size-3 text-muted-foreground",
													isFetching && "animate-spin text-primary",
												)}
											/>
											<span className="hidden sm:inline">{t("common.refresh")}</span>
										</TooltipTrigger>
										<TooltipContent>{t("common.refresh")}</TooltipContent>
									</Tooltip>

									{totalItems > 0 && (
										<span className="hidden font-mono text-2xs text-muted-foreground xl:inline">
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
									<div className="absolute top-2 right-3 z-10 flex items-center gap-1.5 rounded-full border border-input/80 bg-card/95 px-2.5 py-1 text-2xs text-foreground/80 shadow-md backdrop-blur-xs">
										<Spinner className="size-3 text-primary" />
										<span>{t("skinsPage.updating")}</span>
									</div>
								)}

								<ScrollArea className="flex-1">
									<div className="min-h-full p-4">
										{isLoading && skins.length === 0 ? (
											<div className="flex h-full min-h-[300px] flex-col items-center justify-center gap-2 text-muted-foreground">
												<Spinner className="size-6 text-primary" />
												<span className="text-xs">
													{activeTab === "my-skins"
														? t("skinsPage.loadingMine")
														: t("skinsPage.loading")}
												</span>
											</div>
										) : catalogError ? (
											<div className="flex h-full min-h-[300px] flex-col items-center justify-center gap-3 p-4 text-center">
												<AlertCircle className="size-8 text-destructive" />
												<p className="max-w-sm text-destructive text-xs">{catalogError}</p>
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
											<div className="flex h-full min-h-[300px] flex-col items-center justify-center gap-3 text-center text-muted-foreground">
												{activeTab === "my-skins" ? (
													<>
														<div className="flex size-12 items-center justify-center rounded-full border border-border bg-card/80">
															<User className="size-6 text-muted-foreground" />
														</div>
														<p className="font-medium text-foreground/80 text-xs">
															{searchQuery
																? t("skinsPage.noMatchingUploads")
																: targetAccount
																	? t("skinsPage.noUploadsFor", { name: targetAccount.username })
																	: t("skinsPage.noActiveAccount")}
														</p>
														<p className="max-w-xs text-2xs text-muted-foreground">
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
																className="mt-1 h-8 gap-1.5 rounded-lg px-3 text-xs"
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
														<p className="font-medium text-muted-foreground text-xs">
															{t("skinsPage.noSkins")}
														</p>
														<p className="text-2xs text-muted-foreground">
															{t("skinsPage.tryDifferent")}
														</p>
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
																	? "border-primary/80 bg-primary/10 ring-1 ring-primary/60"
																	: "border-border/80 bg-card/50 hover:border-input/80 hover:bg-card/60",
															)}
														>
															{/* Active Selection Badge */}
															{isSelected && (
																<div className="absolute top-2 right-2 z-10 flex size-4 items-center justify-center rounded-full bg-primary text-background shadow-xs">
																	<Check className="size-2.5 stroke-[3]" />
																</div>
															)}

															{/* Custom Skin Remove Button */}
															{skin.isCustom && (
																<Tooltip>
																	<TooltipTrigger
																		render={
																			<Button
																				variant="ghost"
																				size="icon-xs"
																				onClick={(e) => {
																					e.stopPropagation()
																					handleRemoveCustomSkin(skin, e)
																				}}
																				className="absolute top-2 right-2 z-10 bg-card/90 text-muted-foreground any-pointer-coarse:opacity-100 opacity-0 hover:bg-destructive/20 hover:text-destructive group-hover:opacity-100"
																				aria-label={t("skinsPage.deleteCustom")}
																			/>
																		}
																	>
																		<Trash2 className="size-3" />
																	</TooltipTrigger>
																	<TooltipContent>{t("skinsPage.deleteCustom")}</TooltipContent>
																</Tooltip>
															)}

															{/* 2D Skin Body Preview */}
															<div className="flex h-36 w-full items-center justify-center overflow-hidden rounded-lg bg-background/60 p-2">
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
																	className="truncate font-medium text-foreground text-xs group-hover:text-primary"
																	title={title}
																>
																	{title}
																</span>
																<div className="flex items-center justify-between text-3xs text-muted-foreground">
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
									</div>
								</ScrollArea>

								{/* Pagination Bar */}
								<div className="flex shrink-0 flex-col gap-2 border-border/30 border-t bg-background/40 px-4 py-2 sm:flex-row sm:items-center sm:justify-between">
									<span className="text-2xs text-muted-foreground">
										<Trans
											i18nKey="skinsPage.pageOf"
											values={{ page, total: lastPage }}
											components={{ b: <strong className="text-foreground" /> }}
										/>
										{totalItems > 0 && (
											<span className="ml-1 text-muted-foreground">
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
								<div className="flex shrink-0 items-center justify-between gap-3 border-border/30 border-t bg-background/95 px-3 py-2 backdrop-blur-md md:hidden">
									<div className="flex min-w-0 items-center gap-2">
										<SkinAvatar
											username={`skin_${activeSkin.id}`}
											skinUrl={activeSkin.skinUrl}
											size={26}
										/>
										<div className="truncate">
											<p className="truncate font-semibold text-foreground text-xs">
												{activeSkin.name
													? activeSkin.name
													: activeSkin.id === 0
														? t("skinsPage.currentSkin")
														: t("skinsPage.skinNumber", { id: activeSkin.id })}
											</p>
											<span className="text-3xs text-muted-foreground">
												{activeSkin.isSlim ? t("skinsPage.alexSlim") : t("skinsPage.steveClassic")}
											</span>
										</div>
									</div>
									<div className="flex shrink-0 items-center gap-1.5">
										<Button
											variant="outline"
											size="xs"
											className="h-7 gap-1 text-xs"
											onClick={() => setMobileView("preview")}
										>
											<Eye className="size-3" />
											{t("skinsPage.view3d")}
										</Button>
										<Button
											size="xs"
											className="h-7 gap-1 font-medium text-xs"
											disabled={
												isApplying || !targetAccount || (activeSkin.id === 0 && !activeSkin.dataUrl)
											}
											onClick={() =>
												(activeSkin.id !== 0 || !!activeSkin.dataUrl) && handleApplySkin(activeSkin)
											}
										>
											{isApplying ? (
												<Spinner className="size-3" />
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
						<div className="relative flex h-full shrink-0 flex-col justify-between overflow-hidden border-border/30 bg-gradient-to-b from-background via-card/60 to-background md:w-[320px] md:border-r md:border-b-0 lg:w-[350px]">
							{/* Background Studio Ambience */}
							<div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_50%_40%,color-mix(in_oklab,var(--primary)_8%,transparent),transparent_70%)]" />
							<div className="pointer-events-none absolute bottom-24 left-1/2 h-10 w-48 -translate-x-1/2 rounded-[100%] bg-primary/10 blur-md" />

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
							<div className="pointer-events-none relative z-10 flex shrink-0 items-center justify-between bg-gradient-to-b from-background/90 via-background/50 to-transparent p-4 lg:p-5">
								<span className="font-semibold text-foreground text-xs">
									{t("skinsPage.livePreview")}
								</span>
								<Badge variant="secondary" className="pointer-events-auto backdrop-blur-xs">
									{uploadedModel === "default"
										? t("skinsPage.steveClassic4")
										: t("skinsPage.alexSlim3")}
								</Badge>
							</div>

							{/* Transparent Interaction Spacer */}
							<div className="pointer-events-none min-h-0 flex-1" />

							{/* Primary Apply Button (Floating Overlay at Bottom of 3D stage) */}
							<div className="pointer-events-none relative z-10 flex shrink-0 flex-col gap-2 bg-gradient-to-t from-background/95 via-background/60 to-transparent p-4 lg:p-5">
								<Button
									size="default"
									className="pointer-events-auto h-11 w-full gap-2 rounded-xl font-semibold text-sm transition-all active:scale-[0.99] disabled:opacity-50"
									disabled={isApplying || !targetAccount || !uploadedDataUrl}
									onClick={handleUploadSkin}
								>
									{isApplying ? (
										<>
											<Spinner className="size-4" />
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
						<ScrollArea scrollFade className="flex-1">
							<div className="p-5 lg:p-8">
								<div className="mx-auto flex w-full max-w-xl flex-col gap-5">
									<div>
										<h2 className="font-semibold text-base text-foreground">
											{t("skinsPage.uploadTitle")}
										</h2>
										<p className="mt-0.5 text-muted-foreground text-xs">
											{t("skinsPage.uploadDescription")}
										</p>
									</div>

									{/* Drag & Drop Upload Zone */}
									<Label
										onDragOver={handleDragOver}
										onDragLeave={handleDragLeave}
										onDrop={handleDrop}
										className={cn(
											"relative flex cursor-pointer flex-col items-center justify-center rounded-2xl border-2 border-dashed p-6 text-center transition-all sm:p-8",
											isDragging
												? "border-primary bg-primary/10"
												: uploadedDataUrl
													? "border-primary/40 bg-card/40 hover:border-primary/60"
													: "border-border bg-card/20 hover:border-input hover:bg-card/40",
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

										<div className="mb-2.5 flex size-11 items-center justify-center rounded-full border border-border bg-card text-foreground/80">
											<Upload className="size-5 text-primary" />
										</div>
										<p className="font-medium text-foreground text-xs sm:text-sm">
											{uploadedFileName || t("skinsPage.dropHint")}
										</p>
										<p className="mt-1 text-2xs text-muted-foreground">{t("skinsPage.supports")}</p>

										{uploadValidationDetails && (
											<div className="mt-3 flex items-center gap-2.5 rounded-md border border-primary/30 bg-primary/10 px-3 py-1 font-mono text-2xs text-primary">
												<span>
													{t("skinsPage.dimensions", { value: uploadValidationDetails.dimensions })}
												</span>
												<span>•</span>
												<span>
													{t("skinsPage.size", { value: uploadValidationDetails.sizeKb })}
												</span>
											</div>
										)}
									</Label>

									{uploadError && (
										<Alert variant="destructive">
											<AlertCircle />
											<AlertDescription>{uploadError}</AlertDescription>
										</Alert>
									)}

									{/* Skin Arm Model Choice */}
									<div className="flex flex-col gap-2 rounded-xl border border-border/80 bg-card/30 p-4">
										<div>
											<span className="font-medium text-foreground/80 text-xs">
												{t("skinsPage.armModel")}
											</span>
											<p className="mt-0.5 text-2xs text-muted-foreground">
												{t("skinsPage.armModelHint")}
											</p>
										</div>
										<RadioGroup
											value={uploadedModel}
											onValueChange={(model) => setUploadedModel(model as typeof uploadedModel)}
											className="grid-cols-2 pt-1"
										>
											<FieldLabel htmlFor="skin-model-default">
												<Field orientation="horizontal">
													<FieldContent>
														<FieldTitle>Steve</FieldTitle>
														<FieldDescription>{t("skinsPage.classicArms")}</FieldDescription>
													</FieldContent>
													<RadioGroupItem value="default" id="skin-model-default" />
												</Field>
											</FieldLabel>
											<FieldLabel htmlFor="skin-model-slim">
												<Field orientation="horizontal">
													<FieldContent>
														<FieldTitle>Alex</FieldTitle>
														<FieldDescription>{t("skinsPage.slimArms")}</FieldDescription>
													</FieldContent>
													<RadioGroupItem value="slim" id="skin-model-slim" />
												</Field>
											</FieldLabel>
										</RadioGroup>
									</div>

									{/* Target Ely.by Account */}
									<div className="flex items-center justify-between rounded-xl border border-border/80 bg-card/30 p-3.5 text-xs">
										<div>
											<div className="text-2xs text-muted-foreground">
												{t("skinsPage.targetAccount")}
											</div>
											<div className="mt-0.5 font-semibold text-foreground text-xs">
												{targetAccount ? targetAccount.username : t("skinsPage.noAccount")}
											</div>
										</div>
										{targetAccount && (
											<Badge variant="outline" className="border-primary/30 text-primary">
												{t("skinsPage.readyToApply")}
											</Badge>
										)}
									</div>
								</div>
							</div>
						</ScrollArea>
					</div>
				)}
			</div>

			{/* Password Authentication Modal */}
			<Dialog open={showPasswordDialog} onOpenChange={setShowPasswordDialog}>
				<DialogContent className="max-w-md p-6">
					<DialogHeader>
						<div className="mb-2 flex size-10 items-center justify-center rounded-full border border-border bg-card">
							<KeyRound className="size-5 text-primary" />
						</div>
						<DialogTitle className="text-base text-foreground">
							{t("skinsPage.passwordTitle")}
						</DialogTitle>
						<DialogDescription className="text-muted-foreground text-xs">
							<Trans
								i18nKey="skinsPage.passwordDescription"
								values={{ name: targetAccount?.username ?? "" }}
								components={{ b: <strong /> }}
							/>
						</DialogDescription>
					</DialogHeader>

					<div className="space-y-3 py-2">
						<div className="relative">
							<Lock className="absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-muted-foreground" />
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
								className="pl-8 text-xs"
							/>
						</div>
						{passwordError && <p className="text-destructive text-xs">{passwordError}</p>}
					</div>

					<DialogFooter className="gap-2 sm:gap-0">
						<Button
							variant="ghost"
							size="sm"
							onClick={() => setShowPasswordDialog(false)}
							className="text-muted-foreground text-xs"
						>
							{t("common.cancel")}
						</Button>
						<Button
							size="sm"
							onClick={handleProcessPendingAction}
							disabled={isApplying || !passwordInput.trim()}
							className="text-xs"
						>
							{isApplying ? <Spinner className="size-3.5" /> : t("skinsPage.authorize")}
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
		</div>
	)
}

SkinCatalogView.displayName = "SkinCatalogView"

export default memo(SkinCatalogView)
