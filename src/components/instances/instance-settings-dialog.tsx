import { Cpu, FolderOpen, HardDrive, Terminal } from "lucide-react"
import { useEffect, useState } from "react"
import type { InstanceConfig } from "@/bindings"
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
import { instanceService } from "@/services/instance-service"
import { useMemorySettings } from "@/services/settings-service"

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
	const totalRamMb = systemMemory?.totalMb || 16384

	const [name, setName] = useState("")
	const [useCustomRam, setUseCustomRam] = useState(false)
	const [minRamMb, setMinRamMb] = useState(2048)
	const [maxRamMb, setMaxRamMb] = useState(4096)
	const [jvmArgsStr, setJvmArgsStr] = useState("")
	const [javaPath, setJavaPath] = useState("")
	const [isSaving, setIsSaving] = useState(false)

	useEffect(() => {
		if (instance) {
			setName(instance.name)
			const hasCustom = instance.memoryMinMb != null || instance.memoryMaxMb != null
			setUseCustomRam(hasCustom)
			setMinRamMb(instance.memoryMinMb ?? globalMemory.minRamMb)
			setMaxRamMb(instance.memoryMaxMb ?? globalMemory.maxRamMb)
			setJvmArgsStr(instance.jvmArgs ? instance.jvmArgs.join(" ") : "")
			setJavaPath(instance.javaPath || "")
		}
	}, [instance, globalMemory])

	if (!instance) return null

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

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent className="max-h-[90vh] max-w-xl border-border/60 bg-zinc-950 p-6 shadow-2xl backdrop-blur-2xl">
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

				<ScrollArea className="-mx-1 max-h-[65vh] px-1">
					<div className="flex flex-col gap-5 p-1">
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
					</div>
				</ScrollArea>

				<DialogFooter className="mt-4 flex items-center justify-between border-border/40 border-t pt-4 sm:justify-between">
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

					<div className="flex items-center gap-2">
						<Button
							type="button"
							variant="outline"
							size="sm"
							onClick={() => onOpenChange(false)}
							disabled={isSaving}
						>
							Cancel
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
					</div>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	)
}

export default InstanceSettingsDialog
