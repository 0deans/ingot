import i18n from "i18next"
import { AlertCircle, Download, FolderPlus } from "lucide-react"
import { memo, useEffect, useMemo, useState } from "react"
import { useTranslation } from "react-i18next"
import type { InstanceConfig } from "@/bindings"
import {
	SearchableSelect,
	type SearchableSelectOption,
} from "@/components/common/searchable-select"
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
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Spinner } from "@/components/ui/spinner"
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

export const supportsMods = (loader: string): boolean => {
	const l = loader.toLowerCase()
	return l === "fabric" || l === "quilt" || l === "neoforge" || l === "forge"
}

export const isVersionCompatibleWithInstance = (
	instance: InstanceConfig,
	version: UnifiedContentVersion,
	projectType: string,
): boolean => {
	const matchGameVer = version.gameVersions.some((gv) => {
		return gv.toLowerCase() === instance.gameVersion.toLowerCase()
	})
	if (!matchGameVer) {
		return false
	}

	if (projectType === "mod") {
		if (!supportsMods(instance.loader)) {
			return false
		}
		const instLoader = instance.loader.toLowerCase()
		return version.loaders.some((l) => {
			const vl = l.toLowerCase()
			if (vl === instLoader) {
				return true
			}
			if (instLoader === "quilt" && vl === "fabric") {
				return true
			}
			if (instLoader === "neoforge" && vl === "forge" && instance.gameVersion === "1.20.1") {
				return true
			}
			return false
		})
	}

	return true
}

