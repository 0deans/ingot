import { open as openDialog } from "@tauri-apps/plugin-dialog"
import {
	ArrowLeft,
	Box,
	Camera,
	Check,
	CheckCircle2,
	ChevronRight,
	Compass,
	Flame,
	FolderOpen,
	FolderSearch,
	Gamepad2,
	HardDrive,
	Layers,
	Loader2,
	Rocket,
	Search,
	Server,
	Sliders,
	Sparkles,
} from "lucide-react"
import { useEffect, useState } from "react"
import type { DetectedLauncher, ImportableInstance } from "@/bindings"
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
import { importerService } from "@/services/importer-service"

interface ImportInstanceDialogProps {
	open: boolean
	onOpenChange: (open: boolean) => void
	onSuccess?: () => void
}

type Step = "launchers" | "instances" | "options"

const LAUNCHER_META: Record<
	string,
	{ icon: typeof Flame; color: string; bg: string; border: string }
> = {
	curseforge: {
		icon: Flame,
		color: "text-amber-400",
		bg: "bg-amber-500/10",
		border: "hover:border-amber-500/40",
	},
	modrinth: {
		icon: Compass,
		color: "text-emerald-400",
		bg: "bg-emerald-500/10",
		border: "hover:border-emerald-500/40",
	},
	atlauncher: {
		icon: Rocket,
		color: "text-sky-400",
		bg: "bg-sky-500/10",
		border: "hover:border-sky-500/40",
	},
	legacylauncher: {
		icon: Gamepad2,
		color: "text-purple-400",
		bg: "bg-purple-500/10",
		border: "hover:border-purple-500/40",
	},
	vanilla: {
		icon: Box,
		color: "text-zinc-300",
		bg: "bg-zinc-500/10",
		border: "hover:border-zinc-500/40",
	},
}

