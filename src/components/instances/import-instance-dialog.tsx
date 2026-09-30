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
	Rocket,
	Search,
	Server,
	Sliders,
	Sparkles,
} from "lucide-react"
import { useEffect, useState } from "react"
import { Trans, useTranslation } from "react-i18next"
import type { DetectedLauncher, ImportableInstance } from "@/bindings"
import { alertTone } from "@/components/common/alert-tones"
import { ScrollArea } from "@/components/common/scroll-area"
import LoaderIcon from "@/components/instances/loader-icon"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Badge } from "@/components/ui/badge"
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
import { InputGroup, InputGroupAddon, InputGroupInput } from "@/components/ui/input-group"
import {
	Item,
	ItemActions,
	ItemContent,
	ItemDescription,
	ItemMedia,
	ItemTitle,
} from "@/components/ui/item"
import { Label } from "@/components/ui/label"
import { Spinner } from "@/components/ui/spinner"
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
	const { t } = useTranslation()
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
				title: t("importInstance.selectLauncherDir", { launcher: launcher.name }),
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
				title: t("importInstance.selectDir"),
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

			setImportStatus(report.message || t("importInstance.done"))

			if (onSuccess) {
				onSuccess()
			}

			setTimeout(() => {
				onOpenChange(false)
			}, 1200)
		} catch (error) {
			console.error("Failed to import instance:", error)
			setImportStatus(t("syncMaster.error", { error: String(error) }))
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
			<DialogContent className="max-h-[90vh] w-full gap-3 p-4 sm:max-w-xl sm:p-5 md:max-w-2xl">
				<DialogHeader className="gap-1">
					<div className="flex items-center gap-2 text-primary">
						<FolderOpen className="size-5 shrink-0" />
						<DialogTitle className="truncate font-semibold text-foreground text-lg">
							{step === "launchers" && t("importInstance.title")}
							{step === "instances" &&
								t("importInstance.selectInstance", { launcher: selectedLauncher?.name ?? "" })}
							{step === "options" && t("importInstance.configure")}
						</DialogTitle>
					</div>
					<DialogDescription className="text-muted-foreground text-xs">
						{step === "launchers" && t("importInstance.description")}
						{step === "instances" &&
							t("importInstance.found", {
								count: instances.length,
								launcher: selectedLauncher?.name ?? "",
							})}
						{step === "options" && t("importInstance.optionsDescription")}
					</DialogDescription>
				</DialogHeader>

				{/* STEP 1: Launcher Selection */}
				{step === "launchers" && (
					<ScrollArea scrollFade className="-mr-2 max-h-[min(520px,70vh)] py-1 pr-2">
						{isLoadingLaunchers ? (
							<div className="flex flex-col items-center justify-center gap-2 py-12 text-muted-foreground text-xs">
								<Spinner className="size-6 text-primary" />
								<span>{t("importInstance.detecting")}</span>
							</div>
						) : (
							<div className="grid grid-cols-1 gap-2.5 sm:grid-cols-2">
								{launchers.map((l) => {
									const meta = LAUNCHER_META[l.id] || {
										icon: Box,
										color: "text-muted-foreground",
										bg: "bg-muted",
										border: "hover:border-input",
									}
									const Icon = meta.icon

									return (
										<Item
											key={l.id}
											variant="outline"
											render={
												<button
													type="button"
													onClick={() => {
														if (l.available) {
															handleSelectLauncher(l)
														} else {
															handleBrowseLauncherFolder(l)
														}
													}}
												/>
											}
											className="flex-nowrap text-left hover:bg-muted/50"
										>
											<ItemMedia className={`size-9.5 rounded-lg ${meta.bg} ${meta.color}`}>
												<Icon className="size-5" />
											</ItemMedia>
											<ItemContent className="min-w-0">
												<ItemTitle>
													<span className="truncate" title={l.name}>
														{l.name}
													</span>
													{l.available && (
														<Badge variant="outline" className="border-primary/30 text-primary">
															{t("common.installed")}
														</Badge>
													)}
												</ItemTitle>
												<ItemDescription className="truncate">
													{l.available
														? t("importInstance.instancesFound", { count: l.instancesCount })
														: t("importInstance.notDetected")}
												</ItemDescription>
											</ItemContent>
											<ItemActions className="text-muted-foreground">
												{l.available ? (
													<ChevronRight className="size-4" />
												) : (
													<FolderSearch className="size-4" />
												)}
											</ItemActions>
										</Item>
									)
								})}

								{/* Custom Folder Card */}
								<Item
									variant="outline"
									render={<button type="button" onClick={handleBrowseCustomFolder} />}
									className="flex-nowrap border-dashed text-left hover:bg-muted/50"
								>
									<ItemMedia className="size-9.5 rounded-lg bg-muted">
										<FolderSearch className="size-5" />
									</ItemMedia>
									<ItemContent className="min-w-0">
										<ItemTitle>{t("importInstance.custom")}</ItemTitle>
										<ItemDescription className="truncate">
											{t("importInstance.customHint")}
										</ItemDescription>
									</ItemContent>
								</Item>
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
									className="h-8 gap-1 text-muted-foreground text-xs"
								>
									<ArrowLeft className="size-3.5" />
									<span>{t("importInstance.backToLaunchers")}</span>
								</Button>

								{selectedLauncher && (
									<Button
										type="button"
										variant="ghost"
										size="sm"
										onClick={() => handleBrowseLauncherFolder(selectedLauncher)}
										className="h-8 gap-1 text-muted-foreground text-xs"
									>
										<FolderSearch className="size-3.5" />
										<span>{t("importInstance.changeFolder")}</span>
									</Button>
								)}
							</div>

							{instances.length > 3 && (
								<InputGroup className="max-w-xs flex-1">
									<InputGroupAddon>
										<Search />
									</InputGroupAddon>
									<InputGroupInput
										placeholder={t("importInstance.filter")}
										value={searchQuery}
										onChange={(e) => setSearchQuery(e.target.value)}
										className="text-xs"
									/>
								</InputGroup>
							)}
						</div>

						<ScrollArea scrollFade className="max-h-[50vh] pr-2">
							<div className="flex flex-col gap-2 py-1">
								{isLoadingInstances ? (
									<div className="flex flex-col items-center justify-center gap-2 py-12 text-muted-foreground text-xs">
										<Spinner className="size-6 text-primary" />
										<span>{t("importInstance.loadingInstances")}</span>
									</div>
								) : filteredInstances.length === 0 ? (
									<div className="py-10 text-center text-muted-foreground text-xs">
										{t("importInstance.noneFound")}
									</div>
								) : (
									filteredInstances.map((inst) => (
										<div
											key={inst.id}
											className="flex items-center justify-between gap-3 rounded-xl border border-border/80 bg-card/40 p-3 transition-colors hover:border-input hover:bg-card/70"
										>
											<div className="flex min-w-0 flex-1 items-center gap-3">
												<div className="shrink-0">
													<LoaderIcon loader={inst.loader} size={22} />
												</div>
												<div className="min-w-0 flex-1">
													<div className="truncate font-semibold text-foreground text-xs sm:text-sm">
														{inst.name}
													</div>
													<div className="flex flex-wrap items-center gap-2 pt-0.5 text-3xs text-muted-foreground">
														<span>
															MC {inst.gameVersion} • {inst.loader}
														</span>
														{inst.modsCount > 0 && (
															<span className="rounded bg-muted px-1.5 py-0.5 text-foreground/80">
																{t("importInstance.modsCount", { count: inst.modsCount })}
															</span>
														)}
														{inst.savesCount > 0 && (
															<span className="rounded bg-muted px-1.5 py-0.5 text-foreground/80">
																{t("importInstance.worldsCount", { count: inst.savesCount })}
															</span>
														)}
														{inst.screenshotsCount > 0 && (
															<span className="rounded bg-muted px-1.5 py-0.5 text-foreground/80">
																{t("importInstance.screenshotsCount", {
																	count: inst.screenshotsCount,
																})}
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
												<span>{t("importInstance.select")}</span>
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
									className="h-8 gap-1 text-muted-foreground text-xs"
								>
									<ArrowLeft className="size-3.5" />
									<span>{t("versionChange.back")}</span>
								</Button>
							</div>

							{/* Instance Info Card */}
							<div className="flex flex-col gap-2 rounded-xl border border-border/80 bg-card/40 p-3.5">
								<div className="flex items-center gap-3">
									<LoaderIcon loader={selectedInstance.loader} size={24} />
									<div className="flex-1">
										<Label htmlFor="import-name" className="text-3xs text-muted-foreground">
											{t("importInstance.nameInIngot")}
										</Label>
										<Input
											id="import-name"
											value={instanceName}
											onChange={(e) => setInstanceName(e.target.value)}
											className="mt-1 h-8 font-medium text-xs"
											placeholder={t("importInstance.namePlaceholder")}
										/>
									</div>
								</div>

								<div className="flex flex-wrap items-center gap-2 pt-1 font-mono text-3xs text-muted-foreground">
									<span className="rounded bg-muted px-2 py-0.5">
										MC {selectedInstance.gameVersion}
									</span>
									<span className="rounded bg-muted px-2 py-0.5">
										{selectedInstance.loader}{" "}
										{selectedInstance.loaderVersion ? `(${selectedInstance.loaderVersion})` : ""}
									</span>
									<span className="truncate text-muted-foreground">
										{t("importInstance.source", { path: selectedInstance.sourcePath })}
									</span>
								</div>
							</div>

							{/* Transfer components checkboxes */}
							<div className="flex flex-col gap-2">
								<span className="font-medium text-2xs text-muted-foreground">
									{t("importInstance.components")}
								</span>

								<div className="grid grid-cols-1 gap-2 sm:grid-cols-2">
									<Label
										htmlFor="copy-mods"
										className="flex cursor-pointer items-center justify-between gap-2.5 rounded-lg border border-border/40 bg-card/30 p-2.5 transition-colors hover:bg-card/60"
									>
										<div className="flex min-w-0 flex-1 items-center gap-2.5">
											<Layers className="size-4 shrink-0 text-info" />
											<div className="min-w-0 flex-1">
												<div className="truncate font-medium text-foreground text-xs">
													{t("importInstance.mods")}
												</div>
												<div className="truncate text-3xs text-muted-foreground">
													{t("importInstance.modFiles", { count: selectedInstance.modsCount })}
												</div>
											</div>
										</div>
										<Checkbox
											id="copy-mods"
											checked={copyMods}
											onCheckedChange={(c) => setCopyMods(Boolean(c))}
											className="shrink-0"
										/>
									</Label>

									<Label
										htmlFor="copy-configs"
										className="flex cursor-pointer items-center justify-between gap-2.5 rounded-lg border border-border/40 bg-card/30 p-2.5 transition-colors hover:bg-card/60"
									>
										<div className="flex min-w-0 flex-1 items-center gap-2.5">
											<Sliders className="size-4 shrink-0 text-warning" />
											<div className="min-w-0 flex-1">
												<div className="truncate font-medium text-foreground text-xs">
													{t("importInstance.configs")}
												</div>
												<div className="truncate text-3xs text-muted-foreground">
													{t("importInstance.configsHint")}
												</div>
											</div>
										</div>
										<Checkbox
											id="copy-configs"
											checked={copyConfigs}
											onCheckedChange={(c) => setCopyConfigs(Boolean(c))}
											className="shrink-0"
										/>
									</Label>

									<Label
										htmlFor="copy-saves"
										className="flex cursor-pointer items-center justify-between gap-2.5 rounded-lg border border-border/40 bg-card/30 p-2.5 transition-colors hover:bg-card/60"
									>
										<div className="flex min-w-0 flex-1 items-center gap-2.5">
											<HardDrive className="size-4 shrink-0 text-primary" />
											<div className="min-w-0 flex-1">
												<div className="truncate font-medium text-foreground text-xs">
													{t("importInstance.worlds")}
												</div>
												<div className="truncate text-3xs text-muted-foreground">
													{t("importInstance.singleplayerWorlds", {
														count: selectedInstance.savesCount,
													})}
												</div>
											</div>
										</div>
										<Checkbox
											id="copy-saves"
											checked={copySaves}
											onCheckedChange={(c) => setCopySaves(Boolean(c))}
											className="shrink-0"
										/>
									</Label>

									<Label
										htmlFor="copy-resourcepacks"
										className="flex cursor-pointer items-center justify-between gap-2.5 rounded-lg border border-border/40 bg-card/30 p-2.5 transition-colors hover:bg-card/60"
									>
										<div className="flex min-w-0 flex-1 items-center gap-2.5">
											<Box className="size-4 shrink-0 text-pink-400" />
											<div className="min-w-0 flex-1">
												<div className="truncate font-medium text-foreground text-xs">
													{t("importInstance.packs")}
												</div>
												<div className="truncate text-3xs text-muted-foreground">
													{t("importInstance.packsHint")}
												</div>
											</div>
										</div>
										<Checkbox
											id="copy-resourcepacks"
											checked={copyResourcePacks}
											onCheckedChange={(c) => setCopyResourcePacks(Boolean(c))}
											className="shrink-0"
										/>
									</Label>

									<Label
										htmlFor="copy-screenshots"
										className="flex cursor-pointer items-center justify-between gap-2.5 rounded-lg border border-border/40 bg-card/30 p-2.5 transition-colors hover:bg-card/60"
									>
										<div className="flex min-w-0 flex-1 items-center gap-2.5">
											<Camera className="size-4 shrink-0 text-cyan-400" />
											<div className="min-w-0 flex-1">
												<div className="truncate font-medium text-foreground text-xs">
													{t("screenshots.title")}
												</div>
												<div className="truncate text-3xs text-muted-foreground">
													{selectedInstance.screenshotsCount > 0
														? t("importInstance.screenshotsCount", {
																count: selectedInstance.screenshotsCount,
															})
														: t("importInstance.screenshotsHint")}
												</div>
											</div>
										</div>
										<Checkbox
											id="copy-screenshots"
											checked={copyScreenshots}
											onCheckedChange={(c) => setCopyScreenshots(Boolean(c))}
											className="shrink-0"
										/>
									</Label>

									<Label
										htmlFor="copy-extra-data"
										className="flex cursor-pointer items-center justify-between gap-2.5 rounded-lg border border-border/40 bg-card/30 p-2.5 transition-colors hover:bg-card/60"
									>
										<div className="flex min-w-0 flex-1 items-center gap-2.5">
											<Sparkles className="size-4 shrink-0 text-teal-400" />
											<div className="min-w-0 flex-1">
												<div className="truncate font-medium text-foreground text-xs">
													{t("importInstance.modData")}
												</div>
												<div className="truncate text-3xs text-muted-foreground">
													{t("importInstance.modDataHint")}
												</div>
											</div>
										</div>
										<Checkbox
											id="copy-extra-data"
											checked={copyExtraData}
											onCheckedChange={(c) => setCopyExtraData(Boolean(c))}
											className="shrink-0"
										/>
									</Label>

									<Label
										htmlFor="copy-options"
										className="flex cursor-pointer items-center justify-between gap-2.5 rounded-lg border border-border/40 bg-card/30 p-2.5 transition-colors hover:bg-card/60"
									>
										<div className="flex min-w-0 flex-1 items-center gap-2.5">
											<Sliders className="size-4 shrink-0 text-purple-400" />
											<div className="min-w-0 flex-1">
												<div className="truncate font-medium text-foreground text-xs">
													{t("importInstance.options")}
												</div>
												<div className="truncate text-3xs text-muted-foreground">
													{t("importInstance.optionsHint")}
												</div>
											</div>
										</div>
										<Checkbox
											id="copy-options"
											checked={copyOptions}
											onCheckedChange={(c) => setCopyOptions(Boolean(c))}
											className="shrink-0"
										/>
									</Label>

									<Label
										htmlFor="copy-servers"
										className="flex cursor-pointer items-center justify-between gap-2.5 rounded-lg border border-border/40 bg-card/30 p-2.5 transition-colors hover:bg-card/60"
									>
										<div className="flex min-w-0 flex-1 items-center gap-2.5">
											<Server className="size-4 shrink-0 text-primary" />
											<div className="min-w-0 flex-1">
												<div className="truncate font-medium text-foreground text-xs">
													{t("importInstance.servers")}
												</div>
												<div className="truncate text-3xs text-muted-foreground">
													{t("importInstance.serversHint")}
												</div>
											</div>
										</div>
										<Checkbox
											id="copy-servers"
											checked={copyServers}
											onCheckedChange={(c) => setCopyServers(Boolean(c))}
											className="shrink-0"
										/>
									</Label>
								</div>

								{/* Info notice */}
								<div className="mt-1 flex items-start gap-2.5 rounded-lg border border-border/40 bg-card/20 p-2.5 text-muted-foreground text-xs">
									<Sparkles className="mt-0.5 size-4 shrink-0 text-primary" />
									<span className="text-2xs text-muted-foreground leading-relaxed">
										<Trans
											i18nKey="importInstance.note"
											components={{ b: <strong className="text-foreground" /> }}
										/>
									</span>
								</div>
							</div>

							{importStatus && (
								<Alert className={alertTone.success}>
									<CheckCircle2 />
									<AlertDescription>{importStatus}</AlertDescription>
								</Alert>
							)}
						</div>
					</ScrollArea>
				)}

				{step === "options" && (
					<DialogFooter className="mt-2 pt-3">
						<Button
							onClick={handleImport}
							disabled={isImporting || !instanceName.trim()}
							className="w-full gap-1.5 font-medium"
						>
							{isImporting ? <Spinner className="size-3.5" /> : <Check className="size-3.5" />}
							{isImporting ? t("importInstance.importing") : t("importInstance.import")}
						</Button>
					</DialogFooter>
				)}
			</DialogContent>
		</Dialog>
	)
}

export default ImportInstanceDialog
