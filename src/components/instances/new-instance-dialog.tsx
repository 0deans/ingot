import { Loader2 } from "lucide-react"
import { memo, useEffect, useMemo, useState } from "react"
import type { ModLoaderType, VersionManifestEntry } from "@/bindings"
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
import { ScrollArea } from "@/components/ui/scroll-area"
import SearchableSelect from "@/components/ui/searchable-select"
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
]

const NewInstanceDialog = ({ open, onOpenChange, onCreateInstance }: NewInstanceDialogProps) => {
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
				const releases = list.filter((v) => v.type === "release")
				if (releases.length > 0) {
					setGameVersion((current) =>
						releases.some((r) => r.id === current) ? current : releases[0].id,
					)
				}
			})
			.catch((e) => console.error("Failed to load versions:", e))
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
			.catch((e) => console.error("Failed to fetch loader versions:", e))
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
		const loaderLabel = LOADERS.find((l) => l.id === loader)?.label || "Vanilla"
		setInstanceName(`${loaderLabel} ${gameVersion}`)
	}, [loader, gameVersion, isCustomName])

	const filteredVersions = versions.filter((v) => (showSnapshots ? true : v.type === "release"))

	const gameVersionOptions = useMemo(() => {
		return filteredVersions.map((v) => ({
			value: v.id,
			label: v.id,
			badge: v.type === "snapshot" ? "snapshot" : undefined,
		}))
	}, [filteredVersions])

	const loaderVersionOptions = useMemo(() => {
		return loaderVersions.map((v, i) => ({
			value: v,
			label: v,
			badge: i === 0 ? "latest" : undefined,
		}))
	}, [loaderVersions])

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
					<DialogTitle className="font-semibold text-lg">Create New Instance</DialogTitle>
					<DialogDescription className="text-muted-foreground text-xs">
						Choose your Minecraft version and preferred mod loader. Ingot will configure the
						environment and Java automatically.
					</DialogDescription>
				</DialogHeader>

				<ScrollArea className="-mx-1 max-h-[65vh] px-1">
					<div className="grid gap-4 p-1">
						{/* Mod Loader Selector */}
						<div className="grid gap-1.5">
							<div className="font-medium text-muted-foreground text-xs">Mod Loader</div>
							<div className="grid grid-cols-5 gap-1.5 rounded-lg border border-border/40 bg-zinc-950/40 p-1.5">
								{LOADERS.map((item) => {
									const active = loader === item.id
									return (
										<button
											key={item.id}
											type="button"
											onClick={() => setLoader(item.id)}
											className={`flex flex-col items-center justify-center gap-2 rounded-md p-2.5 font-medium text-xs transition-all ${
												active
													? "bg-primary text-primary-foreground shadow-sm"
													: "text-muted-foreground hover:bg-zinc-800/60 hover:text-foreground"
											}`}
										>
											<LoaderIcon loader={item.id} size={22} />
											<span>{item.label}</span>
										</button>
									)
								})}
							</div>
						</div>

						{/* Version Row */}
						<div
							className={loader === "vanilla" ? "grid grid-cols-1 gap-3" : "grid grid-cols-2 gap-3"}
						>
							{/* Game Version */}
							<div className="grid gap-1.5">
								<div className="flex items-center justify-between">
									<label
										htmlFor="game-version"
										className="font-medium text-muted-foreground text-xs"
									>
										Minecraft Version
									</label>
									<label
										htmlFor="snapshots-toggle"
										className="flex cursor-pointer select-none items-center gap-1.5 font-normal text-muted-foreground text-xs transition-colors hover:text-foreground"
									>
										<Checkbox
											id="snapshots-toggle"
											checked={showSnapshots}
											onCheckedChange={(checked) => setShowSnapshots(Boolean(checked))}
										/>
										<span>Snapshots</span>
									</label>
								</div>

								<SearchableSelect
									id="game-version"
									value={gameVersion}
									onValueChange={setGameVersion}
									options={gameVersionOptions}
									placeholder={
										isLoadingVersions ? "Loading versions..." : "Select Minecraft version"
									}
									searchPlaceholder="Search version (e.g. 1.20, 1.16)..."
									disabled={isLoadingVersions}
								/>
							</div>

							{/* Loader Version - Only visible when a loader is selected */}
							{loader !== "vanilla" && (
								<div className="grid gap-1.5">
									<label
										htmlFor="loader-version"
										className="font-medium text-muted-foreground text-xs"
									>
										Loader Version
									</label>

									<SearchableSelect
										id="loader-version"
										value={loaderVersion}
										onValueChange={setLoaderVersion}
										options={loaderVersionOptions}
										placeholder={
											isLoadingLoaderVersions
												? "Loading versions..."
												: loaderVersions.length === 0
													? "No compatible versions"
													: "Select loader version"
										}
										searchPlaceholder="Search loader version..."
										disabled={isLoadingLoaderVersions || loaderVersions.length === 0}
									/>
								</div>
							)}
						</div>

						{/* Instance Name */}
						<div className="grid gap-1.5">
							<label htmlFor="instance-name" className="font-medium text-muted-foreground text-xs">
								Instance Name
							</label>
							<Input
								id="instance-name"
								value={instanceName}
								onChange={(e) => {
									setInstanceName(e.target.value)
									setIsCustomName(true)
								}}
								placeholder="e.g. Fabric 1.21.4"
								onKeyDown={(e) => {
									if (e.key === "Enter") handleCreate()
								}}
								className="border-border/50 bg-zinc-900/80 font-medium text-xs"
							/>
						</div>
					</div>
				</ScrollArea>

				<DialogFooter className="gap-2">
					<Button
						variant="outline"
						size="sm"
						onClick={() => onOpenChange(false)}
						disabled={isSubmitting}
					>
						Cancel
					</Button>
					<Button
						size="sm"
						onClick={handleCreate}
						disabled={!instanceName.trim() || isSubmitting}
						className="gap-2"
					>
						{isSubmitting && <Loader2 className="size-3.5 animate-spin" />}
						Create Instance
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	)
}

NewInstanceDialog.displayName = "NewInstanceDialog"

export default memo(NewInstanceDialog)
