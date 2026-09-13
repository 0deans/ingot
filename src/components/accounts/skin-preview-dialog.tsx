import { Check, Copy, Download, Sparkles } from "lucide-react"
import { memo, useState } from "react"
import SkinCatalogDialog from "@/components/skins/skin-catalog-dialog"
import { Button } from "@/components/ui/button"
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog"
import { ScrollArea } from "@/components/ui/scroll-area"
import SkinAvatar from "@/components/ui/skin-avatar"
import { accountService } from "@/services/account-service"
import type { AccountProfile } from "@/types/account"
import SkinViewer3D, { DEFAULT_STEVE_SKIN } from "./skin-viewer-3d"

export interface SkinPreviewDialogProps {
	account: AccountProfile | null
	open: boolean
	onOpenChange: (open: boolean) => void
}

const SkinPreviewDialog = ({ account, open, onOpenChange }: SkinPreviewDialogProps) => {
	const [copied, setCopied] = useState(false)
	const [downloadStatus, setDownloadStatus] = useState<string | null>(null)
	const [catalogOpen, setCatalogOpen] = useState(false)

	if (!account) return null

	const skinUrl = account.skinUrl || DEFAULT_STEVE_SKIN

	const handleCopyUrl = async () => {
		try {
			await navigator.clipboard.writeText(skinUrl)
			setCopied(true)
			setTimeout(() => setCopied(false), 2000)
		} catch (err) {
			console.error("Failed to copy skin URL:", err)
		}
	}

	const handleDownload = async () => {
		try {
			setDownloadStatus("Saving...")
			const targetSkin = account.skinUrl || DEFAULT_STEVE_SKIN
			await accountService.saveSkinToDownloads(account.username, targetSkin)
			setDownloadStatus("Saved!")
			setTimeout(() => setDownloadStatus(null), 3000)
		} catch (err) {
			console.warn("Native download failed, attempting browser blob fallback:", err)
			try {
				const dataUrl = await accountService.getSkinDataUrl(account.skinUrl || DEFAULT_STEVE_SKIN)
				const href = dataUrl || skinUrl
				const res = await fetch(href)
				const blob = await res.blob()
				const blobUrl = URL.createObjectURL(blob)
				const link = document.createElement("a")
				link.href = blobUrl
				link.download = `${account.username}-skin.png`
				document.body.appendChild(link)
				link.click()
				document.body.removeChild(link)
				URL.revokeObjectURL(blobUrl)
				setDownloadStatus("Saved!")
				setTimeout(() => setDownloadStatus(null), 3000)
			} catch (fallbackErr) {
				console.error("Failed to download skin:", fallbackErr)
				setDownloadStatus("Error")
				setTimeout(() => setDownloadStatus(null), 2500)
			}
		}
	}

	return (
		<>
			<Dialog open={open} onOpenChange={onOpenChange}>
				<DialogContent className="sm:max-w-md">
					<DialogHeader>
						<div className="flex items-center gap-3">
							<SkinAvatar
								username={account.username}
								skinUrl={account.skinUrl || DEFAULT_STEVE_SKIN}
								size={40}
							/>
							<div className="flex flex-col text-left">
								<div className="flex items-center gap-2">
									<DialogTitle className="font-semibold text-base text-foreground">
										{account.username}
									</DialogTitle>
									{account.accountType === "ely" && (
										<span className="inline-flex items-center rounded-xs bg-emerald-500/15 px-1.5 py-0.5 font-medium text-[10px] text-emerald-400">
											Ely.by
										</span>
									)}
									{account.accountType === "offline" && (
										<span className="inline-flex items-center rounded-xs bg-zinc-800 px-1.5 py-0.5 font-medium text-[10px] text-zinc-400">
											Offline
										</span>
									)}
									{account.isActive && (
										<span className="inline-flex items-center gap-1 rounded-xs bg-primary/20 px-1.5 py-0.5 font-medium text-[10px] text-primary">
											<Check className="size-2.5" /> Active
										</span>
									)}
								</div>
								<DialogDescription className="font-mono text-[11px]">
									UUID: {account.uuid}
								</DialogDescription>
							</div>
						</div>
					</DialogHeader>

					{/* 3D Skin Viewer inside ScrollArea that scrolls smoothly when vertical space is constrained */}
					<ScrollArea scrollFade className="min-h-0 w-full flex-1 pr-1">
						<div className="flex flex-col gap-3 py-1">
							<SkinViewer3D
								skinUrl={account.skinUrl}
								username={account.username}
								width={280}
								height={280}
								className="w-full"
							/>
						</div>
					</ScrollArea>

					{/* Footer Actions */}
					<div className="flex shrink-0 flex-col justify-between gap-2 border-border/40 border-t pt-3 sm:flex-row sm:items-center">
						<Button
							variant="default"
							size="xs"
							onClick={() => setCatalogOpen(true)}
							className="gap-1.5 bg-emerald-600 font-medium text-white text-xs hover:bg-emerald-500"
						>
							<Sparkles className="size-3" />
							Change Skin
						</Button>
						<div className="flex items-center gap-2 self-end sm:self-auto">
							<Button
								variant="outline"
								size="xs"
								onClick={handleCopyUrl}
								className="gap-1 text-xs"
								title="Copy skin texture URL"
							>
								{copied ? (
									<Check className="size-3 text-emerald-400" />
								) : (
									<Copy className="size-3" />
								)}
								{copied ? "Copied" : "Copy URL"}
							</Button>
							<Button
								variant="outline"
								size="xs"
								onClick={handleDownload}
								className="gap-1 text-xs"
								title="Download skin texture file to Downloads"
							>
								{downloadStatus ? (
									<>
										<Check className="size-3 text-emerald-400" />
										<span className="text-emerald-400">{downloadStatus}</span>
									</>
								) : (
									<>
										<Download className="size-3" />
										Download
									</>
								)}
							</Button>
						</div>
					</div>
				</DialogContent>
			</Dialog>

			<SkinCatalogDialog
				open={catalogOpen}
				onOpenChange={setCatalogOpen}
				initialAccount={account}
			/>
		</>
	)
}

SkinPreviewDialog.displayName = "SkinPreviewDialog"

export default memo(SkinPreviewDialog)
