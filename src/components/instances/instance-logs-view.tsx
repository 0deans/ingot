import {
	AlertTriangle,
	ArrowDown,
	ArrowDownToLine,
	ArrowLeft,
	Check,
	Copy,
	Eraser,
	FileText,
	FolderOpen,
	RefreshCw,
	Search,
	Square,
	Terminal,
	WrapText,
	X,
} from "lucide-react"
import { memo, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react"
import { useTranslation } from "react-i18next"
import type {
	CrashSuspect,
	InstanceConfig,
	InstanceLogFileContent,
	InstanceLogFileSummary,
	InstanceLogLine,
	LogFileKind,
	LogLevel,
	RunningInstanceSummary,
} from "@/bindings"
import LogPasteButton from "@/components/instances/log-paste-button"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { ScrollArea } from "@/components/ui/scroll-area"
import {
	Select,
	SelectContent,
	SelectGroup,
	SelectItem,
	SelectLabel,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select"
import { Spinner } from "@/components/ui/spinner"
import { Toggle } from "@/components/ui/toggle"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { formatBytes } from "@/lib/format"
import { cn } from "@/lib/utils"
import {
	instanceLogService,
	useInstanceCrashState,
	useInstanceLiveLogs,
} from "@/services/instance-log-service"

export interface InstanceLogsViewProps {
	instance: InstanceConfig
	runningInfo?: RunningInstanceSummary
	initialSourceKey?: string | null
	onBack: () => void
	onStopInstance?: (instanceId: string) => void
}

type LevelFilter = "all" | "error" | "warn" | "info" | "debug"

const LIVE_SOURCE_KEY = "live"

function buildFileSourceKey(kind: LogFileKind, fileName: string): string {
	return `${kind}:${fileName}`
}

function parseFileSourceKey(key: string): { kind: LogFileKind; fileName: string } | null {
	if (key.startsWith("log:")) {
		return { kind: "log", fileName: key.slice(4) }
	}
	if (key.startsWith("crashReport:")) {
		return { kind: "crashReport", fileName: key.slice(12) }
	}
	return null
}

function isAtBottom(el: HTMLDivElement): boolean {
	return el.scrollHeight - el.scrollTop - el.clientHeight < 40
}

function getLevelRowClasses(level: LogLevel): string {
	switch (level) {
		case "fatal":
		case "error": {
			return "text-destructive"
		}
		case "warn": {
			return "text-warning"
		}
		case "launcher": {
			return "text-info"
		}
		case "debug": {
			return "text-muted-foreground"
		}
		case "info": {
			return "text-foreground/85"
		}
	}
}

function getLevelBadgeClasses(level: LogLevel): string {
	switch (level) {
		case "fatal":
		case "error": {
			return "border-destructive/30 bg-destructive/10 text-destructive"
		}
		case "warn": {
			return "border-warning/30 bg-warning/10 text-warning"
		}
		case "launcher": {
			return "border-info/30 bg-info/10 text-info"
		}
		case "debug": {
			return "border-border bg-muted/40 text-muted-foreground"
		}
		case "info": {
			return "border-primary/25 bg-primary/10 text-primary"
		}
	}
}

function matchesLevelFilter(level: LogLevel, filter: LevelFilter): boolean {
	switch (filter) {
		case "all": {
			return true
		}
		case "error": {
			return level === "error" || level === "fatal"
		}
		case "warn": {
			return level === "warn"
		}
		case "info": {
			return level === "info" || level === "launcher"
		}
		case "debug": {
			return level === "debug"
		}
	}
}

interface CrashDiagnosticsBannerProps {
	exitCode?: number | null
	description?: string | null
	suspects: CrashSuspect[]
	crashReportFile?: string | null
	onOpenCrashReport?: (file: string) => void
}

const CrashDiagnosticsBanner = memo(
	({
		exitCode,
		description,
		suspects,
		crashReportFile,
		onOpenCrashReport,
	}: CrashDiagnosticsBannerProps) => {
		const { t } = useTranslation()

		return (
			<div className="flex shrink-0 flex-col gap-2 border-b border-destructive/30 bg-destructive/10 px-4 py-3 text-xs text-foreground">
				<div className="flex items-center justify-between gap-2">
					<div className="flex items-center gap-2">
						<AlertTriangle className="size-4 shrink-0 text-destructive" />
						<span className="font-semibold text-destructive">
							{t("instanceLogs.crashDetectedTitle")}
						</span>
						{exitCode != null && (
							<Badge variant="outline" className="border-destructive/40 text-3xs text-destructive">
								{t("instanceLogs.crashExitCode", { code: exitCode })}
							</Badge>
						)}
					</div>
					{crashReportFile && onOpenCrashReport && (
						<Button
							size="xs"
							variant="outline"
							onClick={() => {
								onOpenCrashReport(crashReportFile)
							}}
							className="gap-1.5"
						>
							<FileText className="size-3" />
							<span>{t("instanceLogs.openCrashReport")}</span>
						</Button>
					)}
				</div>

				{description && (
					<p className="line-clamp-2 font-mono text-2xs text-muted-foreground">{description}</p>
				)}

				{suspects.length > 0 && (
					<div className="flex flex-wrap items-center gap-1.5 pt-0.5">
						<span className="text-3xs font-medium text-muted-foreground">
							{t("instanceLogs.suspectedMods")}
						</span>
						{suspects.map((suspect) => {
							return (
								<Badge
									key={`${suspect.folder}/${suspect.fileName}`}
									variant="outline"
									className="border-warning/40 bg-warning/10 text-3xs font-medium text-warning"
								>
									{suspect.title} ({suspect.fileName})
								</Badge>
							)
						})}
					</div>
				)}
			</div>
		)
	},
)

CrashDiagnosticsBanner.displayName = "CrashDiagnosticsBanner"

const InstanceLogsView = ({
	instance,
	runningInfo,
	initialSourceKey = null,
	onBack,
	onStopInstance,
}: InstanceLogsViewProps) => {
	const { t } = useTranslation()
	const instanceId = instance.id
	const isRunning = Boolean(runningInfo)

	const { logs: liveLogs, clearLogs } = useInstanceLiveLogs(instanceId)
	const { instanceCrash } = useInstanceCrashState(instanceId)

	const scrollRef = useRef<HTMLDivElement | null>(null)
	const lastScroll = useRef({ top: 0, height: 0 })

	const [sourceKey, setSourceKey] = useState<string>(initialSourceKey ?? LIVE_SOURCE_KEY)
	const [logFiles, setLogFiles] = useState<InstanceLogFileSummary[]>([])
	const [selectedFileContent, setSelectedFileContent] = useState<InstanceLogFileContent | null>(
		null,
	)
	const [isLoadingFile, setIsLoadingFile] = useState(false)
	const [levelFilter, setLevelFilter] = useState<LevelFilter>("all")
	const [searchQuery, setSearchQuery] = useState("")
	const [autoScroll, setAutoScroll] = useState(true)
	const [atBottom, setAtBottom] = useState(true)
	const [lineWrap, setLineWrap] = useState(true)
	const [copiedSanitized, setCopiedSanitized] = useState(false)
	const [prevSourceKey, setPrevSourceKey] = useState<string>(sourceKey)

	if (sourceKey !== prevSourceKey) {
		setPrevSourceKey(sourceKey)
		if (sourceKey === LIVE_SOURCE_KEY) {
			setSelectedFileContent(null)
			setIsLoadingFile(false)
		} else {
			setIsLoadingFile(true)
		}
	}

	const refreshFileList = useCallback(async () => {
		try {
			const files = await instanceLogService.listLogFiles(instanceId)
			setLogFiles(files)
		} catch (error) {
			console.error("Failed to list instance log files:", error)
		}
	}, [instanceId])

	const activeLines: InstanceLogLine[] = useMemo(() => {
		if (sourceKey === LIVE_SOURCE_KEY) {
			return liveLogs
		}
		return selectedFileContent?.lines ?? []
	}, [liveLogs, selectedFileContent, sourceKey])

	const levelCounts = useMemo(() => {
		let errorCount = 0
		let warnCount = 0
		let infoCount = 0
		let debugCount = 0
		for (const line of activeLines) {
			if (line.level === "error" || line.level === "fatal") {
				errorCount += 1
			} else if (line.level === "warn") {
				warnCount += 1
			} else if (line.level === "debug") {
				debugCount += 1
			} else {
				infoCount += 1
			}
		}
		return {
			all: activeLines.length,
			error: errorCount,
			warn: warnCount,
			info: infoCount,
			debug: debugCount,
		}
	}, [activeLines])

	const filteredLines = useMemo(() => {
		const trimmedSearch = searchQuery.trim().toLowerCase()
		return activeLines.filter((line) => {
			if (!matchesLevelFilter(line.level, levelFilter)) {
				return false
			}
			if (!trimmedSearch) {
				return true
			}
			return line.raw.toLowerCase().includes(trimmedSearch)
		})
	}, [activeLines, levelFilter, searchQuery])

	const getExportContent = useCallback((): string => {
		if (sourceKey !== LIVE_SOURCE_KEY && selectedFileContent) {
			return selectedFileContent.raw
		}
		return activeLines
			.map((line) => {
				return line.raw
			})
			.join("\n")
	}, [activeLines, selectedFileContent, sourceKey])

	const handleCopySanitized = useCallback(async () => {
		const raw = getExportContent()
		if (!raw.trim()) {
			return
		}
		try {
			const sanitized = await instanceLogService.sanitizeLogText(raw)
			await navigator.clipboard.writeText(sanitized)
			setCopiedSanitized(true)
			setTimeout(() => {
				setCopiedSanitized(false)
			}, 1500)
		} catch (error) {
			console.error("Failed to copy sanitized log:", error)
		}
	}, [getExportContent])

	const handleOpenFolder = useCallback(() => {
		const parsed = parseFileSourceKey(sourceKey)
		void instanceLogService.openLogLocation(
			instance.id,
			parsed?.kind ?? null,
			parsed?.fileName ?? null,
		)
	}, [instance.id, sourceKey])

	const handleStopClick = useCallback(() => {
		if (onStopInstance) {
			onStopInstance(instance.id)
		}
	}, [instance.id, onStopInstance])

	const handleOpenCrashReport = useCallback((fileName: string) => {
		setSourceKey(buildFileSourceKey("crashReport", fileName))
	}, [])

	const jumpToLatest = useCallback(() => {
		const el = scrollRef.current
		if (el) {
			el.scrollTo({ top: el.scrollHeight, behavior: "smooth" })
			setAutoScroll(true)
		}
	}, [])

	useEffect(() => {
		let cancelled = false
		instanceLogService
			.listLogFiles(instanceId)
			.then((files) => {
				if (!cancelled) {
					setLogFiles(files)
				}
			})
			.catch((error) => {
				if (!cancelled) {
					console.error("Failed to list instance log files:", error)
				}
			})
		return () => {
			cancelled = true
		}
	}, [instanceId])

	useEffect(() => {
		if (sourceKey === LIVE_SOURCE_KEY) {
			return
		}
		const parsed = parseFileSourceKey(sourceKey)
		if (!parsed) {
			return
		}

		let cancelled = false
		instanceLogService
			.readLogFile(instanceId, parsed.kind, parsed.fileName)
			.then((content) => {
				if (!cancelled) {
					setSelectedFileContent(content)
					setIsLoadingFile(false)
				}
			})
			.catch((err) => {
				if (!cancelled) {
					console.error("Failed to read log file:", err)
					setIsLoadingFile(false)
				}
			})

		return () => {
			cancelled = true
		}
	}, [instanceId, sourceKey])

	useEffect(() => {
		const el = scrollRef.current
		if (!el) {
			return
		}
		const onScroll = () => {
			const prev = lastScroll.current
			if (el.scrollTop < prev.top - 2 && el.scrollHeight >= prev.height) {
				setAutoScroll(false)
			}
			lastScroll.current = { top: el.scrollTop, height: el.scrollHeight }
			setAtBottom(isAtBottom(el))
		}
		el.addEventListener("scroll", onScroll, { passive: true })
		return () => {
			el.removeEventListener("scroll", onScroll)
		}
	}, [])

	useLayoutEffect(() => {
		if (!autoScroll) {
			return
		}
		const el = scrollRef.current
		if (el) {
			el.scrollTop = el.scrollHeight
		}
	}, [filteredLines, autoScroll])

	const crashReports = logFiles.filter((file) => {
		return file.kind === "crashReport"
	})
	const diskLogs = logFiles.filter((file) => {
		return file.kind === "log"
	})

	const showCrashBanner =
		sourceKey === LIVE_SOURCE_KEY
			? Boolean(instanceCrash) && !isRunning
			: selectedFileContent?.kind === "crashReport" ||
				(selectedFileContent?.suspects.length ?? 0) > 0

	const bannerSuspects =
		sourceKey === LIVE_SOURCE_KEY
			? (instanceCrash?.suspects ?? [])
			: (selectedFileContent?.suspects ?? [])
	const bannerDescription =
		sourceKey === LIVE_SOURCE_KEY ? instanceCrash?.description : selectedFileContent?.description
	const bannerExitCode = sourceKey === LIVE_SOURCE_KEY ? instanceCrash?.exitCode : null
	const bannerCrashFile = sourceKey === LIVE_SOURCE_KEY ? instanceCrash?.crashReportFile : null

	return (
		<div className="flex h-full min-h-0 w-full flex-col overflow-hidden bg-background">
			{/* Top Header Bar */}
			<div className="flex shrink-0 items-center justify-between gap-2.5 border-b border-border/50 bg-card/40 px-3 py-2.5 sm:gap-3 sm:px-6 sm:py-3">
				<div className="flex min-w-0 items-center gap-2 sm:gap-3">
					<Button
						variant="ghost"
						size="sm"
						onClick={onBack}
						className="gap-1.5 px-2 text-xs font-medium text-muted-foreground hover:text-foreground sm:px-2.5"
						aria-label={t("instanceLogs.backToInstances")}
					>
						<ArrowLeft className="size-4" />
						<span className="hidden lg:inline">{t("instanceLogs.backToInstances")}</span>
					</Button>

					<div className="h-5 w-px bg-border/60" />

					<div className="flex size-8 shrink-0 items-center justify-center rounded-lg border border-border bg-card">
						<Terminal className="size-4 text-primary" />
					</div>
					<div className="flex min-w-0 flex-col">
						<div className="flex min-w-0 items-center gap-2">
							<h1 className="truncate text-sm font-semibold text-foreground">{instance.name}</h1>
							{isRunning && runningInfo ? (
								<Badge
									variant="outline"
									className="shrink-0 border-primary/40 text-2xs text-primary"
								>
									<span className="size-1.5 animate-pulse rounded-full bg-primary" />
									{t("instanceLogs.runningBadge", { pid: runningInfo.pid })}
								</Badge>
							) : (
								<Badge variant="outline" className="shrink-0 text-2xs text-muted-foreground">
									{t("instanceLogs.offlineBadge")}
								</Badge>
							)}
						</div>
						<span className="truncate text-2xs text-muted-foreground">
							{instance.loader.toUpperCase()} {instance.gameVersion}
						</span>
					</div>
				</div>

				<div className="flex shrink-0 items-center gap-1.5">
					{isRunning && onStopInstance && (
						<Button
							size="sm"
							variant="destructive"
							onClick={handleStopClick}
							className="gap-1.5 px-2.5 text-xs"
						>
							<Square className="size-3 fill-current" />
							<span className="hidden md:inline">{t("instances.card.stop")}</span>
						</Button>
					)}
					<LogPasteButton
						getContent={getExportContent}
						disabled={activeLines.length === 0}
						size="sm"
						compact
					/>
					<Button
						variant="outline"
						size="sm"
						disabled={activeLines.length === 0}
						onClick={handleCopySanitized}
						className="gap-1.5 px-2.5 text-xs max-lg:size-7 max-lg:p-0"
						aria-label={t("console.copySanitized")}
					>
						{copiedSanitized ? (
							<Check className="size-3.5 text-primary" />
						) : (
							<Copy className="size-3.5" />
						)}
						<span className="hidden lg:inline">{t("console.copySanitized")}</span>
					</Button>
					<Tooltip>
						<TooltipTrigger
							render={
								<Button
									variant="outline"
									size="icon-sm"
									onClick={handleOpenFolder}
									aria-label={t("instances.card.openFolderTooltip")}
								/>
							}
						>
							<FolderOpen className="size-3.5" />
						</TooltipTrigger>
						<TooltipContent side="bottom" align="end">
							{t("instances.card.openFolderTooltip")}
						</TooltipContent>
					</Tooltip>
				</div>
			</div>

			{/* Toolbar: Source Selector, Severity Filters, Search, View Controls */}
			<div className="flex shrink-0 flex-col gap-2.5 border-b border-border/40 bg-background/60 px-4 py-2.5 sm:px-6">
				{/* Row 1: Source Selector + Refresh on left; View Controls on right */}
				<div className="flex items-center justify-between gap-2">
					<div className="flex items-center gap-1.5">
						<Select
							value={sourceKey}
							onValueChange={(val) => {
								if (val) {
									setSourceKey(val)
								}
							}}
						>
							<SelectTrigger className="h-8 w-52 text-xs sm:w-64">
								<SelectValue />
							</SelectTrigger>
							<SelectContent>
								<SelectGroup>
									<SelectLabel className="text-3xs uppercase">
										{t("instanceLogs.liveStream")}
									</SelectLabel>
									<SelectItem value={LIVE_SOURCE_KEY} className="text-xs">
										{t("instanceLogs.sources.live")} ({liveLogs.length})
									</SelectItem>
								</SelectGroup>

								{diskLogs.length > 0 && (
									<SelectGroup>
										<SelectLabel className="text-3xs uppercase">
											{t("instanceLogs.sources.archivedLogs")}
										</SelectLabel>
										{diskLogs.map((file) => {
											const key = buildFileSourceKey("log", file.fileName)
											return (
												<SelectItem key={key} value={key} className="text-xs">
													{file.fileName} ({formatBytes(file.sizeBytes)})
												</SelectItem>
											)
										})}
									</SelectGroup>
								)}

								{crashReports.length > 0 && (
									<SelectGroup>
										<SelectLabel className="text-3xs uppercase">
											{t("instanceLogs.sources.crashReports")}
										</SelectLabel>
										{crashReports.map((file) => {
											const key = buildFileSourceKey("crashReport", file.fileName)
											return (
												<SelectItem key={key} value={key} className="text-xs text-destructive">
													{file.fileName}
												</SelectItem>
											)
										})}
									</SelectGroup>
								)}
							</SelectContent>
						</Select>

						<Tooltip>
							<TooltipTrigger
								render={
									<Button
										variant="ghost"
										size="icon-sm"
										onClick={() => {
											void refreshFileList()
										}}
										aria-label={t("common.refresh")}
									/>
								}
							>
								<RefreshCw className="size-3.5" />
							</TooltipTrigger>
							<TooltipContent side="bottom">{t("common.refresh")}</TooltipContent>
						</Tooltip>
					</div>

					{/* View Controls: Wrap, Auto-Scroll, Clear */}
					<div className="flex items-center gap-1">
						<Toggle
							variant="outline"
							size="sm"
							pressed={lineWrap}
							onPressedChange={setLineWrap}
							aria-label={lineWrap ? t("instanceLogs.wrapOn") : t("instanceLogs.wrapOff")}
							className="h-8 gap-1.5 px-2 text-2xs"
						>
							<WrapText className="size-3.5" />
							<span className="hidden sm:inline">
								{lineWrap ? t("instanceLogs.wrapOn") : t("instanceLogs.wrapOff")}
							</span>
						</Toggle>

						{sourceKey === LIVE_SOURCE_KEY && (
							<>
								<Toggle
									variant="outline"
									size="sm"
									pressed={autoScroll}
									onPressedChange={setAutoScroll}
									className="h-8 gap-1.5 px-2 text-2xs"
									aria-label={autoScroll ? t("console.autoScrollOn") : t("console.autoScrollOff")}
								>
									<ArrowDownToLine className="size-3.5" />
									<span className="hidden sm:inline">
										{autoScroll ? t("console.autoScrollOn") : t("console.autoScrollOff")}
									</span>
								</Toggle>

								<Tooltip>
									<TooltipTrigger
										render={
											<Button
												variant="ghost"
												size="icon-sm"
												onClick={() => {
													void clearLogs()
												}}
												disabled={liveLogs.length === 0}
												aria-label={t("console.clear")}
											/>
										}
									>
										<Eraser className="size-3.5" />
									</TooltipTrigger>
									<TooltipContent side="bottom">{t("console.clear")}</TooltipContent>
								</Tooltip>
							</>
						)}
					</div>
				</div>

				{/* Row 2: Severity Filter Pills on left; Search Box on right */}
				<div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
					{/* Severity Filter Pills */}
					<div className="flex items-center gap-0.5 overflow-x-auto rounded-lg border border-border/60 bg-card/30 p-0.5">
						{(["all", "error", "warn", "info", "debug"] as const).map((lvl) => {
							const active = levelFilter === lvl
							const count = levelCounts[lvl]
							const isErrorWithCount = lvl === "error" && count > 0
							const isWarnWithCount = lvl === "warn" && count > 0
							return (
								<button
									key={lvl}
									type="button"
									onClick={() => {
										setLevelFilter(lvl)
									}}
									className={cn(
										"inline-flex shrink-0 items-center gap-1 rounded-md px-2 py-1 text-2xs font-medium transition-colors",
										active
											? "bg-primary text-primary-foreground shadow-xs"
											: isErrorWithCount
												? "text-destructive hover:bg-destructive/10"
												: isWarnWithCount
													? "text-warning hover:bg-warning/10"
													: "text-muted-foreground hover:bg-muted/60 hover:text-foreground",
									)}
								>
									<span>{t(`instanceLogs.levels.${lvl}`)}</span>
									<span
										className={cn(
											"rounded px-1 font-mono text-3xs",
											active
												? "bg-primary-foreground/20 text-primary-foreground"
												: isErrorWithCount
													? "bg-destructive/20 font-semibold text-destructive"
													: isWarnWithCount
														? "bg-warning/20 font-semibold text-warning"
														: "bg-muted text-muted-foreground",
										)}
									>
										{count}
									</span>
								</button>
							)
						})}
					</div>

					{/* Search Box */}
					<div className="relative min-w-[140px] flex-1 sm:max-w-xs md:max-w-sm">
						<Search className="pointer-events-none absolute top-1/2 left-2.5 size-3.5 -translate-y-1/2 text-muted-foreground" />
						<Input
							value={searchQuery}
							onChange={(e) => {
								setSearchQuery(e.target.value)
							}}
							placeholder={t("instanceLogs.searchPlaceholder")}
							className="h-8 pr-7 pl-8 text-xs"
						/>
						{searchQuery && (
							<button
								type="button"
								onClick={() => {
									setSearchQuery("")
								}}
								className="absolute top-1/2 right-2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
								aria-label={t("common.clearSearch")}
							>
								<X className="size-3.5" />
							</button>
						)}
					</div>
				</div>
			</div>

			{/* Crash Diagnostics Banner if present */}
			{showCrashBanner && (
				<CrashDiagnosticsBanner
					exitCode={bannerExitCode}
					description={bannerDescription}
					suspects={bannerSuspects}
					crashReportFile={bannerCrashFile}
					onOpenCrashReport={handleOpenCrashReport}
				/>
			)}

			{/* Main Log Output Viewport with Reusable ScrollArea */}
			<div className="relative min-h-0 flex-1">
				<ScrollArea
					viewportRef={scrollRef}
					showHorizontal={!lineWrap}
					className="size-full scroll-fade-y"
					viewportClassName="px-4 py-3 font-mono text-2xs leading-relaxed select-text"
				>
					{isLoadingFile ? (
						<div className="flex h-64 items-center justify-center gap-2 font-sans text-xs text-muted-foreground">
							<Spinner className="size-4" />
							<span>Loading log file...</span>
						</div>
					) : activeLines.length === 0 ? (
						<div className="flex h-64 flex-col items-center justify-center gap-1.5 text-center font-sans">
							<Terminal className="size-8 text-muted-foreground/50" />
							<p className="text-sm font-medium text-foreground">
								{isRunning
									? t("instanceLogs.emptyLiveRunning")
									: t("instanceLogs.emptyLiveOffline")}
							</p>
							{!isRunning && (
								<p className="max-w-md text-xs text-muted-foreground">
									{t("instanceLogs.emptyLiveOfflineHint")}
								</p>
							)}
						</div>
					) : filteredLines.length === 0 ? (
						<div className="flex h-64 flex-col items-center justify-center gap-2 text-center font-sans">
							<p className="text-xs text-muted-foreground">{t("instanceLogs.emptyFiltered")}</p>
							<Button
								size="xs"
								variant="outline"
								onClick={() => {
									setLevelFilter("all")
									setSearchQuery("")
								}}
							>
								{t("common.clearFilters")}
							</Button>
						</div>
					) : (
						filteredLines.map((line) => {
							const structured = line.timestamp != null
							return (
								<div
									key={line.seq}
									className={cn(
										"group flex items-start gap-2 rounded-sm px-1.5 py-0.5 transition-colors hover:bg-muted/30",
										getLevelRowClasses(line.level),
									)}
								>
									{structured ? (
										<>
											<span className="w-14 shrink-0 font-mono text-3xs text-muted-foreground/70 tabular-nums select-none">
												{line.timestamp}
											</span>
											<span
												className={cn(
													"mt-0.5 inline-flex w-12 shrink-0 items-center justify-center rounded border px-1 py-0 text-3xs font-semibold tracking-wider uppercase select-none",
													getLevelBadgeClasses(line.level),
												)}
											>
												{line.level}
											</span>
											{line.source && (
												<span
													className="max-w-[140px] shrink-0 truncate text-3xs text-muted-foreground select-none"
													title={line.source}
												>
													[{line.source}]
												</span>
											)}
											<span
												className={cn(
													"min-w-0 flex-1",
													lineWrap ? "wrap-break-word whitespace-pre-wrap" : "whitespace-pre",
												)}
											>
												{line.message}
											</span>
										</>
									) : (
										<span
											className={cn(
												"min-w-0 flex-1",
												lineWrap ? "wrap-break-word whitespace-pre-wrap" : "whitespace-pre",
											)}
										>
											{line.raw}
										</span>
									)}
								</div>
							)
						})
					)}
				</ScrollArea>

				{!atBottom && filteredLines.length > 0 && (
					<Button
						variant="secondary"
						size="xs"
						onClick={jumpToLatest}
						className="absolute bottom-4 left-1/2 z-10 -translate-x-1/2 shadow-lg"
					>
						<ArrowDown className="size-3.5" />
						<span>{t("console.latest")}</span>
					</Button>
				)}
			</div>

			{/* Bottom Status Bar */}
			<div className="flex shrink-0 items-center justify-between border-t border-border/40 bg-card/30 px-4 py-2 text-3xs text-muted-foreground">
				<div className="flex items-center gap-2">
					<span>
						{t("instanceLogs.showingLines", {
							count: filteredLines.length,
							total: activeLines.length,
						})}
					</span>
					{searchQuery && (
						<span className="font-medium text-primary">({t("instanceLogs.searchFiltered")})</span>
					)}
				</div>
				<div className="flex items-center gap-3">
					{sourceKey === LIVE_SOURCE_KEY && isRunning && (
						<span className="flex items-center gap-1.5 text-primary">
							<span className="size-1.5 animate-pulse rounded-full bg-primary" />
							{t("instanceLogs.streamingLive")}
						</span>
					)}
					{selectedFileContent && (
						<span>
							{selectedFileContent.fileName} ({selectedFileContent.lines.length} lines)
						</span>
					)}
				</div>
			</div>
		</div>
	)
}

InstanceLogsView.displayName = "InstanceLogsView"

export default memo(InstanceLogsView)
