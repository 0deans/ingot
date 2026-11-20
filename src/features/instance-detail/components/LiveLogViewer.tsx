import { Terminal, Trash2 } from "lucide-react"
import { useTranslation } from "react-i18next"
import { useInstanceLogStream } from "@/features/instance-detail/hooks/useInstanceLogStream"
import { cn } from "@/shared/lib/utils"

interface LiveLogViewerProps {
	instanceName: string
}

const LiveLogViewer = ({ instanceName }: LiveLogViewerProps) => {
	const { t } = useTranslation()
	const { logs, isRunning, clearLogs } = useInstanceLogStream(instanceName)

	return (
		<div className="flex flex-col overflow-hidden rounded-2xl border border-border bg-card/85 backdrop-blur-md">
			<div className="flex items-center justify-between gap-3 border-b border-border px-3.5 py-2">
				<div className="flex min-w-0 items-center gap-2 text-xs font-medium text-foreground">
					<Terminal className="size-3.5 shrink-0 text-primary" />
					<span className="truncate">{t("instanceDetail.logs.title")}</span>
					<span
						className={cn(
							"shrink-0 rounded-full px-1.5 py-0.5 text-[10px] font-semibold",
							isRunning ? "bg-primary/20 text-primary" : "bg-foreground/8 text-muted-foreground",
						)}
					>
						{isRunning ? t("instanceDetail.logs.live") : t("instanceDetail.logs.idle")}
					</span>
				</div>
				<button
					type="button"
					onClick={clearLogs}
					className="flex shrink-0 cursor-pointer items-center gap-1 rounded-lg bg-foreground/6 px-2 py-1 text-[11px] whitespace-nowrap text-foreground/80 transition-colors hover:bg-foreground/12"
				>
					<Trash2 className="size-3" />
					<span>{t("common.clear")}</span>
				</button>
			</div>

			<pre className="custom-scrollbar max-h-72 overflow-y-auto bg-background/60 p-3.5 font-mono text-[11.5px] leading-relaxed wrap-break-word whitespace-pre-wrap text-foreground/85">
				{logs.length === 0 ? t("instanceDetail.logs.empty") : logs.join("\n")}
			</pre>
		</div>
	)
}

export default LiveLogViewer
