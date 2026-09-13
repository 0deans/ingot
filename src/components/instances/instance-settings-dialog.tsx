import {
	AlertCircle,
	CheckCircle2,
	Cpu,
	DownloadCloud,
	FolderOpen,
	HardDrive,
	Loader2,
	Monitor,
	RefreshCw,
	Terminal,
	UploadCloud,
} from "lucide-react"
import { useEffect, useState } from "react"
import type { InstanceConfig } from "@/bindings"
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
import { ScrollArea } from "@/components/ui/scroll-area"
import Slider from "@/components/ui/slider"
import { Switch } from "@/components/ui/switch"
import { instanceService } from "@/services/instance-service"
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

const STEP_MB = 256
const MIN_RAM_LIMIT_MB = 512

function mbToGb(mb: number): string {
	return (mb / 1024).toFixed(1)
}

export const InstanceSettingsDialog = ({
	instance,
	open,
	onOpenChange,
	onSave,
}: InstanceSettingsDialogProps) => {
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
			setSyncStatusMsg(`Push failed: ${err}`)
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
			setSyncStatusMsg(`Pull failed: ${err}`)
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

			const updated: InstanceConfig = {
				...instance,
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
			alert(`Failed to save settings: ${e}`)
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
			<DialogContent className="max-h-[90vh] w-full gap-3 border-border/60 bg-zinc-950 p-4 shadow-2xl backdrop-blur-2xl sm:max-w-xl sm:p-5 md:max-w-2xl">
				<DialogHeader className="gap-1.5">
					<div className="flex items-center gap-2.5">
						<LoaderIcon loader={instance.loader} size={24} />
						<DialogTitle className="font-semibold text-lg text-white">
							Instance Settings
						</DialogTitle>
					</div>
					<DialogDescription className="text-muted-foreground text-xs">
						Configure execution parameters and memory limits for{" "}
						<span className="font-medium text-foreground">{instance.name}</span>.
					</DialogDescription>
				</DialogHeader>

				<ScrollArea scrollFade className="-mr-2 max-h-[68vh] pr-2">
					<div className="flex flex-col gap-4 py-1 pr-1">
						{/* Instance Name */}
						<div className="flex flex-col gap-2">
							<label htmlFor="instance-name-input" className="font-medium text-foreground text-xs">
								Instance Name
							</label>
							<Input
								id="instance-name-input"
								value={name}
								onChange={(e) => setName(e.target.value)}
								placeholder="My Minecraft Instance"
								className="h-9 border-zinc-800 bg-zinc-900/80 text-xs"
							/>
						</div>

						{/* Memory Override */}
						<div className="flex flex-col gap-3 rounded-xl border border-zinc-800/80 bg-zinc-900/30 p-4">
							<div className="flex items-center justify-between">
								<div className="flex items-center gap-2">
									<HardDrive className="size-4 text-sky-400" />
									<div>
										<h4 className="font-medium text-foreground text-xs">Memory Allocation (RAM)</h4>
										<p className="text-[11px] text-muted-foreground">
											Override global memory for this instance
										</p>
									</div>
								</div>

								<button
									type="button"
									onClick={() => setUseCustomRam(!useCustomRam)}
									className={`relative inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full border-2 border-transparent transition-colors focus-visible:outline-none ${
										useCustomRam ? "bg-primary" : "bg-zinc-800"
									}`}
								>
									<span
										className={`pointer-events-none inline-block size-4 rounded-full bg-white shadow-lg ring-0 transition-transform ${
											useCustomRam ? "translate-x-4" : "translate-x-0"
										}`}
									/>
								</button>
							</div>

							{useCustomRam ? (
								<div className="mt-2 flex flex-col gap-3 pt-2">
									{/* Preset Buttons */}
									<div className="flex flex-wrap gap-1.5">
										{[
											{ label: "2 - 4 GB", min: 2048, max: 4096 },
											{ label: "4 - 6 GB", min: 4096, max: 6144 },
											{ label: "6 - 8 GB", min: 6144, max: 8192 },
											{ label: "8 - 12 GB", min: 8192, max: 12288 },
										].map((p) => (
											<button
												key={p.label}
												type="button"
												onClick={() => applyPreset(p.min, p.max)}
												className="rounded-md border border-zinc-800 bg-zinc-900/80 px-2.5 py-1 text-[11px] text-zinc-300 transition-colors hover:border-zinc-700 hover:text-white"
											>
												{p.label}
											</button>
										))}
									</div>

									{/* RAM Sliders */}
									<div className="flex flex-col gap-3 rounded-lg border border-zinc-800/60 bg-zinc-950/60 p-3">
										<div className="flex items-center justify-between text-xs">
											<span className="text-muted-foreground">Initial Memory (Min):</span>
											<span className="font-medium font-mono text-foreground">
												{mbToGb(minRamMb)} GB ({minRamMb} MB)
											</span>
										</div>
										<Slider
											min={MIN_RAM_LIMIT_MB}
											max={totalRamMb}
											step={STEP_MB}
											value={minRamMb}
											onValueChange={(val) => {
												const n = Array.isArray(val) ? val[0] : val
												setMinRamMb(Math.min(n, maxRamMb))
											}}
										/>

										<div className="mt-2 flex items-center justify-between text-xs">
											<span className="text-muted-foreground">Maximum Memory (Max):</span>
											<span className="font-medium font-mono text-foreground">
												{mbToGb(maxRamMb)} GB ({maxRamMb} MB)
											</span>
										</div>
										<Slider
											min={MIN_RAM_LIMIT_MB}
											max={totalRamMb}
											step={STEP_MB}
											value={maxRamMb}
											onValueChange={(val) => {
												const n = Array.isArray(val) ? val[0] : val
												setMaxRamMb(Math.max(n, minRamMb))
											}}
										/>
									</div>
								</div>
							) : (
								<div className="rounded-lg border border-zinc-800/50 bg-zinc-950/40 px-3 py-2.5 text-[11px] text-muted-foreground">
									Inheriting global settings:{" "}
									<strong className="text-foreground">
										{mbToGb(globalMemory.minRamMb)} GB Min / {mbToGb(globalMemory.maxRamMb)} GB Max
									</strong>
									. Configurable on the Settings page.
								</div>
							)}
						</div>

						{/* Custom JVM Arguments */}
						<div className="flex flex-col gap-2 rounded-xl border border-zinc-800/80 bg-zinc-900/30 p-4">
							<div className="flex items-center gap-2">
								<Terminal className="size-4 text-emerald-400" />
								<div>
									<h4 className="font-medium text-foreground text-xs">JVM Arguments</h4>
									<p className="text-[11px] text-muted-foreground">
										Extra Java launch arguments (space separated)
									</p>
								</div>
							</div>

							<Input
								value={jvmArgsStr}
								onChange={(e) => setJvmArgsStr(e.target.value)}
								placeholder="-XX:+UseG1GC -Dminecraft.custom=true"
								className="mt-1 h-9 border-zinc-800 bg-zinc-900/80 font-mono text-xs"
							/>
						</div>

						{/* Custom Java Binary */}
						<div className="flex flex-col gap-2 rounded-xl border border-zinc-800/80 bg-zinc-900/30 p-4">
							<div className="flex items-center gap-2">
								<Cpu className="size-4 text-amber-400" />
								<div>
									<h4 className="font-medium text-foreground text-xs">Custom Java Executable</h4>
									<p className="text-[11px] text-muted-foreground">
										Leave blank for auto-managed Adoptium runtime
									</p>
								</div>
							</div>

							<Input
								value={javaPath}
								onChange={(e) => setJavaPath(e.target.value)}
								placeholder="C:\Program Files\Java\jdk-21\bin\javaw.exe"
								className="mt-1 h-9 border-zinc-800 bg-zinc-900/80 font-mono text-xs"
							/>
						</div>

						{/* Window & Display Override */}
						<div className="flex flex-col gap-3 rounded-xl border border-zinc-800/80 bg-zinc-900/30 p-4">
							<div className="flex items-center justify-between">
								<div className="flex items-center gap-2">
									<Monitor className="size-4 text-sky-400" />
									<div>
										<h4 className="font-medium text-foreground text-xs">Window & Display</h4>
										<p className="text-[11px] text-muted-foreground">
											Override fullscreen and window size for this instance
										</p>
									</div>
								</div>

								<Switch checked={useCustomWindow} onCheckedChange={setUseCustomWindow} />
							</div>

							{useCustomWindow ? (
								<div className="mt-2 flex flex-col gap-3 pt-1">
									<div className="flex items-center justify-between rounded-lg border border-zinc-800/60 bg-zinc-950/60 p-3">
										<div>
											<span className="font-medium text-foreground text-xs">
												Start in Fullscreen
											</span>
											<p className="text-[11px] text-muted-foreground">
												Launch directly into fullscreen mode
											</p>
										</div>
										<Switch checked={instanceFullscreen} onCheckedChange={setInstanceFullscreen} />
									</div>

									{!instanceFullscreen && (
										<div className="flex flex-col gap-2 rounded-lg border border-zinc-800/60 bg-zinc-950/60 p-3">
											<div className="flex items-center justify-between">
												<span className="text-muted-foreground text-xs">Resolution:</span>
												<div className="flex items-center gap-1.5">
													<Input
														type="number"
														value={instanceWidth}
														onChange={(e) => setInstanceWidth(Number(e.target.value) || 854)}
														className="h-7 w-16 border-zinc-800 bg-zinc-900 px-1 text-center font-mono text-xs"
													/>
													<span className="text-muted-foreground text-xs">×</span>
													<Input
														type="number"
														value={instanceHeight}
														onChange={(e) => setInstanceHeight(Number(e.target.value) || 480)}
														className="h-7 w-16 border-zinc-800 bg-zinc-900 px-1 text-center font-mono text-xs"
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
													<button
														key={p.label}
														type="button"
														onClick={() => {
															setInstanceWidth(p.w)
															setInstanceHeight(p.h)
														}}
														className="rounded border border-zinc-800 bg-zinc-900/80 px-2 py-0.5 text-[10px] text-zinc-300 hover:text-white"
													>
														{p.label}
													</button>
												))}
											</div>
										</div>
									)}
								</div>
							) : (
								<div className="rounded-lg border border-zinc-800/50 bg-zinc-950/40 px-3 py-2 text-[11px] text-muted-foreground">
									Inheriting global display:{" "}
									<strong className="text-foreground">
										{globalWindow.fullscreen
											? "Fullscreen"
											: `${globalWindow.width} × ${globalWindow.height}`}
									</strong>
									. Configurable in Launcher Settings.
								</div>
							)}
						</div>

						{/* Game Data Synchronization */}
						<div className="flex flex-col gap-3 rounded-xl border border-zinc-800/80 bg-zinc-900/30 p-4">
							<div className="flex items-center justify-between">
								<div className="flex items-center gap-2">
									<RefreshCw className="size-4 text-emerald-400" />
									<div>
										<h4 className="font-medium text-foreground text-xs">
											Game Data Synchronization
										</h4>
										<p className="text-[11px] text-muted-foreground">
											Sync options, servers, packs, history, and hotbars
										</p>
									</div>
								</div>

								<Switch checked={useCustomSync} onCheckedChange={handleToggleCustomSync} />
							</div>

							{/* Manual Sync Actions */}
							<div className="flex flex-col gap-2 rounded-lg border border-emerald-500/20 bg-emerald-500/5 p-3">
								<div>
									<div className="font-medium text-foreground text-xs">Manual Storage Sync</div>
									<div className="text-[11px] text-muted-foreground">
										Export this instance's data or pull latest files from shared storage.
									</div>
								</div>

								<div className="flex flex-wrap gap-2 pt-1">
									<Button
										type="button"
										variant="outline"
										size="sm"
										onClick={handlePushSync}
										disabled={isSyncing}
										className="h-8 gap-1.5 border-emerald-600/30 bg-emerald-950/30 text-emerald-300 text-xs hover:bg-emerald-900/40"
									>
										{isSyncing ? (
											<Loader2 className="size-3.5 animate-spin" />
										) : (
											<UploadCloud className="size-3.5" />
										)}
										Push to Shared Storage
									</Button>

									<Button
										type="button"
										variant="outline"
										size="sm"
										onClick={handlePullSync}
										disabled={isSyncing}
										className="h-8 gap-1.5 border-sky-600/30 bg-sky-950/30 text-sky-300 text-xs hover:bg-sky-900/40"
									>
										{isSyncing ? (
											<Loader2 className="size-3.5 animate-spin" />
										) : (
											<DownloadCloud className="size-3.5" />
										)}
										Pull from Shared Storage
									</Button>
								</div>

								{syncStatusMsg && (
									<div
										className={`mt-1 flex items-center gap-1.5 rounded p-2 text-xs ${
											syncStatusType === "success"
												? "bg-emerald-500/10 text-emerald-300"
												: "bg-red-500/10 text-red-300"
										}`}
									>
										{syncStatusType === "success" ? (
											<CheckCircle2 className="size-3.5 shrink-0 text-emerald-400" />
										) : (
											<AlertCircle className="size-3.5 shrink-0 text-red-400" />
										)}
										<span className="text-[11px]">{syncStatusMsg}</span>
									</div>
								)}
							</div>

							{useCustomSync ? (
								<div className="mt-1 flex flex-col divide-y divide-zinc-800/60 rounded-lg border border-zinc-800/60 bg-zinc-950/60">
									{[
										{
											key: "options",
											label: "Sync Game Options (options.txt)",
											checked: syncOptions,
											setChecked: setSyncOptions,
										},
										{
											key: "servers",
											label: "Sync Multiplayer Servers (servers.dat)",
											checked: syncServers,
											setChecked: setSyncServers,
										},
										{
											key: "packs",
											label: "Sync Resource Packs (resourcepacks/)",
											checked: syncResourcePacks,
											setChecked: setSyncResourcePacks,
										},
										{
											key: "history",
											label: "Sync Command History (command_history.txt)",
											checked: syncCommandHistory,
											setChecked: setSyncCommandHistory,
										},
										{
											key: "hotbars",
											label: "Sync Creative Hotbars (hotbar.nbt)",
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
								<div className="rounded-lg border border-zinc-800/50 bg-zinc-950/40 px-3 py-2 text-[11px] text-muted-foreground">
									Inheriting global sync preferences. Will sync before launch and after exit.
								</div>
							)}
						</div>
					</div>
				</ScrollArea>

				<DialogFooter className="mt-2 flex flex-row items-center justify-between border-border/40 border-t pt-3 sm:flex-row sm:justify-between">
					<Button
						type="button"
						variant="ghost"
						size="sm"
						onClick={() => instanceService.openInstanceFolder(instance.id)}
						className="gap-1.5 text-muted-foreground text-xs hover:text-foreground"
					>
						<FolderOpen className="size-3.5" />
						Open Folder
					</Button>

					<Button
						type="button"
						size="sm"
						onClick={handleSave}
						disabled={isSaving}
						className="font-medium"
					>
						{isSaving ? "Saving..." : "Save Changes"}
					</Button>
				</DialogFooter>
			</DialogContent>

			<InitialSyncDialog
				instanceName={instance.name}
				open={isInitialSyncOpen}
				onChoose={handleInitialSyncChoice}
				onCancel={() => setIsInitialSyncOpen(false)}
			/>
		</Dialog>
	)
}

export default InstanceSettingsDialog
