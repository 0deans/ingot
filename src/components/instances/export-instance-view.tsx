import { save as saveDialog } from "@tauri-apps/plugin-dialog"
import { AlertCircle, ArrowLeft, CheckCircle2, Download } from "lucide-react"
import { memo, useCallback, useEffect, useMemo, useState } from "react"
import { useTranslation } from "react-i18next"
import type {
	EnvSupport,
	ExportFormat,
	ExportOptions,
	ExportPreview,
	ExportReport,
	FileSide,
	InstanceConfig,
} from "@/bindings"
import ExportFileSides from "@/components/instances/export-file-sides"
import ExportFileTree from "@/components/instances/export-file-tree"
import InstanceIcon from "@/components/instances/instance-icon"
import LoaderIcon from "@/components/instances/loader-icon"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Spinner } from "@/components/ui/spinner"
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Textarea } from "@/components/ui/textarea"
import { canShareFiles, shareFile } from "@/lib/share"
import { instanceService } from "@/services/instance-service"

export interface ExportInstanceViewProps {
	instance: InstanceConfig
	isInstanceRunning: boolean
	onBack: () => void
}

const ExportInstanceView = ({ instance, isInstanceRunning, onBack }: ExportInstanceViewProps) => {
	// 1. Context & Global
	const { t } = useTranslation()

	// 2. Refs (none)

	// 3. State
	const [format, setFormat] = useState<ExportFormat>("modrinth")
	const [name, setName] = useState(instance.name)
	const [version, setVersion] = useState("1.0.0")
	const [author, setAuthor] = useState("")
	const [summary, setSummary] = useState("")
	const [preview, setPreview] = useState<ExportPreview | null>(null)
	const [selectedPaths, setSelectedPaths] = useState<Set<string>>(() => {
		return new Set<string>()
	})
	const [fileSides, setFileSides] = useState<
		Map<string, { client: EnvSupport; server: EnvSupport }>
	>(() => {
		return new Map()
	})
	const [activeTab, setActiveTab] = useState<"files" | "sides">("files")
	const [isLoadingPreview, setIsLoadingPreview] = useState(true)
	const [busy, setBusy] = useState(false)
	const [error, setError] = useState<string | null>(null)
	const [report, setReport] = useState<ExportReport | null>(null)

	// 4. Memoization
	const effectiveTab = format === "modrinth" ? activeTab : "files"

	const candidatePaths = useMemo(() => {
		if (!preview) return []
		return preview.entries
			.filter((e) => {
				const lower = e.path.toLowerCase()
				return (
					(lower.startsWith("mods/") ||
						lower.startsWith("resourcepacks/") ||
						lower.startsWith("shaderpacks/")) &&
					!lower.endsWith(".disabled") &&
					selectedPaths.has(e.path)
				)
			})
			.map((e) => {
				return e.path
			})
	}, [preview, selectedPaths])

	const handleTogglePath = useCallback((path: string) => {
		setSelectedPaths((prev) => {
			const next = new Set(prev)
			if (next.has(path)) {
				next.delete(path)
			} else {
				next.add(path)
			}
			return next
		})
	}, [])

	const handleToggleAll = useCallback(
		(selectAll: boolean) => {
			if (!preview) return
			if (selectAll) {
				const all = new Set(
					preview.entries.map((e) => {
						return e.path
					}),
				)
				setSelectedPaths(all)
			} else {
				setSelectedPaths(new Set())
			}
		},
		[preview],
	)

	const handleChangeSide = useCallback((path: string, client: EnvSupport, server: EnvSupport) => {
		setFileSides((prev) => {
			const next = new Map(prev)
			next.set(path, { client, server })
			return next
		})
	}, [])

	const handleFormatChange = useCallback((val: string) => {
		if (val === "modrinth" || val === "curseForge" || val === "prism") {
			setFormat(val)
			if (val !== "modrinth") {
				setActiveTab("files")
			}
		}
	}, [])

	const handleTabChange = useCallback(
		(val: string) => {
			if (val === "files" || (val === "sides" && format === "modrinth")) {
				setActiveTab(val)
			}
		},
		[format],
	)

	// 5. Effects
	useEffect(() => {
		let active = true

		instanceService
			.previewExport(instance.id)
			.then((p) => {
				if (!active) return
				setPreview(p)
				setName(p.defaultName)
				setVersion(p.defaultVersion)
				const defaults = new Set(
					p.entries
						.filter((e) => {
							return e.defaultIncluded
						})
						.map((e) => {
							return e.path
						}),
				)
				setSelectedPaths(defaults)
			})
			.catch((err: unknown) => {
				if (!active) return
				setError(err instanceof Error ? err.message : String(err))
			})
			.finally(() => {
				if (active) {
					setIsLoadingPreview(false)
				}
			})

		return () => {
			active = false
		}
	}, [instance.id])

	const handleExport = async () => {
		const trimmedName = name.trim()
		if (!trimmedName) {
			setError(t("exportInstance.nameRequired"))
			return
		}

		setBusy(true)
		setError(null)

		try {
			const extension = format === "modrinth" ? "mrpack" : "zip"
			const filterName =
				format === "modrinth"
					? t("exportInstance.formatModrinth")
					: format === "curseForge"
						? t("exportInstance.formatCurseForge")
						: t("exportInstance.formatPrism")

			let destination: string | null = null

			if (canShareFiles()) {
				destination = null
			} else {
				const sanitizedName = trimmedName.replace(/[^\w-]+/g, "_")
				const defaultFilename = `${sanitizedName}-${version.trim()}.${extension}`
				const chosen = await saveDialog({
					defaultPath: defaultFilename,
					filters: [{ name: filterName, extensions: [extension] }],
				})

				if (!chosen) {
					setBusy(false)
					return
				}
				destination = chosen
			}

			const sidesArray: FileSide[] = []
			for (const [p, sides] of fileSides.entries()) {
				if (selectedPaths.has(p)) {
					sidesArray.push({
						path: p,
						client: sides.client,
						server: sides.server,
					})
				}
			}

			const options: ExportOptions = {
				format,
				name: trimmedName,
				version: version.trim() || "1.0.0",
				author: author.trim() ? author.trim() : null,
				summary: summary.trim() ? summary.trim() : null,
				includedPaths: Array.from(selectedPaths),
				fileSides: sidesArray,
			}

			const res = await instanceService.exportInstance(instance.id, options, destination)

			if (canShareFiles()) {
				const mimeType =
					format === "modrinth" ? "application/x-modrinth-modpack+zip" : "application/zip"
				shareFile(res.outputPath, mimeType, trimmedName)
			}

			setReport(res)
		} catch (err: unknown) {
			setError(err instanceof Error ? err.message : String(err))
		} finally {
			setBusy(false)
		}
	}

	return (
		<div className="flex h-full min-h-0 w-full flex-col overflow-hidden bg-background">
			{/* Top Header Bar */}
			<div className="flex shrink-0 items-center justify-between gap-3 border-b border-border/50 bg-card/40 px-4 py-3 sm:px-6">
				<div className="flex min-w-0 items-center gap-2.5 sm:gap-3">
					<Button
						variant="ghost"
						size="sm"
						onClick={onBack}
						className="gap-1.5 px-2 text-xs font-medium text-muted-foreground hover:text-foreground sm:px-2.5"
						aria-label={t("exportInstance.cancel")}
					>
						<ArrowLeft className="size-4" />
						<span className="hidden sm:inline">{t("exportInstance.cancel")}</span>
					</Button>

					<div className="h-5 w-px bg-border/60" />

					<div className="flex size-9 shrink-0 items-center justify-center rounded-lg border border-border bg-card">
						<InstanceIcon
							icon={instance.icon}
							loader={instance.loader}
							className="size-5 object-contain"
						/>
					</div>

					<div className="flex min-w-0 flex-col">
						<div className="flex min-w-0 items-center gap-2">
							<h1 className="truncate text-sm font-semibold text-foreground">
								{t("exportInstance.title")}
							</h1>
							<Badge
								variant="outline"
								className="shrink-0 text-2xs text-muted-foreground uppercase"
							>
								{instance.loader}
							</Badge>
						</div>
						<div className="flex items-center gap-1.5 text-2xs text-muted-foreground">
							<LoaderIcon loader={instance.loader} className="size-3" />
							<span>{instance.name}</span>
							<span>•</span>
							<span>{instance.gameVersion}</span>
							{instance.loaderVersion && (
								<>
									<span>•</span>
									<span>{instance.loaderVersion}</span>
								</>
							)}
						</div>
					</div>
				</div>

				<div className="flex shrink-0 items-center gap-2">
					{report ? (
						<Button type="button" size="sm" onClick={onBack} className="text-xs">
							{t("exportInstance.done")}
						</Button>
					) : (
						<Button
							type="button"
							size="sm"
							onClick={() => {
								void handleExport()
							}}
							disabled={busy || isInstanceRunning || isLoadingPreview || selectedPaths.size === 0}
							className="text-xs"
						>
							{busy ? (
								<>
									<Spinner className="mr-1.5 size-3.5" />
									<span>{t("exportInstance.exporting")}</span>
								</>
							) : (
								<>
									<Download className="mr-1.5 size-3.5" />
									<span>{t("exportInstance.exportButton")}</span>
								</>
							)}
						</Button>
					)}
				</div>
			</div>

			{/* Main Content Area */}
			<ScrollArea className="size-full flex-1 scroll-fade-y">
				<div className="p-4 sm:p-6">
					{isInstanceRunning && (
						<Alert variant="destructive" className="mb-4">
							<AlertCircle className="size-4" />
							<AlertTitle>{t("exportInstance.runningWarningTitle")}</AlertTitle>
							<AlertDescription>{t("exportInstance.runningWarningDescription")}</AlertDescription>
						</Alert>
					)}

					{error && (
						<Alert variant="destructive" className="mb-4">
							<AlertCircle className="size-4" />
							<AlertDescription>{error}</AlertDescription>
						</Alert>
					)}

					{isLoadingPreview ? (
						<div className="flex flex-1 flex-col items-center justify-center gap-3 py-24">
							<Spinner className="size-7 text-primary" />
							<span className="text-xs text-muted-foreground">
								{t("exportInstance.scanningFiles")}
							</span>
						</div>
					) : report ? (
						<div className="mx-auto flex w-full max-w-2xl flex-col gap-5 py-6">
							<Alert className="border-emerald-500/50 bg-emerald-500/10 text-emerald-500">
								<CheckCircle2 className="size-5 text-emerald-500" />
								<AlertTitle className="text-sm font-semibold">
									{t("exportInstance.exportSuccessTitle")}
								</AlertTitle>
								<AlertDescription className="mt-1 text-xs text-foreground">
									{t("exportInstance.savedTo", { path: report.outputPath })}
								</AlertDescription>
							</Alert>

							<div className="grid grid-cols-3 gap-3 rounded-lg border border-border bg-card/60 p-4 text-center">
								<div>
									<span className="block text-xs font-medium text-muted-foreground">
										{t("exportInstance.totalFiles")}
									</span>
									<span className="text-lg font-bold text-foreground">{report.totalFiles}</span>
								</div>
								<div>
									<span className="block text-xs font-medium text-muted-foreground">
										{t("exportInstance.resolvedFiles")}
									</span>
									<span className="text-lg font-bold text-emerald-500">{report.resolvedFiles}</span>
								</div>
								<div>
									<span className="block text-xs font-medium text-muted-foreground">
										{t("exportInstance.embeddedFiles")}
									</span>
									<span className="text-lg font-bold text-amber-500">{report.embeddedFiles}</span>
								</div>
							</div>

							{report.embeddedWarnings.length > 0 && (
								<Card className="border-amber-500/30 bg-amber-500/5">
									<CardHeader className="pb-2">
										<CardTitle className="text-xs font-semibold text-amber-500">
											{t("exportInstance.embeddedWarningsTitle")}
										</CardTitle>
									</CardHeader>
									<CardContent>
										<ScrollArea className="h-40 scroll-fade-y pr-2">
											<ul className="flex flex-col gap-1.5 text-xs text-muted-foreground">
												{report.embeddedWarnings.map((warn) => {
													return <li key={warn}>• {warn}</li>
												})}
											</ul>
										</ScrollArea>
									</CardContent>
								</Card>
							)}

							<div className="flex justify-end gap-2 pt-2">
								<Button
									type="button"
									variant="outline"
									size="sm"
									onClick={() => {
										setReport(null)
									}}
								>
									{t("exportInstance.title")}
								</Button>
								<Button type="button" size="sm" onClick={onBack}>
									{t("exportInstance.done")}
								</Button>
							</div>
						</div>
					) : (
						<div className="grid grid-cols-1 items-start gap-6 lg:grid-cols-12">
							{/* Left Column: Format and Metadata */}
							<div className="flex flex-col gap-5 lg:col-span-5">
								{/* Format Selection Card */}
								<Card>
									<CardHeader className="pb-3">
										<CardTitle className="text-xs font-semibold tracking-wider text-muted-foreground uppercase">
											{t("exportInstance.formatLabel")}
										</CardTitle>
									</CardHeader>
									<CardContent className="flex flex-col gap-2.5">
										<RadioGroup
											value={format}
											onValueChange={handleFormatChange}
											className="flex flex-col gap-2"
										>
											<label
												className={`flex cursor-pointer items-center justify-between rounded-lg border p-3 text-xs transition-colors hover:border-primary ${
													format === "modrinth" ? "border-primary bg-primary/10" : "border-border"
												}`}
											>
												<div className="flex items-center gap-2.5 font-medium">
													<RadioGroupItem value="modrinth" id="format-mrpack" />
													<div className="flex flex-col">
														<span>{t("exportInstance.formatModrinth")}</span>
														<span className="text-2xs text-muted-foreground">
															Modrinth index + overrides
														</span>
													</div>
												</div>
												<Badge variant="secondary" className="font-mono text-2xs">
													.mrpack
												</Badge>
											</label>

											<label
												className={`flex cursor-pointer items-center justify-between rounded-lg border p-3 text-xs transition-colors hover:border-primary ${
													format === "curseForge" ? "border-primary bg-primary/10" : "border-border"
												}`}
											>
												<div className="flex items-center gap-2.5 font-medium">
													<RadioGroupItem value="curseForge" id="format-cf" />
													<div className="flex flex-col">
														<span>{t("exportInstance.formatCurseForge")}</span>
														<span className="text-2xs text-muted-foreground">
															manifest.json + modlist.html
														</span>
													</div>
												</div>
												<Badge variant="secondary" className="font-mono text-2xs">
													.zip
												</Badge>
											</label>

											<label
												className={`flex cursor-pointer items-center justify-between rounded-lg border p-3 text-xs transition-colors hover:border-primary ${
													format === "prism" ? "border-primary bg-primary/10" : "border-border"
												}`}
											>
												<div className="flex items-center gap-2.5 font-medium">
													<RadioGroupItem value="prism" id="format-prism" />
													<div className="flex flex-col">
														<span>{t("exportInstance.formatPrism")}</span>
														<span className="text-2xs text-muted-foreground">
															instance.cfg + mmc-pack.json
														</span>
													</div>
												</div>
												<Badge variant="secondary" className="font-mono text-2xs">
													.zip
												</Badge>
											</label>
										</RadioGroup>
									</CardContent>
								</Card>

								{/* Metadata Card */}
								<Card>
									<CardHeader className="pb-3">
										<CardTitle className="text-xs font-semibold tracking-wider text-muted-foreground uppercase">
											{t("exportInstance.packName")}
										</CardTitle>
									</CardHeader>
									<CardContent className="flex flex-col gap-3.5">
										<div className="flex flex-col gap-1.5">
											<Label htmlFor="export-name" className="text-xs font-medium">
												{t("exportInstance.packName")}
											</Label>
											<Input
												id="export-name"
												value={name}
												onChange={(e) => {
													setName(e.target.value)
												}}
												className="h-8 text-xs"
												placeholder={instance.name}
											/>
										</div>

										<div className="flex flex-col gap-1.5">
											<Label htmlFor="export-version" className="text-xs font-medium">
												{t("exportInstance.packVersion")}
											</Label>
											<Input
												id="export-version"
												value={version}
												onChange={(e) => {
													setVersion(e.target.value)
												}}
												className="h-8 text-xs"
												placeholder="1.0.0"
											/>
										</div>

										{format !== "prism" && (
											<>
												<div className="flex flex-col gap-1.5">
													<Label htmlFor="export-author" className="text-xs font-medium">
														{t("exportInstance.packAuthor")}
													</Label>
													<Input
														id="export-author"
														value={author}
														onChange={(e) => {
															setAuthor(e.target.value)
														}}
														className="h-8 text-xs"
														placeholder={t("exportInstance.optional")}
													/>
												</div>

												<div className="flex flex-col gap-1.5">
													<Label htmlFor="export-summary" className="text-xs font-medium">
														{t("exportInstance.packSummary")}
													</Label>
													<Textarea
														id="export-summary"
														value={summary}
														onChange={(e) => {
															setSummary(e.target.value)
														}}
														rows={3}
														className="text-xs"
														placeholder={t("exportInstance.optional")}
													/>
												</div>
											</>
										)}
									</CardContent>
								</Card>
							</div>

							{/* Right Column: Files & Environment Sides */}
							<div className="flex flex-col gap-4 lg:col-span-7">
								{preview && (
									<Card className="flex flex-col">
										<CardHeader className="pb-2">
											<CardTitle className="text-xs font-semibold tracking-wider text-muted-foreground uppercase">
												{t("exportInstance.filesTab")}
											</CardTitle>
										</CardHeader>
										<CardContent>
											<Tabs value={effectiveTab} onValueChange={handleTabChange} className="w-full">
												<TabsList className="grid h-8 w-full grid-cols-2 text-xs">
													<TabsTrigger value="files" className="text-xs">
														{t("exportInstance.filesTab")} ({selectedPaths.size})
													</TabsTrigger>
													<TabsTrigger
														value="sides"
														disabled={format !== "modrinth"}
														className="text-xs"
													>
														{t("exportInstance.sidesTab")}
													</TabsTrigger>
												</TabsList>

												<TabsContent value="files" className="mt-3">
													<ExportFileTree
														entries={preview.entries}
														selectedPaths={selectedPaths}
														onTogglePath={handleTogglePath}
														onToggleAll={handleToggleAll}
													/>
												</TabsContent>

												<TabsContent value="sides" className="mt-3">
													<ExportFileSides
														candidatePaths={candidatePaths}
														fileSides={fileSides}
														onChangeSide={handleChangeSide}
													/>
												</TabsContent>
											</Tabs>
										</CardContent>
									</Card>
								)}
							</div>
						</div>
					)}
				</div>
			</ScrollArea>
		</div>
	)
}

ExportInstanceView.displayName = "ExportInstanceView"

export default memo(ExportInstanceView)