export const ImportInstanceDialog = ({
	open,
	onOpenChange,
	onSuccess,
}: ImportInstanceDialogProps) => {
	const [step, setStep] = useState<Step>("launchers")
	const [launchers, setLaunchers] = useState<DetectedLauncher[]>([])
	const [isLoadingLaunchers, setIsLoadingLaunchers] = useState(false)

	const [selectedLauncher, setSelectedLauncher] = useState<DetectedLauncher | null>(null)
	const [instances, setInstances] = useState<ImportableInstance[]>([])
	const [isLoadingInstances, setIsLoadingInstances] = useState(false)
	const [searchQuery, setSearchQuery] = useState("")

	const [selectedInstance, setSelectedInstance] = useState<ImportableInstance | null>(null)
	const [instanceName, setInstanceName] = useState("")

	// Checkbox options
	const [copyMods, setCopyMods] = useState(true)
	const [copyConfigs, setCopyConfigs] = useState(true)
	const [copySaves, setCopySaves] = useState(true)
	const [copyResourcePacks, setCopyResourcePacks] = useState(true)
	const [copyScreenshots, setCopyScreenshots] = useState(true)
	const [copyOptions, setCopyOptions] = useState(true)
	const [copyServers, setCopyServers] = useState(true)
	const [copyExtraData, setCopyExtraData] = useState(true)

	const [isImporting, setIsImporting] = useState(false)
	const [importStatus, setImportStatus] = useState<string | null>(null)

	// Fetch launchers on open
	useEffect(() => {
		if (open) {
			setStep("launchers")
			setSelectedLauncher(null)
			setSelectedInstance(null)
			setImportStatus(null)
			setIsLoadingLaunchers(true)
			importerService
				.getDetectedLaunchers()
				.then((list) => setLaunchers(list))
				.catch((err) => console.error("Failed to detect launchers:", err))
				.finally(() => setIsLoadingLaunchers(false))
		}
	}, [open])

	const handleSelectLauncher = async (launcher: DetectedLauncher) => {
		setSelectedLauncher(launcher)
		setIsLoadingInstances(true)
		setStep("instances")
		setSearchQuery("")

		try {
			const list = await importerService.getLauncherInstances(launcher.id, null)
			setInstances(list)
		} catch (error) {
			console.error("Failed to load launcher instances:", error)
			setInstances([])
		} finally {
			setIsLoadingInstances(false)
		}
	}

	const handleBrowseLauncherFolder = async (launcher: DetectedLauncher) => {
		try {
			const selected = await openDialog({
				directory: true,
				multiple: false,
				title: `Select ${launcher.name} Instances Directory`,
			})
			if (selected && typeof selected === "string") {
				setSelectedLauncher(launcher)
				setIsLoadingInstances(true)
				setStep("instances")
				const list = await importerService.getLauncherInstances(launcher.id, selected)
				setInstances(list)
				setIsLoadingInstances(false)
			}
		} catch (error) {
			console.error("Failed to open dialog:", error)
		}
	}

	const handleBrowseCustomFolder = async () => {
		try {
			const selected = await openDialog({
				directory: true,
				multiple: false,
				title: "Select Minecraft Instance Directory",
			})
			if (selected && typeof selected === "string") {
				setIsLoadingInstances(true)
				const inst = await importerService.detectCustomInstance(selected)
				setIsLoadingInstances(false)
				if (inst) {
					handleSelectInstance(inst)
				}
			}
		} catch (error) {
			console.error("Failed to open directory dialog:", error)
		}
	}

	const handleSelectInstance = (inst: ImportableInstance) => {
		setSelectedInstance(inst)
		setInstanceName(inst.name)
		setStep("options")
	}

	const handleImport = async () => {
		if (!selectedInstance) return

		setIsImporting(true)
		setImportStatus(null)

		try {
			const report = await importerService.importInstance({
				sourcePath: selectedInstance.sourcePath,
				launcherId: selectedInstance.launcherId,
				name: instanceName.trim() || selectedInstance.name,
				gameVersion: selectedInstance.gameVersion,
				loader: selectedInstance.loader,
				loaderVersion: selectedInstance.loaderVersion,
				copyMods,
				copyConfigs,
				copySaves,
				copyResourcePacks,
				copyOptions,
				copyServers,
				copyScreenshots,
				copyExtraData,
			})

			setImportStatus(report.message || "Instance imported successfully!")

			if (onSuccess) {
				onSuccess()
			}

			setTimeout(() => {
				onOpenChange(false)
			}, 1200)
		} catch (error) {
			console.error("Failed to import instance:", error)
			setImportStatus(`Error: ${error}`)
		} finally {
			setIsImporting(false)
		}
	}

	const filteredInstances = instances.filter((inst) => {
		const q = searchQuery.toLowerCase()
		return (
			inst.name.toLowerCase().includes(q) ||
			inst.gameVersion.toLowerCase().includes(q) ||
			inst.loader.toLowerCase().includes(q)
		)
	})

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent className="max-h-[90vh] w-full gap-3 border-border/60 bg-zinc-950 p-4 shadow-2xl backdrop-blur-2xl sm:max-w-xl sm:p-5 md:max-w-2xl">
				<DialogHeader className="gap-1">
					<div className="flex items-center gap-2 text-primary">
						<FolderOpen className="size-5 shrink-0" />
						<DialogTitle className="truncate font-semibold text-lg text-white">
							{step === "launchers" && "Import from Another Launcher"}
							{step === "instances" && `Select Instance (${selectedLauncher?.name})`}
							{step === "options" && "Configure Import"}
						</DialogTitle>
					</div>
					<DialogDescription className="text-xs text-zinc-400">
						{step === "launchers" &&
							"Import existing Minecraft installations, mods, worlds, and settings directly into Ingot."}
						{step === "instances" &&
							`Found ${instances.length} instance${instances.length === 1 ? "" : "s"} in ${selectedLauncher?.name}. Choose which one to import.`}
						{step === "options" &&
							"Select which data components you want to transfer into your new Ingot instance."}
					</DialogDescription>
				</DialogHeader>

				{/* STEP 1: Launcher Selection */}
				{step === "launchers" && (
					<ScrollArea scrollFade className="-mr-2 max-h-[min(520px,70vh)] py-1 pr-2">
						{isLoadingLaunchers ? (
							<div className="flex flex-col items-center justify-center gap-2 py-12 text-muted-foreground text-xs">
								<Loader2 className="size-6 animate-spin text-primary" />
								<span>Detecting installed launchers...</span>
							</div>
						) : (
							<div className="grid grid-cols-1 gap-2.5 sm:grid-cols-2">
								{launchers.map((l) => {
									const meta = LAUNCHER_META[l.id] || {
										icon: Box,
										color: "text-zinc-400",
										bg: "bg-zinc-800",
										border: "hover:border-zinc-700",
									}
									const Icon = meta.icon

									return (
										<button
											key={l.id}
											type="button"
											onClick={() => {
												if (l.available) {
													handleSelectLauncher(l)
												} else {
													handleBrowseLauncherFolder(l)
												}
											}}
											className={`group flex cursor-pointer items-center justify-between gap-3 rounded-xl border border-zinc-800/80 bg-zinc-900/40 p-3 text-left transition-all sm:p-3.5 ${meta.border} hover:bg-zinc-900/80`}
										>
											<div className="flex min-w-0 flex-1 items-center gap-3">
												<div
													className={`flex size-9.5 shrink-0 items-center justify-center rounded-xl ${meta.bg} ${meta.color} transition-transform group-hover:scale-105`}
												>
													<Icon className="size-5" />
												</div>
												<div className="min-w-0 flex-1">
													<div className="flex items-center gap-2">
														<span
															className="truncate font-semibold text-foreground text-sm"
															title={l.name}
														>
															{l.name}
														</span>
														{l.available && (
															<span className="shrink-0 rounded-full border border-emerald-500/30 bg-emerald-500/10 px-1.5 py-0.5 font-medium text-[10px] text-emerald-400">
																Installed
															</span>
														)}
													</div>
													<div className="mt-0.5 truncate text-[11px] text-muted-foreground">
														{l.available
															? `${l.instancesCount} instance${l.instancesCount === 1 ? "" : "s"} found`
															: "Not detected • Click to locate"}
													</div>
												</div>
											</div>

											<div className="shrink-0 text-muted-foreground/40 transition-colors group-hover:text-foreground">
												{l.available ? (
													<ChevronRight className="size-4.5 transition-transform group-hover:translate-x-0.5" />
												) : (
													<FolderSearch className="size-4" />
												)}
											</div>
										</button>
									)
								})}

								{/* Custom Folder Card */}
								<button
									type="button"
									onClick={handleBrowseCustomFolder}
									className="group flex cursor-pointer items-center justify-between gap-3 rounded-xl border border-border/40 border-dashed bg-zinc-900/20 p-3 text-left transition-all hover:border-primary/50 hover:bg-zinc-900/50 sm:p-3.5"
								>
									<div className="flex min-w-0 flex-1 items-center gap-3">
										<div className="flex size-9.5 shrink-0 items-center justify-center rounded-xl bg-zinc-800 text-zinc-300 transition-transform group-hover:scale-105">
											<FolderSearch className="size-5" />
										</div>
										<div className="min-w-0 flex-1">
											<div className="truncate font-semibold text-foreground text-sm">
												Custom Directory
											</div>
											<div className="mt-0.5 truncate text-[11px] text-muted-foreground">
												Select any instance folder on disk
											</div>
										</div>
									</div>

									<div className="shrink-0 text-muted-foreground/40 transition-colors group-hover:text-foreground">
										<FolderSearch className="size-4" />
									</div>
								</button>
							</div>
						)}
					</ScrollArea>
				)}

				{/* STEP 2: Instance Selection */}
				{step === "instances" && (
					<div className="flex flex-col gap-3 py-1">
						<div className="flex items-center justify-between gap-2">
							<div className="flex items-center gap-1.5">
								<Button
									type="button"
									variant="ghost"
									size="sm"
									onClick={() => setStep("launchers")}
									className="h-8 gap-1 text-muted-foreground text-xs hover:text-foreground"
								>
									<ArrowLeft className="size-3.5" />
									<span>Back to Launchers</span>
								</Button>

								{selectedLauncher && (
									<Button
										type="button"
										variant="ghost"
										size="sm"
										onClick={() => handleBrowseLauncherFolder(selectedLauncher)}
										className="h-8 gap-1 text-muted-foreground text-xs hover:text-foreground"
									>
										<FolderSearch className="size-3.5" />
										<span>Change Folder</span>
									</Button>
								)}
							</div>

							{instances.length > 3 && (
								<div className="relative max-w-xs flex-1">
									<Search className="absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-muted-foreground" />
									<Input
										placeholder="Filter instances..."
										value={searchQuery}
										onChange={(e) => setSearchQuery(e.target.value)}
										className="h-8 pl-8 text-xs"
									/>
								</div>
							)}
						</div>

						<ScrollArea scrollFade className="max-h-[50vh] pr-2">
							<div className="flex flex-col gap-2 py-1">
								{isLoadingInstances ? (
									<div className="flex flex-col items-center justify-center gap-2 py-12 text-muted-foreground text-xs">
										<Loader2 className="size-6 animate-spin text-primary" />
										<span>Loading instances...</span>
									</div>
								) : filteredInstances.length === 0 ? (
									<div className="py-10 text-center text-muted-foreground text-xs">
										No instances found in this launcher directory.
									</div>
								) : (
									filteredInstances.map((inst) => (
										<div
											key={inst.id}
											className="flex items-center justify-between gap-3 rounded-xl border border-zinc-800/80 bg-zinc-900/40 p-3 transition-colors hover:border-zinc-700 hover:bg-zinc-900/70"
										>
											<div className="flex min-w-0 flex-1 items-center gap-3">
												<div className="shrink-0">
													<LoaderIcon loader={inst.loader} size={22} />
												</div>
												<div className="min-w-0 flex-1">
													<div className="truncate font-semibold text-foreground text-xs sm:text-sm">
														{inst.name}
													</div>
													<div className="flex flex-wrap items-center gap-2 pt-0.5 text-[10px] text-muted-foreground">
														<span>
															MC {inst.gameVersion} • {inst.loader}
														</span>
														{inst.modsCount > 0 && (
															<span className="rounded bg-zinc-800 px-1.5 py-0.5 text-zinc-300">
																{inst.modsCount} mods
															</span>
														)}
														{inst.savesCount > 0 && (
															<span className="rounded bg-zinc-800 px-1.5 py-0.5 text-zinc-300">
																{inst.savesCount} worlds
															</span>
														)}
														{inst.screenshotsCount > 0 && (
															<span className="rounded bg-zinc-800 px-1.5 py-0.5 text-zinc-300">
																{inst.screenshotsCount} screenshots
															</span>
														)}
													</div>
												</div>
											</div>

											<Button
												size="sm"
												variant="default"
												onClick={() => handleSelectInstance(inst)}
												className="h-8 shrink-0 gap-1 text-xs"
											>
												<span>Select</span>
											</Button>
										</div>
									))
								)}
							</div>
						</ScrollArea>
					</div>
				)}

				{/* STEP 3: Options & Confirmation */}
				{step === "options" && selectedInstance && (
					<ScrollArea scrollFade className="-mr-2 max-h-[min(540px,65vh)] py-1 pr-2">
						<div className="flex flex-col gap-4 py-1">
							<div className="flex items-center justify-between">
								<Button
									type="button"
									variant="ghost"
									size="sm"
									onClick={() => setStep(selectedLauncher ? "instances" : "launchers")}
									className="h-8 gap-1 text-muted-foreground text-xs hover:text-foreground"
								>
									<ArrowLeft className="size-3.5" />
									<span>Back</span>
								</Button>
							</div>

							{/* Instance Info Card */}
							<div className="flex flex-col gap-2 rounded-xl border border-zinc-800/80 bg-zinc-900/40 p-3.5">
								<div className="flex items-center gap-3">
									<LoaderIcon loader={selectedInstance.loader} size={24} />
									<div className="flex-1">
										<label htmlFor="import-name" className="text-[10px] text-muted-foreground">
											Instance Name in Ingot:
										</label>
										<Input
											id="import-name"
											value={instanceName}
											onChange={(e) => setInstanceName(e.target.value)}
											className="mt-1 h-8 font-medium text-xs"
											placeholder="My Imported Instance"
										/>
									</div>
								</div>

								<div className="flex flex-wrap items-center gap-2 pt-1 font-mono text-[10px] text-zinc-400">
									<span className="rounded bg-zinc-800 px-2 py-0.5">
										MC {selectedInstance.gameVersion}
									</span>
									<span className="rounded bg-zinc-800 px-2 py-0.5">
										{selectedInstance.loader}{" "}
										{selectedInstance.loaderVersion ? `(${selectedInstance.loaderVersion})` : ""}
									</span>
									<span className="truncate text-zinc-500">
										Source: {selectedInstance.sourcePath}
									</span>
								</div>
							</div>

							{/* Transfer components checkboxes */}
							<div className="flex flex-col gap-2">
								<span className="font-medium text-[11px] text-zinc-400">
									Select components to import:
								</span>

								<div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
									<label
										htmlFor="copy-mods"
										className="flex cursor-pointer items-center justify-between gap-2.5 rounded-lg border border-border/40 bg-zinc-900/30 p-2.5 transition-colors hover:bg-zinc-900/60"
									>
										<div className="flex min-w-0 flex-1 items-center gap-2.5">
											<Layers className="size-4 shrink-0 text-sky-400" />
											<div className="min-w-0 flex-1">
												<div className="truncate font-medium text-foreground text-xs">Mods</div>
												<div className="truncate text-[10px] text-muted-foreground">
													{selectedInstance.modsCount} mod files
												</div>
											</div>
										</div>
										<Checkbox
											id="copy-mods"
											checked={copyMods}
											onCheckedChange={(c) => setCopyMods(Boolean(c))}
											className="shrink-0"
										/>
									</label>

									<label
										htmlFor="copy-configs"
										className="flex cursor-pointer items-center justify-between gap-2.5 rounded-lg border border-border/40 bg-zinc-900/30 p-2.5 transition-colors hover:bg-zinc-900/60"
									>
										<div className="flex min-w-0 flex-1 items-center gap-2.5">
											<Sliders className="size-4 shrink-0 text-amber-400" />
											<div className="min-w-0 flex-1">
												<div className="truncate font-medium text-foreground text-xs">Configs</div>
												<div className="truncate text-[10px] text-muted-foreground">
													Settings & mod configs
												</div>
											</div>
										</div>
										<Checkbox
											id="copy-configs"
											checked={copyConfigs}
											onCheckedChange={(c) => setCopyConfigs(Boolean(c))}
											className="shrink-0"
										/>
									</label>

									<label
										htmlFor="copy-saves"
										className="flex cursor-pointer items-center justify-between gap-2.5 rounded-lg border border-border/40 bg-zinc-900/30 p-2.5 transition-colors hover:bg-zinc-900/60"
									>
										<div className="flex min-w-0 flex-1 items-center gap-2.5">
											<HardDrive className="size-4 shrink-0 text-emerald-400" />
											<div className="min-w-0 flex-1">
												<div className="truncate font-medium text-foreground text-xs">
													Worlds & Saves
												</div>
												<div className="truncate text-[10px] text-muted-foreground">
													{selectedInstance.savesCount} singleplayer worlds
												</div>
											</div>
										</div>
										<Checkbox
											id="copy-saves"
											checked={copySaves}
											onCheckedChange={(c) => setCopySaves(Boolean(c))}
											className="shrink-0"
										/>
									</label>

									<label
										htmlFor="copy-resourcepacks"
										className="flex cursor-pointer items-center justify-between gap-2.5 rounded-lg border border-border/40 bg-zinc-900/30 p-2.5 transition-colors hover:bg-zinc-900/60"
									>
										<div className="flex min-w-0 flex-1 items-center gap-2.5">
											<Box className="size-4 shrink-0 text-pink-400" />
											<div className="min-w-0 flex-1">
												<div className="truncate font-medium text-foreground text-xs">
													Resource Packs
												</div>
												<div className="truncate text-[10px] text-muted-foreground">
													Textures & shaderpacks
												</div>
											</div>
										</div>
										<Checkbox
											id="copy-resourcepacks"
											checked={copyResourcePacks}
											onCheckedChange={(c) => setCopyResourcePacks(Boolean(c))}
											className="shrink-0"
										/>
									</label>

									<label
										htmlFor="copy-screenshots"
										className="flex cursor-pointer items-center justify-between gap-2.5 rounded-lg border border-border/40 bg-zinc-900/30 p-2.5 transition-colors hover:bg-zinc-900/60"
									>
										<div className="flex min-w-0 flex-1 items-center gap-2.5">
											<Camera className="size-4 shrink-0 text-cyan-400" />
											<div className="min-w-0 flex-1">
												<div className="truncate font-medium text-foreground text-xs">
													Screenshots
												</div>
												<div className="truncate text-[10px] text-muted-foreground">
													{selectedInstance.screenshotsCount > 0
														? `${selectedInstance.screenshotsCount} screenshots`
														: "In-game screenshots"}
												</div>
											</div>
										</div>
										<Checkbox
											id="copy-screenshots"
											checked={copyScreenshots}
											onCheckedChange={(c) => setCopyScreenshots(Boolean(c))}
											className="shrink-0"
										/>
									</label>

									<label
										htmlFor="copy-extra-data"
										className="flex cursor-pointer items-center justify-between gap-2.5 rounded-lg border border-border/40 bg-zinc-900/30 p-2.5 transition-colors hover:bg-zinc-900/60"
									>
										<div className="flex min-w-0 flex-1 items-center gap-2.5">
											<Sparkles className="size-4 shrink-0 text-teal-400" />
											<div className="min-w-0 flex-1">
												<div className="truncate font-medium text-foreground text-xs">
													Mod Data & Others
												</div>
												<div className="truncate text-[10px] text-muted-foreground">
													Waypoints, schematics, recordings, etc.
												</div>
											</div>
										</div>
										<Checkbox
											id="copy-extra-data"
											checked={copyExtraData}
											onCheckedChange={(c) => setCopyExtraData(Boolean(c))}
											className="shrink-0"
										/>
									</label>

									<label
										htmlFor="copy-options"
										className="flex cursor-pointer items-center justify-between gap-2.5 rounded-lg border border-border/40 bg-zinc-900/30 p-2.5 transition-colors hover:bg-zinc-900/60"
									>
										<div className="flex min-w-0 flex-1 items-center gap-2.5">
											<Sliders className="size-4 shrink-0 text-purple-400" />
											<div className="min-w-0 flex-1">
												<div className="truncate font-medium text-foreground text-xs">
													Game Options
												</div>
												<div className="truncate text-[10px] text-muted-foreground">
													options.txt & keybindings
												</div>
											</div>
										</div>
										<Checkbox
											id="copy-options"
											checked={copyOptions}
											onCheckedChange={(c) => setCopyOptions(Boolean(c))}
											className="shrink-0"
										/>
									</label>

									<label
										htmlFor="copy-servers"
										className="flex cursor-pointer items-center justify-between gap-2.5 rounded-lg border border-border/40 bg-zinc-900/30 p-2.5 transition-colors hover:bg-zinc-900/60"
									>
										<div className="flex min-w-0 flex-1 items-center gap-2.5">
											<Server className="size-4 shrink-0 text-emerald-400" />
											<div className="min-w-0 flex-1">
												<div className="truncate font-medium text-foreground text-xs">
													Multiplayer Servers
												</div>
												<div className="truncate text-[10px] text-muted-foreground">
													servers.dat server list
												</div>
											</div>
										</div>
										<Checkbox
											id="copy-servers"
											checked={copyServers}
											onCheckedChange={(c) => setCopyServers(Boolean(c))}
											className="shrink-0"
										/>
									</label>
								</div>

								{/* Info notice */}
								<div className="mt-1 flex items-start gap-2.5 rounded-lg border border-border/40 bg-zinc-900/20 p-2.5 text-xs text-zinc-400">
									<Sparkles className="mt-0.5 size-4 shrink-0 text-primary" />
									<span className="text-[11px] text-zinc-400 leading-relaxed">
										All components above are optional. With{" "}
										<strong className="text-zinc-200">Mod Data & Others</strong> enabled, custom mod
										folders (minimap waypoints, schematics, recordings) are safely copied. Uncheck
										it if you prefer a clean import without extra files.
									</span>
								</div>
							</div>

							{importStatus && (
								<div className="flex items-center gap-2 rounded-lg border border-emerald-500/20 bg-emerald-500/5 p-2.5 text-emerald-300 text-xs">
									<CheckCircle2 className="size-4 shrink-0 text-emerald-400" />
									<span>{importStatus}</span>
								</div>
							)}
						</div>
					</ScrollArea>
				)}

				{step === "options" && (
					<DialogFooter className="mt-2 border-border/40 border-t pt-3">
						<Button
							onClick={handleImport}
							disabled={isImporting || !instanceName.trim()}
							className="w-full gap-1.5 font-medium"
						>
							{isImporting ? (
								<Loader2 className="size-3.5 animate-spin" />
							) : (
								<Check className="size-3.5" />
							)}
							{isImporting ? "Importing..." : "Import Instance"}
						</Button>
					</DialogFooter>
				)}
			</DialogContent>
		</Dialog>
	)
}

export default ImportInstanceDialog
