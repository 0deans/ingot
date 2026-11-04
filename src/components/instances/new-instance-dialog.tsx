import { Folder, ImagePlus, RotateCcw } from "lucide-react"
import { memo, useEffect, useMemo, useRef, useState } from "react"
import { useTranslation } from "react-i18next"
import { toast } from "sonner"
import type { ModLoaderType, VersionManifestEntry } from "@/bindings"
import SearchableSelect from "@/components/common/searchable-select"
import InstanceIcon, {
	COLOR_TAGS,
	getColorTagDefinition,
	type InstanceColorTag,
	PRESET_ICONS,
	processUploadedIconFile,
} from "@/components/instances/instance-icon"
import LoaderIcon from "@/components/instances/loader-icon"
import { Button } from "@/components/ui/button"
import { Checkbox } from "@/components/ui/checkbox"
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Spinner } from "@/components/ui/spinner"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
import { instanceService } from "@/services/instance-service"

export interface NewInstanceCustomization {
	group: string | null
	colorTag: InstanceColorTag | null
	icon: string | null
}

interface NewInstanceDialogProps {
	open: boolean
	onOpenChange: (open: boolean) => void
	onCreateInstance: (
		name: string,
		gameVersion: string,
		loader: ModLoaderType,
		loaderVersion: string | null,
		customization?: NewInstanceCustomization,
	) => Promise<void> | void
	initialVersion?: string
	existingGroups?: string[]
}

const LOADERS: { id: ModLoaderType; label: string; badge?: string }[] = [
	{ id: "vanilla", label: "Vanilla" },
	{ id: "fabric", label: "Fabric", badge: "Popular" },
	{ id: "quilt", label: "Quilt" },
	{ id: "neoforge", label: "NeoForge" },
	{ id: "forge", label: "Forge" },
	{ id: "horizon", label: "Horizon", badge: "Rust" },
]

