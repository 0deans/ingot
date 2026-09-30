import {
	AlertCircle,
	ArrowUpDown,
	CheckCircle2,
	Cpu,
	DownloadCloud,
	FolderOpen,
	HardDrive,
	Monitor,
	RefreshCw,
	Terminal,
	Undo2,
	UploadCloud,
} from "lucide-react"
import { useEffect, useState } from "react"
import { Trans, useTranslation } from "react-i18next"
import type { InstanceConfig, VersionBackup } from "@/bindings"
import { ScrollArea } from "@/components/common/scroll-area"
import ChangeVersionDialog from "@/components/instances/change-version-dialog"
import InitialSyncDialog from "@/components/instances/initial-sync-dialog"
import LoaderIcon from "@/components/instances/loader-icon"
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
import { Label } from "@/components/ui/label"
import { Slider } from "@/components/ui/slider"
import { Spinner } from "@/components/ui/spinner"
import { Switch } from "@/components/ui/switch"
import { loaderName } from "@/components/version-change/plan-review"
import { formatMegabytes } from "@/lib/format"
import { formatBytes } from "@/lib/minecraft"
import { instanceService, rpc } from "@/services/instance-service"
import {
	settingsService,
	useMemorySettings,
	useSyncSettings,
	useWindowSettings,
} from "@/services/settings-service"

interface InstanceSettingsDialogProps {
	instance: InstanceConfig | null
	open: boolean
	onOpenChange: (open: boolean) => void
	onSave?: (updated: InstanceConfig) => Promise<void> | void
}

/** "1.21.4", or "Fabric 1.21.4" when the loader changed too */
function backupSide(backup: VersionBackup, side: "from" | "to"): string {
	const version = side === "from" ? backup.fromGameVersion : backup.toGameVersion
	if (backup.fromLoader === backup.toLoader) return version
	return `${loaderName(side === "from" ? backup.fromLoader : backup.toLoader)} ${version}`
}

const STEP_MB = 256
const MIN_RAM_LIMIT_MB = 512

