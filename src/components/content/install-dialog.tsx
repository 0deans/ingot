import { AlertCircle, Download, FolderPlus, Loader2 } from "lucide-react"
import { memo, useEffect, useMemo, useState } from "react"
import type { InstanceConfig } from "@/bindings"
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
import { SearchableSelect, type SearchableSelectOption } from "@/components/ui/searchable-select"
import {
	type ContentSource,
	contentService,
	type UnifiedContentItem,
	type UnifiedContentVersion,
} from "@/services/content-service"
import { instanceService } from "@/services/instance-service"

interface InstallDialogProps {
	open: boolean
	onOpenChange: (open: boolean) => void
	item: UnifiedContentItem | null
	specificVersion?: UnifiedContentVersion | null
	onSuccess?: (message: string) => void
}

function formatError(err: unknown): string {
	if (!err) return "Failed to load installation options"
	if (typeof err === "string") return err
	if (err instanceof Error) return err.message
	if (
		typeof err === "object" &&
		err !== null &&
		"message" in err &&
		typeof (err as { message: unknown }).message === "string"
	) {
		return (err as { message: string }).message
	}
	return String(err)
}

export const InstallDialog = memo(
	({ open, onOpenChange, item, specificVersion, onSuccess }: InstallDialogProps) => {
		const [instances, setInstances] = useState<InstanceConfig[]>([])
		const [selectedInstanceId, setSelectedInstanceId] = useState<string>("")
		const [versions, setVersions] = useState<UnifiedContentVersion[]>([])
		const [selectedVersionId, setSelectedVersionId] = useState<string>("")
		const [modpackName, setModpackName] = useState<string>("")

		const [isLoadingData, setIsLoadingData] = useState(false)
		const [isSubmitting, setIsSubmitting] = useState(false)
		const [errorMessage, setErrorMessage] = useState<string | null>(null)

		const isModpack = item?.projectType === "modpack"

		// Load instances & versions when dialog opens
		useEffect(() => {
			if (!open || !item) {
				setErrorMessage(null)
				setIsSubmitting(false)
				return
			}

			setModpackName(item.title)
			setIsLoadingData(true)
			setErrorMessage(null)

			let cancelled = false
			const cleanId = item.id.replace(/^(mr|cf|modrinth|curseforge):/, "")

			// Load existing local instances independently
			instanceService
				.getInstances()
				.then((instList) => {
					if (cancelled) return
					setInstances(instList)
				})
				.catch((err) => {
					console.error("Failed to load instances in InstallDialog:", err)
				})

			// Load content versions independently
			contentService
				.getContentDetails(item.source as ContentSource, cleanId)
				.then((details) => {
					if (cancelled) return
					setVersions(details.versions)

					// If specific version passed, pick it
					if (specificVersion) {
						setSelectedVersionId(specificVersion.id)
					} else if (details.versions.length > 0) {
						setSelectedVersionId(details.versions[0].id)
					}
				})
				.catch((err) => {
					if (cancelled) return
					setErrorMessage(formatError(err))
				})
				.finally(() => {
					if (!cancelled) {
						setIsLoadingData(false)
					}
				})

			return () => {
				cancelled = true
			}
		}, [open, item, specificVersion])

		// Auto-select best compatible instance when instances or versions are loaded
		useEffect(() => {
			if (isModpack || instances.length === 0) return

			if (selectedInstanceId && instances.some((i) => i.id === selectedInstanceId)) {
				return
			}

			if (versions.length > 0) {
				const compInst = instances.find((inst) =>
					versions.some((v) => {
						const matchVer = v.gameVersions.some(
							(gv) => gv.toLowerCase() === inst.gameVersion.toLowerCase(),
						)
						const matchLdr = v.loaders.some((l) => l.toLowerCase() === inst.loader.toLowerCase())
						return matchVer && matchLdr
					}),
				)
				setSelectedInstanceId(compInst ? compInst.id : instances[0].id)
			} else {
				setSelectedInstanceId(instances[0].id)
			}
		}, [instances, versions, isModpack, selectedInstanceId])

		const selectedInstance = useMemo(() => {
			return instances.find((i) => i.id === selectedInstanceId)
		}, [instances, selectedInstanceId])

		// For the selected instance, filter compatible versions
		const compatibleVersions = useMemo(() => {
			if (isModpack || !selectedInstance) return versions
			return versions.filter((v) => {
				const matchVer = v.gameVersions.some(
					(gv) => gv.toLowerCase() === selectedInstance.gameVersion.toLowerCase(),
				)
				const matchLdr = v.loaders.some(
					(l) => l.toLowerCase() === selectedInstance.loader.toLowerCase(),
				)
				return matchVer && matchLdr
			})
		}, [versions, selectedInstance, isModpack])

		// Auto-select version when instance changes if current version is not compatible
		useEffect(() => {
			if (isModpack) return
			if (compatibleVersions.length > 0) {
				const currentIsComp = compatibleVersions.some((v) => v.id === selectedVersionId)
				if (!currentIsComp) {
					setSelectedVersionId(compatibleVersions[0].id)
				}
			}
		}, [compatibleVersions, selectedVersionId, isModpack])

		const selectedVersion = useMemo(() => {
			return versions.find((v) => v.id === selectedVersionId) || versions[0]
		}, [versions, selectedVersionId])

		// Searchable options for Instance Selector
		const instanceOptions: SearchableSelectOption[] = useMemo(() => {
			return instances.map((inst) => {
				const isComp = versions.some((v) => {
					const matchVer = v.gameVersions.some(
						(gv) => gv.toLowerCase() === inst.gameVersion.toLowerCase(),
					)
					const matchLdr = v.loaders.some((l) => l.toLowerCase() === inst.loader.toLowerCase())
					return matchVer && matchLdr
				})
				return {
					value: inst.id,
					label: inst.name,
					badge: `${inst.gameVersion} • ${inst.loader}${isComp ? " • ✓ Compatible" : ""}`,
				}
			})
		}, [instances, versions])

		// Searchable options for Modpack Versions
		const modpackVersionOptions: SearchableSelectOption[] = useMemo(() => {
			return versions.map((ver) => ({
				value: ver.id,
				label: ver.name,
				badge: `${ver.versionType.toUpperCase()} • MC ${ver.gameVersions.slice(0, 2).join(", ") || "All"}`,
			}))
		}, [versions])

		// Searchable options for Mod File Versions
		const singleVersionOptions: SearchableSelectOption[] = useMemo(() => {
			const sourceList = compatibleVersions.length > 0 ? compatibleVersions : versions
			return sourceList.map((ver) => ({
				value: ver.id,
				label: ver.name,
				badge: `${ver.versionType.toUpperCase()} • ${ver.loaders.slice(0, 2).join(", ") || "Any"}`,
			}))
		}, [compatibleVersions, versions])

		const handleInstall = async () => {
			if (!item || !selectedVersion) return
			setIsSubmitting(true)
			setErrorMessage(null)

			try {
				if (isModpack) {
					const name = modpackName.trim() || item.title
					await contentService.installModpackInstance(
						name,
						item.source as ContentSource,
						selectedVersion.downloadUrl,
						selectedVersion.filename,
					)
					await instanceService.refreshInstances()
					onSuccess?.(`Modpack "${name}" installed successfully as a new instance!`)
					onOpenChange(false)
				} else {
					if (!selectedInstanceId) {
						setErrorMessage("Please select a target instance.")
						setIsSubmitting(false)
						return
					}
					await contentService.installContentFile(
						selectedInstanceId,
						item.projectType,
						selectedVersion.downloadUrl,
						selectedVersion.filename,
					)
					const instName = selectedInstance ? selectedInstance.name : "instance"
					onSuccess?.(`Added ${selectedVersion.filename} to ${instName}!`)
					onOpenChange(false)
				}
			} catch (err) {
				setErrorMessage(formatError(err))
			} finally {
				setIsSubmitting(false)
			}
		}

		if (!item) return null

		return (
			<Dialog open={open} onOpenChange={onOpenChange}>
				<DialogContent className="sm:max-w-lg">
					<DialogHeader>
						<DialogTitle className="flex items-center gap-2">
							{isModpack ? (
								<>
									<FolderPlus className="size-5 text-primary" />
									<span>Install Modpack</span>
								</>
							) : (
								<>
									<Download className="size-5 text-primary" />
									<span>Add {item.projectType} to Instance</span>
								</>
							)}
						</DialogTitle>
						<DialogDescription className="text-xs">
							{isModpack
								? `Create a new Minecraft instance from "${item.title}".`
								: `Install "${item.title}" into one of your existing instances.`}
						</DialogDescription>
					</DialogHeader>

					{isLoadingData ? (
						<div className="flex flex-col items-center justify-center gap-3 py-10 text-muted-foreground text-xs">
							<Loader2 className="size-6 animate-spin text-primary" />
							<span>Loading options...</span>
						</div>
					) : errorMessage && versions.length === 0 ? (
						<div className="flex flex-col items-center justify-center gap-3 py-8 text-center">
							<AlertCircle className="size-8 text-destructive" />
							<p className="font-medium text-destructive text-sm">{errorMessage}</p>
							<Button
								size="sm"
								variant="outline"
								onClick={() => {
									setIsLoadingData(true)
									setErrorMessage(null)
									const cleanId = item.id.replace(/^(mr|cf|modrinth|curseforge):/, "")
									contentService
										.getContentDetails(item.source as ContentSource, cleanId)
										.then((details) => {
											setVersions(details.versions)
											if (details.versions.length > 0) {
												setSelectedVersionId(details.versions[0].id)
											}
										})
										.catch((err) => setErrorMessage(formatError(err)))
										.finally(() => setIsLoadingData(false))
								}}
								className="text-xs"
							>
								Retry
							</Button>
						</div>
					) : (
						<div className="flex flex-col gap-4 py-2">
							{/* Item summary banner */}
							<div className="flex items-center gap-3 rounded-lg border border-border/40 bg-zinc-900/50 p-3">
								{item.iconUrl ? (
									<img
										src={item.iconUrl}
										alt={item.title}
										className="size-10 shrink-0 rounded-lg bg-zinc-800 object-cover"
									/>
								) : (
									<div className="flex size-10 shrink-0 items-center justify-center rounded-lg bg-zinc-800 font-bold text-xs">
										{item.title.charAt(0).toUpperCase()}
									</div>
								)}
								<div className="flex min-w-0 flex-1 flex-col">
									<h4 className="truncate font-semibold text-foreground text-sm">{item.title}</h4>
									<span className="text-muted-foreground text-xs">
										by {item.author} • {item.source}
									</span>
								</div>
							</div>

							{isModpack ? (
								/* Modpack Flow: Instance Name + Version Selection */
								<div className="flex flex-col gap-3">
									<div className="flex flex-col gap-1.5">
										<label htmlFor="modpack-name" className="font-medium text-foreground text-xs">
											Instance Name
										</label>
										<Input
											id="modpack-name"
											value={modpackName}
											onChange={(e) => setModpackName(e.target.value)}
											placeholder="Instance Name"
											className="h-9 text-xs"
										/>
									</div>

									<div className="flex flex-col gap-1.5">
										<span className="font-medium text-foreground text-xs">Modpack Version</span>
										<SearchableSelect
											value={selectedVersionId}
											onValueChange={setSelectedVersionId}
											options={modpackVersionOptions}
											placeholder="Select modpack version..."
											searchPlaceholder="Search versions..."
										/>
									</div>

									{selectedVersion && (
										<div className="flex flex-wrap items-center gap-2 rounded-lg border border-border/30 bg-zinc-950/40 p-2.5 text-muted-foreground text-xs">
											<span>Loaders:</span>
											{selectedVersion.loaders.map((l) => (
												<span
													key={l}
													className="rounded bg-zinc-800 px-1.5 py-0.5 font-mono text-[10px] text-zinc-300 capitalize"
												>
													{l}
												</span>
											))}
											<span className="ml-2">Minecraft:</span>
											{selectedVersion.gameVersions.map((gv) => (
												<span
													key={gv}
													className="rounded bg-primary/15 px-1.5 py-0.5 font-mono text-[10px] text-primary"
												>
													{gv}
												</span>
											))}
										</div>
									)}
								</div>
							) : (
								/* Single Mod/Resourcepack/Shader Flow: Select Instance */
								<div className="flex flex-col gap-3">
									{instances.length === 0 ? (
										<div className="flex flex-col items-center justify-center gap-2 rounded-lg border border-amber-500/30 bg-amber-500/10 p-4 text-center">
											<AlertCircle className="size-6 text-amber-400" />
											<p className="font-medium text-amber-300 text-xs">
												No Minecraft instances found
											</p>
											<p className="text-[11px] text-muted-foreground">
												Create an instance first from the Instances tab before adding mods or
												resource packs.
											</p>
										</div>
									) : (
										<>
											<div className="flex flex-col gap-1.5">
												<span className="font-medium text-foreground text-xs">Target Instance</span>
												<SearchableSelect
													value={selectedInstanceId}
													onValueChange={setSelectedInstanceId}
													options={instanceOptions}
													placeholder="Select target instance..."
													searchPlaceholder="Search instances..."
												/>
											</div>

											{/* Select Version */}
											<div className="flex flex-col gap-1.5">
												<div className="flex items-center justify-between">
													<span className="font-medium text-foreground text-xs">File Version</span>
													{compatibleVersions.length > 0 && (
														<span className="text-[11px] text-emerald-400">
															✓ {compatibleVersions.length} compatible version(s)
														</span>
													)}
												</div>

												<SearchableSelect
													value={selectedVersionId}
													onValueChange={setSelectedVersionId}
													options={singleVersionOptions}
													placeholder="Select file version..."
													searchPlaceholder="Search versions..."
												/>
											</div>

											{selectedVersion && (
												<div className="flex flex-col gap-1 rounded-lg border border-border/30 bg-zinc-950/40 p-2.5 text-muted-foreground text-xs">
													<div className="flex items-center justify-between">
														<span className="font-mono text-[11px] text-foreground">
															{selectedVersion.filename}
														</span>
														<span className="capitalize">{selectedVersion.versionType}</span>
													</div>
													<div className="mt-1 flex flex-wrap items-center gap-1.5">
														<span>Supports:</span>
														{selectedVersion.loaders.map((l) => (
															<span
																key={l}
																className="rounded bg-zinc-800 px-1.5 py-0.5 font-mono text-[10px] text-zinc-300 capitalize"
															>
																{l}
															</span>
														))}
														{selectedVersion.gameVersions.map((gv) => (
															<span
																key={gv}
																className="rounded bg-primary/10 px-1.5 py-0.5 font-mono text-[10px] text-primary"
															>
																{gv}
															</span>
														))}
													</div>
												</div>
											)}
										</>
									)}
								</div>
							)}

							{errorMessage && (
								<div className="rounded-lg border border-destructive/30 bg-destructive/10 p-2.5 text-destructive text-xs">
									{errorMessage}
								</div>
							)}
						</div>
					)}

					<DialogFooter className="gap-2 sm:justify-end">
						<Button
							type="button"
							variant="outline"
							disabled={isSubmitting}
							onClick={() => onOpenChange(false)}
							className="text-xs"
						>
							Cancel
						</Button>
						<Button
							type="button"
							disabled={
								isLoadingData ||
								isSubmitting ||
								(!isModpack && instances.length === 0) ||
								!selectedVersion
							}
							onClick={handleInstall}
							className="gap-1.5 font-semibold text-xs"
						>
							{isSubmitting ? (
								<>
									<Loader2 className="size-3.5 animate-spin" />
									<span>Installing...</span>
								</>
							) : (
								<>
									<Download className="size-3.5" />
									<span>{isModpack ? "Create & Install" : "Install to Instance"}</span>
								</>
							)}
						</Button>
					</DialogFooter>
				</DialogContent>
			</Dialog>
		)
	},
)

InstallDialog.displayName = "InstallDialog"
