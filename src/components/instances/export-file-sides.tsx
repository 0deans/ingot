import { memo } from "react"
import { useTranslation } from "react-i18next"
import type { EnvSupport } from "@/bindings"
import { ScrollArea } from "@/components/ui/scroll-area"
import {
	Select,
	SelectContent,
	SelectItem,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select"

interface ExportFileSidesProps {
	candidatePaths: string[]
	fileSides: Map<string, { client: EnvSupport; server: EnvSupport }>
	onChangeSide: (path: string, client: EnvSupport, server: EnvSupport) => void
}

const ENV_OPTIONS = [
	{ value: "required" as const, labelKey: "exportInstance.envRequired" as const },
	{ value: "optional" as const, labelKey: "exportInstance.envOptional" as const },
	{ value: "unsupported" as const, labelKey: "exportInstance.envUnsupported" as const },
]

const ExportFileSides = ({ candidatePaths, fileSides, onChangeSide }: ExportFileSidesProps) => {
	const { t } = useTranslation()

	if (candidatePaths.length === 0) {
		return (
			<div className="rounded-md border border-border bg-card/40 p-4 text-center text-xs text-muted-foreground">
				{t("exportInstance.noCandidateFilesForSides")}
			</div>
		)
	}

	return (
		<div className="flex flex-col gap-2 rounded-md border border-border bg-card/40 p-2">
			<div className="grid grid-cols-[1fr_110px_110px] items-center gap-2 border-b border-border/60 pb-2 text-xs font-medium text-muted-foreground">
				<span>{t("exportInstance.fileColumn")}</span>
				<span className="text-center">{t("exportInstance.clientColumn")}</span>
				<span className="text-center">{t("exportInstance.serverColumn")}</span>
			</div>

			<ScrollArea className="h-72 scroll-fade-y pr-2">
				<div className="flex flex-col gap-1.5 py-1">
					{candidatePaths.map((path) => {
						const current = fileSides.get(path) ?? {
							client: "required",
							server: "required",
						}
						const filename = path.slice(path.lastIndexOf("/") + 1)

						return (
							<div
								key={path}
								className="grid grid-cols-[1fr_110px_110px] items-center gap-2 rounded px-1.5 py-1 text-xs hover:bg-muted/40"
							>
								<span className="truncate font-mono text-foreground" title={path}>
									{filename}
								</span>

								<Select
									value={current.client}
									onValueChange={(val) => {
										if (val === "required" || val === "optional" || val === "unsupported") {
											onChangeSide(path, val, current.server)
										}
									}}
								>
									<SelectTrigger className="h-7 text-xs">
										<SelectValue />
									</SelectTrigger>
									<SelectContent>
										{ENV_OPTIONS.map((opt) => {
											return (
												<SelectItem key={opt.value} value={opt.value} className="text-xs">
													{t(opt.labelKey)}
												</SelectItem>
											)
										})}
									</SelectContent>
								</Select>

								<Select
									value={current.server}
									onValueChange={(val) => {
										if (val === "required" || val === "optional" || val === "unsupported") {
											onChangeSide(path, current.client, val)
										}
									}}
								>
									<SelectTrigger className="h-7 text-xs">
										<SelectValue />
									</SelectTrigger>
									<SelectContent>
										{ENV_OPTIONS.map((opt) => {
											return (
												<SelectItem key={opt.value} value={opt.value} className="text-xs">
													{t(opt.labelKey)}
												</SelectItem>
											)
										})}
									</SelectContent>
								</Select>
							</div>
						)
					})}
				</div>
			</ScrollArea>
		</div>
	)
}

ExportFileSides.displayName = "ExportFileSides"

export default memo(ExportFileSides)