export const InstanceSettingsDialog = ({
	instance,
	open,
	onOpenChange,
	onSave,
}: InstanceSettingsDialogProps) => {
	const { t } = useTranslation()
	const { memory: globalMemory, systemMemory } = useMemorySettings()
	const { windowSettings: globalWindow } = useWindowSettings()
	const { syncSettings: globalSync } = useSyncSettings()
	const totalRamMb = systemMemory?.totalMb || 16384

	const [name, setName] = useState("")
	const [useCustomRam, setUseCustomRam] = useState(false)
	const [minRamMb, setMinRamMb] = useState(2048)
	const [maxRamMb, setMaxRamMb] = useState(4096)
	const [jvmArgsStr, setJvmArgsStr] = useState("")
	const [javaPath, setJavaPath] = useState("")
	const [isSaving, setIsSaving] = useState(false)

	// Window mode settings
	const [useCustomWindow, setUseCustomWindow] = useState(false)
	const [instanceFullscreen, setInstanceFullscreen] = useState(false)
	const [instanceWidth, setInstanceWidth] = useState(854)
	const [instanceHeight, setInstanceHeight] = useState(480)

	// Sync settings
	const [useCustomSync, setUseCustomSync] = useState(false)
	const [syncOptions, setSyncOptions] = useState(true)
	const [syncServers, setSyncServers] = useState(true)
	const [syncResourcePacks, setSyncResourcePacks] = useState(true)
	const [syncCommandHistory, setSyncCommandHistory] = useState(false)
	const [syncCreativeHotbars, setSyncCreativeHotbars] = useState(false)
	const [lastSyncedAt, setLastSyncedAt] = useState<number | null>(instance?.lastSyncedAt ?? null)
	const [isInitialSyncOpen, setIsInitialSyncOpen] = useState(false)

	// Version change
	const [versionOpen, setVersionOpen] = useState(false)
	const [versionBackup, setVersionBackup] = useState<VersionBackup | null>(null)
	const [versionBusy, setVersionBusy] = useState(false)
	const [versionMsg, setVersionMsg] = useState<{ ok: boolean; text: string } | null>(null)

	// Manual sync actions state
	const [isSyncing, setIsSyncing] = useState(false)
	const [syncStatusMsg, setSyncStatusMsg] = useState<string | null>(null)
	const [syncStatusType, setSyncStatusType] = useState<"success" | "error" | null>(null)

	useEffect(() => {
		if (instance) {
			setName(instance.name)
			const hasCustom = instance.memoryMinMb != null || instance.memoryMaxMb != null
			setUseCustomRam(hasCustom)
			setMinRamMb(instance.memoryMinMb ?? globalMemory.minRamMb)
			setMaxRamMb(instance.memoryMaxMb ?? globalMemory.maxRamMb)
			setJvmArgsStr(instance.jvmArgs ? instance.jvmArgs.join(" ") : "")
			setJavaPath(instance.javaPath || "")

			const hasCustomWin =
				instance.fullscreen != null || instance.windowWidth != null || instance.windowHeight != null
			setUseCustomWindow(hasCustomWin)
			setInstanceFullscreen(instance.fullscreen ?? globalWindow.fullscreen)
			setInstanceWidth(instance.windowWidth ?? globalWindow.width)
			setInstanceHeight(instance.windowHeight ?? globalWindow.height)

			const hasCustomSyncData =
				instance.syncOptions != null ||
				instance.syncServers != null ||
				instance.syncResourcePacks != null ||
				instance.syncCommandHistory != null ||
				instance.syncCreativeHotbars != null
			setUseCustomSync(hasCustomSyncData)
			setSyncOptions(instance.syncOptions ?? globalSync.syncOptions)
			setSyncServers(instance.syncServers ?? globalSync.syncServers)
			setSyncResourcePacks(instance.syncResourcePacks ?? globalSync.syncResourcePacks)
			setSyncCommandHistory(instance.syncCommandHistory ?? globalSync.syncCommandHistory)
			setSyncCreativeHotbars(instance.syncCreativeHotbars ?? globalSync.syncCreativeHotbars)
			setLastSyncedAt(instance.lastSyncedAt ?? null)

			setSyncStatusMsg(null)
			setSyncStatusType(null)
		}
	}, [instance, globalMemory, globalWindow, globalSync])

	// By id: the instance object is replaced after a version change, and that must not
	// clear the message about it
	const instanceId = instance?.id
	useEffect(() => {
		if (!open || !instanceId) return
		setVersionMsg(null)
		rpc
			.get_instance_version_backup(instanceId)
			.then(setVersionBackup)
			.catch(() => setVersionBackup(null))
	}, [open, instanceId])

	const handleUndoVersion = async () => {
		if (!instance || !versionBackup) return
		setVersionBusy(true)
		setVersionMsg(null)
		try {
			const restored = await rpc.undo_instance_version_change(instance.id)
			setVersionBackup(null)
			setVersionMsg({ ok: true, text: t("crashDialog.undone", { version: restored.gameVersion }) })
			await onSave?.(restored)
		} catch (e) {
			setVersionMsg({ ok: false, text: String(e) })
		} finally {
			setVersionBusy(false)
		}
	}

	const handleDiscardBackup = async () => {
		if (!instance) return
		setVersionBusy(true)
		try {
			await rpc.discard_instance_version_backup(instance.id)
			setVersionBackup(null)
		} catch (e) {
			setVersionMsg({ ok: false, text: String(e) })
		} finally {
			setVersionBusy(false)
		}
	}

	const handleToggleCustomSync = (checked: boolean) => {
		if (checked && !lastSyncedAt) {
			setIsInitialSyncOpen(true)
			return
		}
		setUseCustomSync(checked)
	}

	const handleInitialSyncChoice = async (source: "instance" | "shared") => {
		if (!instance) return
		try {
			if (source === "instance") {
				await handlePushSync()
			} else {
				await handlePullSync()
			}
			const now = Math.floor(Date.now() / 1000)
			setLastSyncedAt(now)
			setUseCustomSync(true)
			setIsInitialSyncOpen(false)
		} catch (e) {
			console.error("Initial sync choice failed:", e)
		}
	}

	const handlePushSync = async () => {
		if (!instance) return
		setIsSyncing(true)
		setSyncStatusMsg(null)
		try {
			const report = await settingsService.pushInstanceSync(instance.id)
			setSyncStatusMsg(report.message)
			setSyncStatusType(
				report.optionsSynced ||
					report.serversSynced ||
					report.resourcePacksCount > 0 ||
					report.commandHistorySynced ||
					report.creativeHotbarsSynced
					? "success"
					: "success",
			)
		} catch (err) {
			setSyncStatusMsg(t("instanceSettings.pushFailed", { error: String(err) }))
			setSyncStatusType("error")
		} finally {
			setIsSyncing(false)
		}
	}

	const handlePullSync = async () => {
		if (!instance) return
		setIsSyncing(true)
		setSyncStatusMsg(null)
		try {
			const report = await settingsService.pullInstanceSync(instance.id)
			setSyncStatusMsg(report.message)
			setSyncStatusType("success")
		} catch (err) {
			setSyncStatusMsg(t("instanceSettings.pullFailed", { error: String(err) }))
			setSyncStatusType("error")
		} finally {
			setIsSyncing(false)
		}
	}

	const handleSave = async () => {
		if (!instance) return
		setIsSaving(true)
		try {
			const parsedArgs = jvmArgsStr.trim().split(/\s+/).filter(Boolean)

			// The version may have changed (or been undone) while this dialog was open
			const latest = (await rpc.get_instances()).find((i) => i.id === instance.id) ?? instance
			const updated: InstanceConfig = {
				...latest,
				name: name.trim() || instance.name,
				memoryMinMb: useCustomRam ? minRamMb : null,
				memoryMaxMb: useCustomRam ? maxRamMb : null,
				jvmArgs: parsedArgs.length > 0 ? parsedArgs : null,
				javaPath: javaPath.trim() || null,
				fullscreen: useCustomWindow ? instanceFullscreen : null,
				windowWidth: useCustomWindow ? (instanceFullscreen ? null : instanceWidth) : null,
				windowHeight: useCustomWindow ? (instanceFullscreen ? null : instanceHeight) : null,
				syncOptions: useCustomSync ? syncOptions : null,
				syncServers: useCustomSync ? syncServers : null,
				syncResourcePacks: useCustomSync ? syncResourcePacks : null,
				syncCommandHistory: useCustomSync ? syncCommandHistory : null,
				syncCreativeHotbars: useCustomSync ? syncCreativeHotbars : null,
				lastSyncedAt: lastSyncedAt,
			}

			await instanceService.updateInstance(updated)
			if (onSave) {
				await onSave(updated)
			}
			onOpenChange(false)
		} catch (e) {
			console.error("Failed to update instance:", e)
			alert(t("instanceSettings.saveFailed", { error: String(e) }))
		} finally {
			setIsSaving(false)
		}
	}

	const applyPreset = (min: number, max: number) => {
		setMinRamMb(min)
		setMaxRamMb(Math.min(max, totalRamMb))
	}

	if (!instance) return null

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent className="max-h-[90vh] w-full gap-3 p-4 sm:max-w-xl sm:p-5 md:max-w-2xl">
				<DialogHeader className="gap-1.5">
					<div className="flex items-center gap-2.5">
						<LoaderIcon loader={instance.loader} size={24} />
						<DialogTitle className="font-semibold text-foreground text-lg">
							{t("instanceSettings.title")}
						</DialogTitle>
					</div>
					<DialogDescription className="text-muted-foreground text-xs">
						<Trans
							i18nKey="instanceSettings.description"
							values={{ name: instance.name }}
							components={{ b: <span className="font-medium text-foreground" /> }}
						/>
					</DialogDescription>
				</DialogHeader>

				<ScrollArea scrollFade className="-mr-2 max-h-[68vh] pr-2">
					<div className="flex flex-col gap-4 py-1 pr-1">
						{/* Instance Name */}
						<div className="flex flex-col gap-2">
							<Label htmlFor="instance-name-input" className="font-medium text-foreground text-xs">
								{t("newInstance.name")}
							</Label>
							<Input
								id="instance-name-input"
								value={name}
								onChange={(e) => setName(e.target.value)}
								placeholder={t("instanceSettings.namePlaceholder")}
								className="h-9 text-xs"
							/>
						</div>

						{/* Minecraft version */}
						<div className="flex flex-col gap-3 rounded-xl border border-border/80 bg-card/30 p-4">
							<div className="flex items-center justify-between gap-3">
								<div className="flex items-center gap-2">
									<ArrowUpDown className="size-4 text-primary" />
									<div>
										<h4 className="font-medium text-foreground text-xs">
											{t("wizard.minecraftVersion")}
										</h4>
										<p className="text-2xs text-muted-foreground">
											{instance.gameVersion}
											{instance.loaderVersion
												? ` · ${t("instanceSettings.loader", { version: instance.loaderVersion })}`
												: ""}
										</p>
									</div>
								</div>
								<Button
									variant="outline"
									size="sm"
									onClick={() => setVersionOpen(true)}
									disabled={versionBusy}
									className="h-8 text-xs"
								>
									{t("versionChange.title")}
								</Button>
							</div>
							{versionBackup && (
								<div className="flex flex-wrap items-center justify-between gap-2 border-border/80 border-t pt-3">
									<p className="text-2xs text-muted-foreground">
										{t(
											versionBackup.worldsBackedUp
												? "instanceSettings.backupWithWorlds"
												: "instanceSettings.backup",
											{
												from: backupSide(versionBackup, "from"),
												to: backupSide(versionBackup, "to"),
												size: formatBytes(versionBackup.bytes),
											},
										)}
									</p>
									<div className="flex gap-1.5">
										<Button
											variant="ghost"
											size="sm"
											onClick={handleDiscardBackup}
											disabled={versionBusy}
											className="h-7 text-2xs text-muted-foreground"
										>
											{t("transfer.deleteBackup")}
										</Button>
										<Button
											size="sm"
											onClick={handleUndoVersion}
											disabled={versionBusy}
											className="h-7 gap-1.5 text-2xs"
										>
											{versionBusy ? <Spinner className="size-3" /> : <Undo2 className="size-3" />}
											{t("transfer.undo", { version: backupSide(versionBackup, "from") })}
										</Button>
									</div>
								</div>
							)}
							{versionMsg && (
								<p
									className={versionMsg.ok ? "text-2xs text-primary" : "text-2xs text-destructive"}
								>
									{versionMsg.text}
								</p>
							)}
						</div>

						{/* Memory Override */}
						<div className="flex flex-col gap-3 rounded-xl border border-border/80 bg-card/30 p-4">
							<div className="flex items-center justify-between">
								<div className="flex items-center gap-2">
									<HardDrive className="size-4 text-info" />
									<div>
										<h4 className="font-medium text-foreground text-xs">
											{t("settings.memory.title")}
										</h4>
										<p className="text-2xs text-muted-foreground">
											{t("instanceSettings.memoryOverride")}
										</p>
									</div>
								</div>

								<Switch
									checked={useCustomRam}
									onCheckedChange={setUseCustomRam}
									aria-label={t("instanceSettings.memoryOverride")}
								/>
							</div>

							{useCustomRam ? (
								<div className="mt-2 flex flex-col gap-3 pt-2">
									{/* Preset Buttons */}
									<div className="flex flex-wrap gap-1.5">
										{[
											{ min: 2048, max: 4096 },
											{ min: 4096, max: 6144 },
											{ min: 6144, max: 8192 },
											{ min: 8192, max: 12288 },
										].map((p) => (
											<Button
												key={p.min}
												type="button"
												variant="outline"
												size="xs"
												onClick={() => applyPreset(p.min, p.max)}
											>
												{formatMegabytes(p.min)} – {formatMegabytes(p.max)}
											</Button>
										))}
									</div>

									{/* RAM Sliders */}
									<div className="flex flex-col gap-3 rounded-lg border border-border/60 bg-background/60 p-3">
										<div className="flex items-center justify-between text-xs">
											<span className="text-muted-foreground">
												{t("instanceSettings.minMemory")}
											</span>
											<span className="font-medium font-mono text-foreground">
												{formatMegabytes(minRamMb)}
											</span>
										</div>
										<Slider
											min={MIN_RAM_LIMIT_MB}
											max={totalRamMb}
											step={STEP_MB}
											value={[minRamMb]}
											onValueChange={(val) => {
												const n = Array.isArray(val) ? val[0] : val
												setMinRamMb(Math.min(n, maxRamMb))
											}}
										/>

										<div className="mt-2 flex items-center justify-between text-xs">
											<span className="text-muted-foreground">
												{t("instanceSettings.maxMemory")}
											</span>
											<span className="font-medium font-mono text-foreground">
												{formatMegabytes(maxRamMb)}
											</span>
										</div>
										<Slider
											min={MIN_RAM_LIMIT_MB}
											max={totalRamMb}
											step={STEP_MB}
											value={[maxRamMb]}
											onValueChange={(val) => {
												const n = Array.isArray(val) ? val[0] : val
												setMaxRamMb(Math.max(n, minRamMb))
											}}
										/>
									</div>
								</div>
							) : (
								<div className="rounded-lg border border-border/50 bg-background/40 px-3 py-2.5 text-2xs text-muted-foreground">
									<Trans
										i18nKey="instanceSettings.inheritMemory"
										values={{
											min: formatMegabytes(globalMemory.minRamMb),
											max: formatMegabytes(globalMemory.maxRamMb),
										}}
										components={{ b: <strong className="text-foreground" /> }}
									/>
								</div>
							)}
						</div>

						{/* Custom JVM Arguments */}
						<div className="flex flex-col gap-2 rounded-xl border border-border/80 bg-card/30 p-4">
							<div className="flex items-center gap-2">
								<Terminal className="size-4 text-primary" />
								<div>
									<h4 className="font-medium text-foreground text-xs">
										{t("instanceSettings.jvmArgs")}
									</h4>
									<p className="text-2xs text-muted-foreground">
										{t("instanceSettings.jvmArgsHint")}
									</p>
								</div>
							</div>

							<Input
								value={jvmArgsStr}
								onChange={(e) => setJvmArgsStr(e.target.value)}
								placeholder="-XX:+UseG1GC -Dminecraft.custom=true"
								className="mt-1 h-9 font-mono text-xs"
							/>
						</div>

						{/* Custom Java Binary */}
						<div className="flex flex-col gap-2 rounded-xl border border-border/80 bg-card/30 p-4">
							<div className="flex items-center gap-2">
								<Cpu className="size-4 text-warning" />
								<div>
									<h4 className="font-medium text-foreground text-xs">
										{t("instanceSettings.java")}
									</h4>
									<p className="text-2xs text-muted-foreground">{t("instanceSettings.javaHint")}</p>
								</div>
							</div>

							<Input
								value={javaPath}
								onChange={(e) => setJavaPath(e.target.value)}
								placeholder="C:\Program Files\Java\jdk-21\bin\javaw.exe"
								className="mt-1 h-9 font-mono text-xs"
							/>
						</div>

						{/* Window & Display Override */}
						<div className="flex flex-col gap-3 rounded-xl border border-border/80 bg-card/30 p-4">
							<div className="flex items-center justify-between">
								<div className="flex items-center gap-2">
									<Monitor className="size-4 text-info" />
									<div>
										<h4 className="font-medium text-foreground text-xs">
											{t("settings.window.title")}
										</h4>
										<p className="text-2xs text-muted-foreground">
											{t("instanceSettings.windowOverride")}
										</p>
									</div>
								</div>

								<Switch checked={useCustomWindow} onCheckedChange={setUseCustomWindow} />
							</div>

							{useCustomWindow ? (
								<div className="mt-2 flex flex-col gap-3 pt-1">
									<div className="flex items-center justify-between rounded-lg border border-border/60 bg-background/60 p-3">
										<div>
											<span className="font-medium text-foreground text-xs">
												{t("settings.window.fullscreenTitle")}
											</span>
											<p className="text-2xs text-muted-foreground">
												{t("instanceSettings.fullscreenHint")}
											</p>
										</div>
										<Switch checked={instanceFullscreen} onCheckedChange={setInstanceFullscreen} />
									</div>

									{!instanceFullscreen && (
										<div className="flex flex-col gap-2 rounded-lg border border-border/60 bg-background/60 p-3">
											<div className="flex items-center justify-between">
												<span className="text-muted-foreground text-xs">
													{t("instanceSettings.resolution")}
												</span>
												<div className="flex items-center gap-1.5">
													<Input
														type="number"
														value={instanceWidth}
														onChange={(e) => setInstanceWidth(Number(e.target.value) || 854)}
														className="h-7 w-16 px-1 text-center font-mono text-xs"
													/>
													<span className="text-muted-foreground text-xs">×</span>
													<Input
														type="number"
														value={instanceHeight}
														onChange={(e) => setInstanceHeight(Number(e.target.value) || 480)}
														className="h-7 w-16 px-1 text-center font-mono text-xs"
													/>
												</div>
											</div>

											<div className="flex flex-wrap gap-1 pt-1">
												{[
													{ label: "854 × 480", w: 854, h: 480 },
													{ label: "1024 × 768", w: 1024, h: 768 },
													{ label: "1280 × 720", w: 1280, h: 720 },
													{ label: "1920 × 1080", w: 1920, h: 1080 },
												].map((p) => (
													<Button
														key={p.label}
														type="button"
														size="xs"
														variant="outline"
														onClick={() => {
															setInstanceWidth(p.w)
															setInstanceHeight(p.h)
														}}
													>
														{p.label}
													</Button>
												))}
											</div>
										</div>
									)}
								</div>
							) : (
								<div className="rounded-lg border border-border/50 bg-background/40 px-3 py-2 text-2xs text-muted-foreground">
									<Trans
										i18nKey="instanceSettings.inheritDisplay"
										values={{
											value: globalWindow.fullscreen
												? t("instanceSettings.fullscreen")
												: `${globalWindow.width} × ${globalWindow.height}`,
										}}
										components={{ b: <strong className="text-foreground" /> }}
									/>
								</div>
							)}
						</div>

						{/* Game Data Synchronization */}
						<div className="flex flex-col gap-3 rounded-xl border border-border/80 bg-card/30 p-4">
							<div className="flex items-center justify-between">
								<div className="flex items-center gap-2">
									<RefreshCw className="size-4 text-primary" />
									<div>
										<h4 className="font-medium text-foreground text-xs">
											{t("settings.sync.title")}
										</h4>
										<p className="text-2xs text-muted-foreground">
											{t("instanceSettings.syncHint")}
										</p>
									</div>
								</div>

								<Switch checked={useCustomSync} onCheckedChange={handleToggleCustomSync} />
							</div>

							{/* Manual Sync Actions */}
							<div className="flex flex-col gap-2 rounded-lg border border-primary/20 bg-primary/5 p-3">
								<div>
									<div className="font-medium text-foreground text-xs">
										{t("instanceSettings.manualSync")}
									</div>
									<div className="text-2xs text-muted-foreground">
										{t("instanceSettings.manualSyncHint")}
									</div>
								</div>

								<div className="flex flex-wrap gap-2 pt-1">
									<Button
										type="button"
										variant="outline"
										size="sm"
										onClick={handlePushSync}
										disabled={isSyncing}
										className="h-8 gap-1.5 border-primary/30 bg-primary/10 text-primary text-xs hover:bg-primary/10"
									>
										{isSyncing ? (
											<Spinner className="size-3.5" />
										) : (
											<UploadCloud className="size-3.5" />
										)}
										{t("instanceSettings.push")}
									</Button>

									<Button
										type="button"
										variant="outline"
										size="sm"
										onClick={handlePullSync}
										disabled={isSyncing}
										className="h-8 gap-1.5 border-info/30 bg-info/10 text-info text-xs hover:bg-info/10"
									>
										{isSyncing ? (
											<Spinner className="size-3.5" />
										) : (
											<DownloadCloud className="size-3.5" />
										)}
										{t("instanceSettings.pull")}
									</Button>
								</div>

								{syncStatusMsg && (
									<div
										className={`mt-1 flex items-center gap-1.5 rounded p-2 text-xs ${
											syncStatusType === "success"
												? "bg-primary/10 text-primary"
												: "bg-destructive/10 text-destructive"
										}`}
									>
										{syncStatusType === "success" ? (
											<CheckCircle2 className="size-3.5 shrink-0 text-primary" />
										) : (
											<AlertCircle className="size-3.5 shrink-0 text-destructive" />
										)}
										<span className="text-2xs">{syncStatusMsg}</span>
									</div>
								)}
							</div>

							{useCustomSync ? (
								<div className="mt-1 flex flex-col divide-y divide-border/60 rounded-lg border border-border/60 bg-background/60">
									{[
										{
											key: "options",
											label: t("instanceSettings.syncOptions"),
											checked: syncOptions,
											setChecked: setSyncOptions,
										},
										{
											key: "servers",
											label: t("instanceSettings.syncServers"),
											checked: syncServers,
											setChecked: setSyncServers,
										},
										{
											key: "packs",
											label: t("instanceSettings.syncPacks"),
											checked: syncResourcePacks,
											setChecked: setSyncResourcePacks,
										},
										{
											key: "history",
											label: t("instanceSettings.syncHistory"),
											checked: syncCommandHistory,
											setChecked: setSyncCommandHistory,
										},
										{
											key: "hotbars",
											label: t("instanceSettings.syncHotbars"),
											checked: syncCreativeHotbars,
											setChecked: setSyncCreativeHotbars,
										},
									].map((item) => (
										<div key={item.key} className="flex items-center justify-between p-2.5">
											<span className="text-foreground text-xs">{item.label}</span>
											<Switch checked={item.checked} onCheckedChange={item.setChecked} />
										</div>
									))}
								</div>
							) : (
								<div className="rounded-lg border border-border/50 bg-background/40 px-3 py-2 text-2xs text-muted-foreground">
									{t("instanceSettings.inheritSync")}
								</div>
							)}
						</div>
					</div>
				</ScrollArea>

				<DialogFooter className="mt-2 flex flex-row items-center justify-between pt-3 sm:flex-row sm:justify-between">
					<Button
						type="button"
						variant="ghost"
						size="sm"
						onClick={() => instanceService.openInstanceFolder(instance.id)}
						className="gap-1.5 text-muted-foreground text-xs"
					>
						<FolderOpen className="size-3.5" />
						{t("common.openFolder")}
					</Button>

					<Button
						type="button"
						size="sm"
						onClick={handleSave}
						disabled={isSaving}
						className="font-medium"
					>
						{isSaving ? t("skinPreview.saving") : t("common.save")}
					</Button>
				</DialogFooter>
			</DialogContent>

			<InitialSyncDialog
				instanceName={instance.name}
				open={isInitialSyncOpen}
				onChoose={handleInitialSyncChoice}
				onCancel={() => setIsInitialSyncOpen(false)}
			/>

			{/* Mounted only while open, so every opening starts from the version picker */}
			{versionOpen && (
				<ChangeVersionDialog
					instance={instance}
					open={versionOpen}
					onOpenChange={setVersionOpen}
					onChanged={async (updated) => {
						setVersionBackup(await rpc.get_instance_version_backup(updated.id).catch(() => null))
						setVersionMsg(null)
						await onSave?.(updated)
					}}
				/>
			)}
		</Dialog>
	)
}

export default InstanceSettingsDialog
