import { memo, useEffect, useMemo, useState } from "react"
import { useTranslation } from "react-i18next"
import type { ModLoaderType, VersionManifestEntry } from "@/bindings"
import SearchableSelect from "@/components/common/searchable-select"
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

interface NewInstanceDialogProps {
	open: boolean
	onOpenChange: (open: boolean) => void
	onCreateInstance: (
		name: string,
		gameVersion: string,
		loader: ModLoaderType,
		loaderVersion: string | null,
	) => Promise<void> | void
}

const LOADERS: { id: ModLoaderType; label: string; badge?: string }[] = [
	{ id: "vanilla", label: "Vanilla" },
	{ id: "fabric", label: "Fabric", badge: "Popular" },
	{ id: "quilt", label: "Quilt" },
	{ id: "neoforge", label: "NeoForge" },
	{ id: "forge", label: "Forge" },
	{ id: "horizon", label: "Horizon", badge: "Rust" },
]

const NewInstanceDialog = ({ open, onOpenChange, onCreateInstance }: NewInstanceDialogProps) => {
	const { t } = useTranslation()
	const [instanceName, setInstanceName] = useState("")
	const [gameVersion, setGameVersion] = useState("1.21.4")
	const [loader, setLoader] = useState<ModLoaderType>("vanilla")
	const [loaderVersion, setLoaderVersion] = useState<string>("")
	const [showSnapshots, setShowSnapshots] = useState(false)

	const [versions, setVersions] = useState<VersionManifestEntry[]>([])
	const [loaderVersions, setLoaderVersions] = useState<string[]>([])
	const [isLoadingVersions, setIsLoadingVersions] = useState(false)
	const [isLoadingLoaderVersions, setIsLoadingLoaderVersions] = useState(false)
	const [isSubmitting, setIsSubmitting] = useState(false)
	const [isCustomName, setIsCustomName] = useState(false)

	// Fetch game versions when dialog opens
	useEffect(() => {
		if (!open) return
		let cancelled = false
		setIsLoadingVersions(true)

		instanceService
			.getAvailableGameVersions()
			.then((list) => {
				if (cancelled) return
				setVersions(list)
				const releases = list.filter((v) => {
					return v.type === "release"
				})
				if (releases.length > 0) {
					setGameVersion((current) => {
						const hasCurrent = releases.some((r) => {
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
	}, [open])

	// Fetch loader versions when gameVersion or loader changes
	useEffect(() => {
		if (!open || loader === "vanilla") {
			setLoaderVersions([])
			setLoaderVersion("")
			return
		}

		let cancelled = false
		setIsLoadingLoaderVersions(true)

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

	// Auto-generate name if user hasn't entered a custom name
	useEffect(() => {
		if (isCustomName) return
		const loaderLabel =
			LOADERS.find((l) => {
				return l.id === loader
			})?.label || "Vanilla"
		setInstanceName(`${loaderLabel} ${gameVersion}`)
	}, [loader, gameVersion, isCustomName])

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
			)
			setInstanceName("")
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
			<DialogContent className="max-h-[90vh] sm:max-w-lg">
				<DialogHeader>
					<DialogTitle className="text-lg font-semibold">{t("newInstance.title")}</DialogTitle>
					<DialogDescription className="text-xs text-muted-foreground">
						{t("newInstance.description")}
					</DialogDescription>
				</DialogHeader>

				<ScrollArea className="-mx-1 max-h-[65vh] scroll-fade-y px-1">
					<div className="grid gap-4 p-1">
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