const NewInstanceDialog = ({
	open,
	onOpenChange,
	onCreateInstance,
	initialVersion,
	existingGroups = [],
}: NewInstanceDialogProps) => {
	const { t } = useTranslation()
	const iconFileInputRef = useRef<HTMLInputElement | null>(null)
	const [instanceName, setInstanceName] = useState("")
	const [gameVersion, setGameVersion] = useState(initialVersion || "")
	const [loader, setLoader] = useState<ModLoaderType>("vanilla")
	const [loaderVersion, setLoaderVersion] = useState<string>("")
	const [showSnapshots, setShowSnapshots] = useState(false)
	const [group, setGroup] = useState("")
	const [colorTag, setColorTag] = useState<InstanceColorTag | null>(null)
	const [iconSpec, setIconSpec] = useState<string | null>(null)

	const handleUploadIconChange = async (e: React.ChangeEvent<HTMLInputElement>) => {
		const file = e.target.files?.[0]
		e.target.value = ""
		if (!file) {
			return
		}
		try {
			const encoded = await processUploadedIconFile(file)
			setIconSpec(encoded)
		} catch (err) {
			toast.error(
				t("instanceSettings.iconUploadFailed", {
					defaultValue: "Failed to process icon image: {{error}}",
					error: err instanceof Error ? err.message : String(err),
				}),
			)
		}
	}

	const [versions, setVersions] = useState<VersionManifestEntry[]>([])
	const [loaderVersions, setLoaderVersions] = useState<string[]>([])
	const [isLoadingVersions, setIsLoadingVersions] = useState(false)
	const [isLoadingLoaderVersions, setIsLoadingLoaderVersions] = useState(false)
	const [isSubmitting, setIsSubmitting] = useState(false)
	const [isCustomName, setIsCustomName] = useState(false)

	const [prevOpen, setPrevOpen] = useState(open)
	if (open && !prevOpen) {
		setPrevOpen(true)
		setIsLoadingVersions(true)
	} else if (!open && prevOpen) {
		setPrevOpen(false)
		setGroup("")
		setColorTag(null)
		setIconSpec(null)
		setIsCustomName(false)
		if (!initialVersion) {
			setGameVersion("")
		}
	}

	// Fetch game versions when dialog opens
	useEffect(() => {
		if (!open) return
		let cancelled = false

		instanceService
			.getAvailableGameVersions()
			.then((list) => {
				if (cancelled) return
				setVersions(list)
				const releases = list.filter((v) => {
					return v.type === "release"
				})
				if (initialVersion) {
					setGameVersion(initialVersion)
					const isSnapshot = list.some((v) => {
						return v.id === initialVersion && v.type === "snapshot"
					})
					if (isSnapshot || /snapshot|pre|rc/i.test(initialVersion)) {
						setShowSnapshots(true)
					}
				} else if (releases.length > 0) {
					setGameVersion((current) => {
						const hasCurrent =
							Boolean(current) &&
							releases.some((r) => {
								return r.id === current
							})
						return hasCurrent ? current : releases[0].id
					})
				}
			})
			.catch((e) => {
				console.error("Failed to load versions:", e)
			})
			.finally(() => {
				if (!cancelled) setIsLoadingVersions(false)
			})

		return () => {
			cancelled = true
		}
	}, [open, initialVersion])

	const [prevLoaderConfig, setPrevLoaderConfig] = useState({ open, loader, gameVersion })
	if (
		open !== prevLoaderConfig.open ||
		loader !== prevLoaderConfig.loader ||
		gameVersion !== prevLoaderConfig.gameVersion
	) {
		setPrevLoaderConfig({ open, loader, gameVersion })
		if (!open || loader === "vanilla") {
			setLoaderVersions([])
			setLoaderVersion("")
			setIsLoadingLoaderVersions(false)
		} else {
			setIsLoadingLoaderVersions(true)
		}
	}

	// Fetch loader versions when gameVersion or loader changes
	useEffect(() => {
		if (!open || loader === "vanilla") {
			return
		}

		let cancelled = false

		instanceService
			.getAvailableLoaderVersions(gameVersion, loader)
			.then((vers) => {
				if (cancelled) return
				setLoaderVersions(vers)
				setLoaderVersion(vers[0] || "")
			})
			.catch((e) => {
				console.error("Failed to fetch loader versions:", e)
			})
			.finally(() => {
				if (!cancelled) setIsLoadingLoaderVersions(false)
			})

		return () => {
			cancelled = true
		}
	}, [open, loader, gameVersion])

	const [prevNameConfig, setPrevNameConfig] = useState({ loader, gameVersion, isCustomName })
	if (
		!isCustomName &&
		(loader !== prevNameConfig.loader ||
			gameVersion !== prevNameConfig.gameVersion ||
			isCustomName !== prevNameConfig.isCustomName)
	) {
		setPrevNameConfig({ loader, gameVersion, isCustomName })
		const loaderLabel =
			LOADERS.find((l) => {
				return l.id === loader
			})?.label || "Vanilla"
		if (gameVersion) {
			setInstanceName(`${loaderLabel} ${gameVersion}`)
		} else {
			setInstanceName(loaderLabel)
		}
	} else if (isCustomName !== prevNameConfig.isCustomName) {
		setPrevNameConfig({ loader, gameVersion, isCustomName })
	}

	const filteredVersions = versions.filter((v) => {
		return showSnapshots ? true : v.type === "release"
	})

	const gameVersionOptions = useMemo(() => {
		const options = filteredVersions.map((v) => {
			return {
				value: v.id,
				label: v.id,
				badge: v.type === "snapshot" ? t("newInstance.snapshotBadge") : undefined,
			}
		})
		if (
			loader === "horizon" &&
			!options.some((o) => {
				return o.value === "26.3"
			})
		) {
			return [
				{
					value: "26.3",
					label: "26.3",
					badge: "Horizon",
				},
				...options,
			]
		}
		return options
	}, [filteredVersions, loader, t])

	const loaderVersionOptions = useMemo(() => {
		return loaderVersions.map((v, i) => {
			return {
				value: v,
				label: v,
				badge: i === 0 ? t("newInstance.latestBadge") : undefined,
			}
		})
	}, [loaderVersions, t])

	const handleCreate = async () => {
		if (!instanceName.trim() || isSubmitting) return
		setIsSubmitting(true)
		try {
			await onCreateInstance(
				instanceName.trim(),
				gameVersion,
				loader,
				loader !== "vanilla" ? loaderVersion || null : null,
				{
					group: group.trim() ? group.trim() : null,
					colorTag,
					icon: iconSpec,
				},
			)
			setInstanceName("")
			setGroup("")
			setColorTag(null)
			setIconSpec(null)
			setIsCustomName(false)
			onOpenChange(false)
		} catch (e) {
			console.error("Failed to create instance:", e)
		} finally {
			setIsSubmitting(false)
		}
	}

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent className="max-h-[90vh] sm:max-w-xl">
				<DialogHeader>
					<DialogTitle className="text-lg font-semibold">{t("newInstance.title")}</DialogTitle>
					<DialogDescription className="text-xs text-muted-foreground">
						{t("newInstance.description")}
					</DialogDescription>
				</DialogHeader>

				<ScrollArea className="-mr-2 max-h-[68vh] scroll-fade-y pr-2">
					<div className="grid gap-4 py-1 pr-1">
						{/* Mod Loader Selector */}
						<div className="grid gap-1.5">
							<div className="text-xs font-medium text-muted-foreground">
								{t("newInstance.modLoader")}
							</div>
							<ToggleGroup
								variant="outline"
								value={[loader]}
								onValueChange={(value) => {
									if (value[0]) {
										const next = value[0] as ModLoaderType
										setLoader(next)
										if (next === "horizon") {
											setGameVersion("26.3")
										}
									}
								}}
								className="grid w-full grid-cols-3 gap-1.5 sm:grid-cols-6 sm:gap-2"
							>
								{LOADERS.map((item) => {
									return (
										<ToggleGroupItem
											key={item.id}
											value={item.id}
											className="h-auto w-full flex-col gap-2 p-2.5 text-xs"
										>
											<LoaderIcon loader={item.id} size={22} />
											<span>{item.label}</span>
										</ToggleGroupItem>
									)
								})}
							</ToggleGroup>
						</div>

						{/* Version Row */}
						<div
							className={
								loader === "vanilla"
									? "grid grid-cols-1 items-start gap-3"
									: "grid grid-cols-2 items-start gap-3"
							}
						>
							{/* Game Version */}
							<div className="flex flex-col gap-1.5">
								<div className="flex h-5 items-center justify-between">
									<Label
										htmlFor="game-version"
										className="text-xs font-medium text-muted-foreground"
									>
										{t("newInstance.minecraftVersion")}
									</Label>
									{loader !== "horizon" && (
										<Label
											htmlFor="snapshots-toggle"
											className="flex cursor-pointer items-center gap-1.5 text-xs font-normal text-muted-foreground transition-colors select-none hover:text-foreground"
										>
											<Checkbox
												id="snapshots-toggle"
												checked={showSnapshots}
												onCheckedChange={(checked) => {
													setShowSnapshots(Boolean(checked))
												}}
											/>
											<span>{t("newInstance.snapshots")}</span>
										</Label>
									)}
								</div>

								<SearchableSelect
									id="game-version"
									value={gameVersion}
									onValueChange={setGameVersion}
									options={gameVersionOptions}
									placeholder={
										isLoadingVersions
											? t("newInstance.loadingVersions")
											: t("newInstance.selectVersion")
									}
									searchPlaceholder={t("newInstance.searchVersion")}
									disabled={isLoadingVersions || loader === "horizon"}
								/>
							</div>

							{/* Loader Version - Only visible when a loader is selected */}
							{loader !== "vanilla" && (
								<div className="flex flex-col gap-1.5">
									<div className="flex h-5 items-center">
										<Label
											htmlFor="loader-version"
											className="text-xs font-medium text-muted-foreground"
										>
											{loader === "horizon"
												? t("newInstance.horizonRelease", { defaultValue: "Horizon Release" })
												: t("newInstance.loaderVersion")}
										</Label>
									</div>

									<SearchableSelect
										id="loader-version"
										value={loaderVersion}
										onValueChange={setLoaderVersion}
										options={loaderVersionOptions}
										placeholder={
											isLoadingLoaderVersions
												? t("newInstance.loadingVersions")
												: loaderVersions.length === 0
													? t("newInstance.noCompatible")
													: t("newInstance.selectLoaderVersion")
										}
										searchPlaceholder={t("newInstance.searchLoaderVersion")}
										disabled={isLoadingLoaderVersions || loaderVersions.length === 0}
									/>
								</div>
							)}
						</div>

						{loader === "horizon" && (
							<p className="-mt-1 text-2xs text-muted-foreground">
								{t("newInstance.horizonVersionNotice", {
									defaultValue: "Horizon Rust engine runs Minecraft 26.3",
								})}
							</p>
						)}

						{/* Instance Name */}
						<div className="grid gap-1.5">
							<Label htmlFor="instance-name" className="text-xs font-medium text-muted-foreground">
								{t("newInstance.name")}
							</Label>
							<Input
								id="instance-name"
								value={instanceName}
								onChange={(e) => {
									setInstanceName(e.target.value)
									setIsCustomName(true)
								}}
								placeholder={t("newInstance.namePlaceholder")}
								onKeyDown={(e) => {
									if (e.key === "Enter") handleCreate()
								}}
								className="text-xs font-medium"
							/>
						</div>

						{/* Optional Group & Color Tag */}
						<div className="grid gap-3 rounded-xl border border-border/70 bg-card/30 p-3.5">
							<div className="grid gap-1.5">
								<Label
									htmlFor="instance-group"
									className="text-xs font-medium text-muted-foreground"
								>
									{t("instanceSettings.groupTitle", { defaultValue: "Group / Folder" })}
								</Label>
								<Input
									id="instance-group"
									value={group}
									onChange={(e) => setGroup(e.target.value)}
									placeholder={t("instanceSettings.groupPlaceholder", {
										defaultValue: "e.g. Vanilla, Modpacks, 1.21 SMP",
									})}
									className="h-8 text-xs"
								/>
								{existingGroups.length > 0 && (
									<div className="flex flex-wrap gap-1.5 pt-0.5">
										{existingGroups.slice(0, 6).map((g) => {
											const isCurrent = group.trim() === g
											return (
												<button
													key={g}
													type="button"
													onClick={() => setGroup(isCurrent ? "" : g)}
													className={`inline-flex items-center gap-1 rounded border px-2 py-0.5 text-2xs transition-colors ${
														isCurrent
															? "border-primary bg-primary/10 text-primary"
															: "border-border/70 bg-background/60 text-muted-foreground hover:border-border hover:text-foreground"
													}`}
												>
													<Folder className="size-3 text-amber-400" />
													<span>{g}</span>
												</button>
											)
										})}
									</div>
								)}
							</div>

							<div className="grid gap-1.5 border-t border-border/50 pt-2.5">
								<div className="flex items-center justify-between">
									<Label className="text-xs font-medium text-muted-foreground">
										{t("instanceSettings.colorTagTitle", { defaultValue: "Color Tag" })}
									</Label>
									{colorTag && (
										<button
											type="button"
											onClick={() => setColorTag(null)}
											className="text-2xs text-muted-foreground hover:text-foreground"
										>
											{t("instances.colors.none", { defaultValue: "No Color" })}
										</button>
									)}
								</div>
								<div className="flex flex-wrap items-center gap-2 pt-0.5">
									{COLOR_TAGS.map((tag) => {
										const activeColorDef = getColorTagDefinition(colorTag)
										const isSelected = activeColorDef?.id === tag.id
										return (
											<button
												key={tag.id}
												type="button"
												onClick={() => setColorTag(isSelected ? null : tag.id)}
												title={t(tag.labelKey, { defaultValue: tag.defaultLabel })}
												className={`flex size-6 items-center justify-center rounded-full border transition-all ${
													isSelected
														? `border-foreground ring-2 ${tag.ringClass}`
														: "border-border/70 opacity-75 hover:opacity-100"
												}`}
											>
												<span className={`size-3.5 rounded-full ${tag.dotClass}`} />
											</button>
										)
									})}
									<label
										title={t("instances.colors.custom", { defaultValue: "Custom Color..." })}
										className={`relative inline-flex h-6 cursor-pointer items-center gap-1.5 rounded-full border px-2 text-3xs font-medium transition-all ${
											getColorTagDefinition(colorTag)?.isCustom
												? "border-foreground bg-card text-foreground ring-1 ring-primary/40"
												: "border-border/70 bg-background/60 text-muted-foreground hover:text-foreground"
										}`}
									>
										<input
											type="color"
											value={getColorTagDefinition(colorTag)?.hex ?? "#3b82f6"}
											onChange={(e) => setColorTag(e.target.value)}
											className="absolute inset-0 size-full cursor-pointer opacity-0"
										/>
										<span
											className="size-3 rounded-full border border-border/60"
											style={{
												background: getColorTagDefinition(colorTag)?.isCustom
													? getColorTagDefinition(colorTag)?.hex
													: "conic-gradient(from 0deg, #ef4444, #f59e0b, #10b981, #3b82f6, #8b5cf6, #ec4899, #ef4444)",
											}}
										/>
										<span>
											{getColorTagDefinition(colorTag)?.isCustom
												? getColorTagDefinition(colorTag)?.defaultLabel
												: t("instances.colors.customShort", { defaultValue: "Custom" })}
										</span>
									</label>
								</div>
							</div>

							{/* Optional Icon Picker (Presets + Upload) */}
							<div className="grid gap-2 border-t border-border/50 pt-2.5">
								<div className="flex items-center justify-between gap-2">
									<div className="flex items-center gap-2">
										<div className="flex size-8 shrink-0 items-center justify-center overflow-hidden rounded-lg border border-border bg-background">
											<InstanceIcon icon={iconSpec} loader={loader} size={20} />
										</div>
										<Label className="text-xs font-medium text-muted-foreground">
											{t("instanceSettings.iconTitle", { defaultValue: "Instance Icon" })}
										</Label>
									</div>
									<div className="flex items-center gap-1.5">
										<input
											ref={iconFileInputRef}
											type="file"
											accept="image/png,image/jpeg,image/webp,image/gif"
											onChange={(e) => {
												return void handleUploadIconChange(e)
											}}
											className="hidden"
										/>
										<Button
											type="button"
											variant="outline"
											size="xs"
											onClick={() => iconFileInputRef.current?.click()}
											className="gap-1.5"
										>
											<ImagePlus className="size-3.5 text-primary" />
											{t("instanceSettings.uploadIcon", { defaultValue: "Upload PNG / GIF" })}
										</Button>
										{iconSpec != null && (
											<Button
												type="button"
												variant="ghost"
												size="xs"
												onClick={() => setIconSpec(null)}
												className="gap-1 text-muted-foreground"
											>
												<RotateCcw className="size-3" />
												{t("common.reset", { defaultValue: "Reset" })}
											</Button>
										)}
									</div>
								</div>

								<div className="grid grid-cols-6 gap-1.5 sm:grid-cols-6">
									{PRESET_ICONS.map((preset) => {
										const isSelected = iconSpec === preset.spec
										return (
											<button
												key={preset.id}
												type="button"
												onClick={() => setIconSpec(isSelected ? null : preset.spec)}
												title={t(preset.labelKey, { defaultValue: preset.defaultLabel })}
												className={`flex h-9 items-center justify-center rounded-lg border bg-background/60 transition-all hover:border-primary/60 hover:bg-card ${
													isSelected
														? "border-primary bg-primary/10 ring-1 ring-primary/40"
														: "border-border/60"
												}`}
											>
												<InstanceIcon icon={preset.spec} loader={loader} size={22} />
											</button>
										)
									})}
								</div>
							</div>
						</div>
					</div>
				</ScrollArea>

				<DialogFooter>
					<Button
						onClick={handleCreate}
						disabled={!instanceName.trim() || isSubmitting}
						className="w-full gap-2"
					>
						{isSubmitting && <Spinner className="size-3.5" />}
						{t("quickJoin.createInstance")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	)
}

NewInstanceDialog.displayName = "NewInstanceDialog"

export default memo(NewInstanceDialog)