function formatError(err: unknown): string {
	if (!err) return i18n.t("install.loadFailed")
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
		const { t } = useTranslation()
		const [instances, setInstances] = useState<InstanceConfig[]>([])
		const [selectedInstanceId, setSelectedInstanceId] = useState<string>("")
		const [versions, setVersions] = useState<UnifiedContentVersion[]>([])
		const [selectedVersionId, setSelectedVersionId] = useState<string>("")
		const [modpackName, setModpackName] = useState<string>("")

		const [isLoadingData, setIsLoadingData] = useState(false)
		const [isSubmitting, setIsSubmitting] = useState(false)
		const [errorMessage, setErrorMessage] = useState<string | null>(null)

		const isModpack = item?.projectType === "modpack"
		const isMod = item?.projectType === "mod"

		const [prevOpen, setPrevOpen] = useState(open)
		const [prevItem, setPrevItem] = useState(item)

		if ((open && !prevOpen) || (item && item !== prevItem)) {
			setPrevOpen(open)
			setPrevItem(item)
			setErrorMessage(null)
			setIsSubmitting(false)
			if (item) {
				setModpackName(item.title)
				setIsLoadingData(true)
			}
		} else if (!open && prevOpen) {
			setPrevOpen(false)
			setErrorMessage(null)
			setIsSubmitting(false)
		}

		// Load instances & versions when dialog opens
		useEffect(() => {
			if (!open || !item) {
				return
			}

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

		if (!isModpack && instances.length > 0) {
			const hasSelected =
				selectedInstanceId &&
				instances.some((i) => {
					return i.id === selectedInstanceId
				})
			if (!hasSelected) {
				if (versions.length > 0) {
					// 1. Try to find an instance compatible with at least one version
					const compInst = instances.find((inst) => {
						return versions.some((v) => {
							return isVersionCompatibleWithInstance(inst, v, item?.projectType ?? "")
						})
					})
					// 2. Fall back to an instance that supports mods (or first instance)
					const fallbackInst = isMod
						? (instances.find((inst) => {
								return supportsMods(inst.loader)
							}) ?? instances[0])
						: instances[0]

					setSelectedInstanceId(compInst ? compInst.id : fallbackInst.id)
				} else {
					const fallbackInst = isMod
						? (instances.find((inst) => {
								return supportsMods(inst.loader)
							}) ?? instances[0])
						: instances[0]
					setSelectedInstanceId(fallbackInst.id)
				}
			}
		}

		const selectedInstance = useMemo(() => {
			return instances.find((i) => {
				return i.id === selectedInstanceId
			})
		}, [instances, selectedInstanceId])

		// For the selected instance, filter compatible versions
		const compatibleVersions = useMemo(() => {
			if (isModpack || !selectedInstance) {
				return versions
			}
			return versions.filter((v) => {
				return isVersionCompatibleWithInstance(selectedInstance, v, item?.projectType ?? "")
			})
		}, [versions, selectedInstance, isModpack, item?.projectType])

		if (!isModpack && compatibleVersions.length > 0) {
			const currentIsComp = compatibleVersions.some((v) => {
				return v.id === selectedVersionId
			})
			if (!currentIsComp) {
				setSelectedVersionId(compatibleVersions[0].id)
			}
		}

		const selectedVersion = useMemo(() => {
			if (isModpack) {
				return (
					versions.find((v) => {
						return v.id === selectedVersionId
					}) ?? (versions.length > 0 ? versions[0] : null)
				)
			}
			if (compatibleVersions.length === 0) {
				return null
			}
			return (
				compatibleVersions.find((v) => {
					return v.id === selectedVersionId
				}) ?? compatibleVersions[0]
			)
		}, [versions, selectedVersionId, isModpack, compatibleVersions])

		// Searchable options for Instance Selector
		const instanceOptions: SearchableSelectOption[] = useMemo(() => {
			return instances.map((inst) => {
				const hasModSupport = !isMod || supportsMods(inst.loader)
				const isComp =
					hasModSupport &&
					versions.some((v) => {
						return isVersionCompatibleWithInstance(inst, v, item?.projectType ?? "")
					})

				let badge = `${inst.gameVersion} • ${inst.loader}`
				if (isMod && !supportsMods(inst.loader)) {
					badge += ` • ✗ ${t("install.noModSupport")}`
				} else if (isComp) {
					badge += ` • ✓ ${t("install.compatible")}`
				} else {
					badge += ` • ✗ ${t("install.incompatible")}`
				}

				return {
					value: inst.id,
					label: inst.name,
					badge,
					disabled: isMod && !supportsMods(inst.loader),
				}
			})
		}, [instances, versions, isMod, item?.projectType, t])

		// Searchable options for Modpack Versions
		const modpackVersionOptions: SearchableSelectOption[] = useMemo(() => {
			return versions.map((ver) => {
				return {
					value: ver.id,
					label: ver.name,
					badge: `${ver.versionType.toUpperCase()} • MC ${ver.gameVersions.slice(0, 2).join(", ") || t("install.all")}`,
				}
			})
		}, [versions, t])

		// Searchable options for Mod File Versions
		const singleVersionOptions: SearchableSelectOption[] = useMemo(() => {
			return versions.map((ver) => {
				const isComp = selectedInstance
					? isVersionCompatibleWithInstance(selectedInstance, ver, item?.projectType ?? "")
					: true
				return {
					value: ver.id,
					label: ver.name,
					badge: `${ver.versionType.toUpperCase()} • ${ver.loaders.slice(0, 2).join(", ") || t("install.any")}${!isComp ? ` • ✗ ${t("install.incompatible")}` : ""}`,
					disabled: !isComp,
				}
			})
		}, [versions, selectedInstance, item?.projectType, t])

		const handleInstall = async () => {
			if (!item) {
				return
			}
			if (!isModpack) {
				if (!selectedInstance) {
					setErrorMessage(t("install.selectTarget"))
					return
				}
				if (isMod && !supportsMods(selectedInstance.loader)) {
					setErrorMessage(
						t("install.unsupportedLoaderWarning", {
							instance: selectedInstance.name,
							loader: selectedInstance.loader,
						}),
					)
					return
				}
				if (
					!selectedVersion ||
					!isVersionCompatibleWithInstance(selectedInstance, selectedVersion, item.projectType)
				) {
					setErrorMessage(
						t("install.versionIncompatibleWarning", {
							instance: selectedInstance.name,
							version: selectedInstance.gameVersion,
							loader: selectedInstance.loader,
						}),
					)
					return
				}
			}
			if (!selectedVersion) {
				return
			}

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
					onSuccess?.(t("install.modpackDone", { name }))
					onOpenChange(false)
				} else {
					await contentService.installContentFile(
						selectedInstanceId,
						item.projectType,
						selectedVersion.downloadUrl,
						selectedVersion.filename,
					)
					const instName = selectedInstance ? selectedInstance.name : ""
					onSuccess?.(t("install.added", { file: selectedVersion.filename, instance: instName }))
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
									<span>{t("install.modpackTitle")}</span>
								</>
							) : (
								<>
									<Download className="size-5 text-primary" />
									<span>{t(`install.addTitle_${item.projectType}` as "install.addTitle_mod")}</span>
								</>
							)}
						</DialogTitle>
						<DialogDescription className="text-xs">
							{isModpack
								? t("install.modpackDescription", { title: item.title })
								: t("install.description", { title: item.title })}
						</DialogDescription>
					</DialogHeader>

					{isLoadingData ? (
						<div className="flex flex-col items-center justify-center gap-3 py-10 text-xs text-muted-foreground">
							<Spinner className="size-6 text-primary" />
							<span>{t("install.loading")}</span>
						</div>
					) : errorMessage && versions.length === 0 ? (
						<div className="flex flex-col items-center justify-center gap-3 py-8 text-center">
							<AlertCircle className="size-8 text-destructive" />
							<p className="text-sm font-medium text-destructive">{errorMessage}</p>
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
								{t("install.retry")}
							</Button>
						</div>
					) : (
						<div className="flex flex-col gap-4 py-2">
							{/* Item summary banner */}
							<div className="flex items-center gap-3 rounded-lg border border-border/40 bg-card/50 p-3">
								{item.iconUrl ? (
									<img
										src={item.iconUrl}
										alt=""
										className="size-10 shrink-0 rounded-lg bg-muted object-cover"
									/>
								) : (
									<div className="flex size-10 shrink-0 items-center justify-center rounded-lg bg-muted text-xs font-bold">
										{item.title.charAt(0).toUpperCase()}
									</div>
								)}
								<div className="flex min-w-0 flex-1 flex-col">
									<h4 className="truncate text-sm font-semibold text-foreground">{item.title}</h4>
									<span className="text-xs text-muted-foreground">
										{t("modpacks.byAuthor", { author: item.author })} • {item.source}
									</span>
								</div>
							</div>

							{isModpack ? (
								/* Modpack Flow: Instance Name + Version Selection */
								<div className="flex flex-col gap-3">
									<div className="flex flex-col gap-1.5">
										<Label htmlFor="modpack-name" className="text-xs font-medium text-foreground">
											{t("newInstance.name")}
										</Label>
										<Input
											id="modpack-name"
											value={modpackName}
											onChange={(e) => setModpackName(e.target.value)}
											placeholder={t("newInstance.name")}
											className="h-9 text-xs"
										/>
									</div>

									<div className="flex flex-col gap-1.5">
										<span className="text-xs font-medium text-foreground">
											{t("install.modpackVersion")}
										</span>
										<SearchableSelect
											value={selectedVersionId}
											onValueChange={setSelectedVersionId}
											options={modpackVersionOptions}
											placeholder={t("install.selectModpackVersion")}
											searchPlaceholder={t("versionChange.searchVersion")}
										/>
									</div>

									{selectedVersion && (
										<div className="flex flex-wrap items-center gap-2 rounded-lg border border-border/30 bg-background/40 p-2.5 text-xs text-muted-foreground">
											<span>{t("install.loaders")}</span>
											{selectedVersion.loaders.map((l) => (
												<Badge key={l} variant="secondary" className="font-mono capitalize">
													{l}
												</Badge>
											))}
											<span className="ml-2">{t("install.minecraft")}</span>
											{selectedVersion.gameVersions.map((gv) => (
												<Badge key={gv} variant="outline" className="font-mono">
													{gv}
												</Badge>
											))}
										</div>
									)}
								</div>
							) : (
								/* Single Mod/Resourcepack/Shader Flow: Select Instance */
								<div className="flex flex-col gap-3">
									{instances.length === 0 ? (
										<div className="flex flex-col items-center justify-center gap-2 rounded-lg border border-warning/30 bg-warning/10 p-4 text-center">
											<AlertCircle className="size-6 text-warning" />
											<p className="text-xs font-medium text-warning">
												{t("quickJoin.noInstances")}
											</p>
											<p className="text-2xs text-muted-foreground">
												{t("install.noInstancesHint")}
											</p>
										</div>
									) : (
										<>
											<div className="flex flex-col gap-1.5">
												<span className="text-xs font-medium text-foreground">
													{t("install.target")}
												</span>
												<SearchableSelect
													value={selectedInstanceId}
													onValueChange={setSelectedInstanceId}
													options={instanceOptions}
													placeholder={t("install.selectTarget")}
													searchPlaceholder={t("instances.searchPlaceholder")}
												/>
											</div>

											{/* Select Version */}
											<div className="flex flex-col gap-1.5">
												<div className="flex items-center justify-between">
													<span className="text-xs font-medium text-foreground">
														{t("install.fileVersion")}
													</span>
													{compatibleVersions.length > 0 && (
														<span className="text-2xs text-primary">
															✓ {t("install.compatibleCount", { count: compatibleVersions.length })}
														</span>
													)}
												</div>

												<SearchableSelect
													value={selectedVersionId}
													onValueChange={setSelectedVersionId}
													options={singleVersionOptions}
													placeholder={t("install.selectFileVersion")}
													searchPlaceholder={t("versionChange.searchVersion")}
												/>
											</div>

											{selectedVersion && (
												<div className="flex flex-col gap-1 rounded-lg border border-border/30 bg-background/40 p-2.5 text-xs text-muted-foreground">
													<div className="flex items-center justify-between">
														<span className="font-mono text-2xs text-foreground">
															{selectedVersion.filename}
														</span>
														<span className="capitalize">{selectedVersion.versionType}</span>
													</div>
													<div className="mt-1 flex flex-wrap items-center gap-1.5">
														<span>{t("install.supports")}</span>
														{selectedVersion.loaders.map((l) => (
															<Badge key={l} variant="secondary" className="font-mono capitalize">
																{l}
															</Badge>
														))}
														{selectedVersion.gameVersions.map((gv) => (
															<Badge key={gv} variant="outline" className="font-mono">
																{gv}
															</Badge>
														))}
													</div>
												</div>
											)}
											{selectedInstance && isMod && !supportsMods(selectedInstance.loader) && (
												<Alert variant="destructive">
													<AlertCircle className="size-4 shrink-0" />
													<AlertDescription>
														{t("install.unsupportedLoaderWarning", {
															instance: selectedInstance.name,
															loader: selectedInstance.loader,
														})}
													</AlertDescription>
												</Alert>
											)}

											{selectedInstance &&
												(!isMod || supportsMods(selectedInstance.loader)) &&
												compatibleVersions.length === 0 && (
													<Alert variant="destructive">
														<AlertCircle className="size-4 shrink-0" />
														<AlertDescription>
															{t("install.noCompatibleVersionWarning", {
																instance: selectedInstance.name,
																version: selectedInstance.gameVersion,
																loader: selectedInstance.loader,
															})}
														</AlertDescription>
													</Alert>
												)}
										</>
									)}
								</div>
							)}

							{errorMessage && (
								<Alert variant="destructive">
									<AlertDescription>{errorMessage}</AlertDescription>
								</Alert>
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
							{t("common.cancel")}
						</Button>
						<Button
							type="button"
							disabled={
								isLoadingData ||
								isSubmitting ||
								(!isModpack &&
									(instances.length === 0 ||
										!selectedInstance ||
										(isMod && !supportsMods(selectedInstance.loader)) ||
										!selectedVersion))
							}
							onClick={handleInstall}
							className="gap-1.5 text-xs font-semibold"
						>
							{isSubmitting ? (
								<>
									<Spinner className="size-3.5" />
									<span>{t("pluginDetails.installing")}</span>
								</>
							) : (
								<>
									<Download className="size-3.5" />
									<span>
										{isModpack ? t("install.createAndInstall") : t("install.installToInstance")}
									</span>
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
