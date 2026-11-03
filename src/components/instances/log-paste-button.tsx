import { openUrl } from "@tauri-apps/plugin-opener"
import { Share2 } from "lucide-react"
import { memo, useCallback, useState } from "react"
import { useTranslation } from "react-i18next"
import { toast } from "sonner"
import { Button } from "@/components/ui/button"
import { Spinner } from "@/components/ui/spinner"
import { cn } from "@/lib/utils"
import { instanceLogService } from "@/services/instance-log-service"

export interface LogPasteButtonProps {
	getContent: () => string
	disabled?: boolean
	size?: "xs" | "sm"
	compact?: boolean
}

const LogPasteButton = ({
	getContent,
	disabled = false,
	size = "sm",
	compact = false,
}: LogPasteButtonProps) => {
	const { t } = useTranslation()
	const [isUploading, setIsUploading] = useState(false)

	const handleUpload = useCallback(async () => {
		const content = getContent()
		if (!content.trim()) {
			return
		}
		setIsUploading(true)
		try {
			const result = await instanceLogService.uploadLogPaste(content, "mclogs")
			await navigator.clipboard.writeText(result.url).catch(() => {})
			toast.success(
				t("console.pasteUploaded", {
					provider: "mclo.gs",
				}),
				{
					description: result.url,
					action: {
						label: t("console.openPasteUrl"),
						onClick: () => {
							openUrl(result.url).catch(() => {})
						},
					},
				},
			)
		} catch (error) {
			const message =
				error && typeof error === "object" && "message" in error
					? String((error as { message: unknown }).message)
					: String(error)
			toast.error(t("console.pasteFailed", { error: message }))
		} finally {
			setIsUploading(false)
		}
	}, [getContent, t])

	return (
		<Button
			variant="outline"
			size={size}
			disabled={disabled || isUploading}
			onClick={handleUpload}
			className={cn("gap-1.5 px-2.5 text-xs", compact && "max-lg:size-7 max-lg:p-0")}
			aria-label={t("console.uploadPaste", { provider: "mclo.gs" })}
		>
			{isUploading ? <Spinner className="size-3.5" /> : <Share2 className="size-3.5" />}
			<span className={cn(compact && "hidden lg:inline")}>
				{isUploading
					? t("console.uploadingPaste")
					: t("console.uploadPaste", { provider: "mclo.gs" })}
			</span>
		</Button>
	)
}

LogPasteButton.displayName = "LogPasteButton"

export default memo(LogPasteButton)
