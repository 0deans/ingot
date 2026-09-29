import { useNavigate } from "@tanstack/react-router"
import { Check, Copy, Download, Sparkles } from "lucide-react"
import { memo, useState } from "react"
import { useTranslation } from "react-i18next"
import { toast } from "sonner"
import { AccountTypeBadge } from "@/components/accounts/account-type-badge"
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "@/components/common/dialog"
import SkinAvatar from "@/components/common/skin-avatar"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Spinner } from "@/components/ui/spinner"
import { accountService } from "@/services/account-service"
import type { AccountProfile } from "@/types/account"
import SkinViewer3D, { DEFAULT_STEVE_SKIN } from "./skin-viewer-3d"

export interface SkinPreviewDialogProps {
	account: AccountProfile | null
	open: boolean
	onOpenChange: (open: boolean) => void
}

const SkinPreviewDialog = ({ account, open, onOpenChange }: SkinPreviewDialogProps) => {
	const { t } = useTranslation()
	const navigate = useNavigate()
	const [copied, setCopied] = useState(false)
	const [isSaving, setIsSaving] = useState(false)

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
		setIsSaving(true)
		try {
			const targetSkin = account.skinUrl || DEFAULT_STEVE_SKIN
			const savedPath = await accountService.saveSkinToDownloads(account.username, targetSkin)
			// No path means the user cancelled the save dialog
			if (savedPath) toast.success(t("skinPreview.saved"))
		} catch (err) {
			console.error("Failed to save skin:", err)
			toast.error(t("skinPreview.error"))
		} finally {
			setIsSaving(false)
		}
	}

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent
				style={{ padding: 0 }}
				className="!p-0 sm:!p-0 !gap-0 sm:!max-w-md flex h-[580px] max-h-[calc(100vh-3.5rem)] flex-col justify-between overflow-hidden"
			>
				{/* Background Studio Lighting Glow */}
				<div className="pointer-events-none absolute inset-0 z-0 bg-[radial-gradient(circle_at_50%_35%,rgba(16,185,129,0.08),transparent_70%)]" />

				{/* Floating Header */}
				<DialogHeader className="pointer-events-none relative z-10 flex shrink-0 bg-gradient-to-b from-popover/90 via-popover/50 to-transparent p-4 sm:p-5 sm:pb-2">
					<div className="pointer-events-auto flex items-center gap-3">
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
								<AccountTypeBadge type={account.accountType} />
								{account.isActive && (
									<Badge>
										<Check /> {t("common.active")}
									</Badge>
								)}
							</div>
							<DialogDescription className="font-mono text-[11px]">
								UUID: {account.uuid}
							</DialogDescription>
						</div>
					</div>
				</DialogHeader>

				{/* 3D Skin Viewer Stage: Canvas fills entire dialog behind overlays */}
				<div className="absolute inset-0 z-0 size-full">
					<SkinViewer3D
						skinUrl={account.skinUrl}
						username={account.username}
						showToolbar={false}
						autoResize={true}
						borderless={true}
						floatingToolbar={true}
						zoom={0.68}
						floatingControlsClassName="top-16 right-3"
						className="size-full"
						footerActions={
							<>
								<Button
									variant="default"
									size="xs"
									onClick={() => {
										onOpenChange(false)
										navigate({ to: "/skins" })
									}}
									className="gap-1.5 font-medium text-xs"
								>
									<Sparkles className="size-3" />
									{t("skinPreview.changeSkin")}
								</Button>
								<div className="flex items-center gap-2">
									<Button
										variant="outline"
										size="xs"
										onClick={handleCopyUrl}
										className="gap-1 text-xs"
										title={t("skinPreview.copyUrlTitle")}
									>
										{copied ? (
											<Check className="size-3 text-primary" />
										) : (
											<Copy className="size-3" />
										)}
										{copied ? t("skinPreview.copied") : t("skinPreview.copyUrl")}
									</Button>
									<Button
										variant="outline"
										size="xs"
										onClick={handleDownload}
										disabled={isSaving}
										className="gap-1 text-xs"
										title={t("skinPreview.downloadTitle")}
									>
										{isSaving ? <Spinner /> : <Download />}
										{isSaving ? t("skinPreview.saving") : t("skinPreview.download")}
									</Button>
								</div>
							</>
						}
					/>
				</div>
			</DialogContent>
		</Dialog>
	)
}

SkinPreviewDialog.displayName = "SkinPreviewDialog"

export default memo(SkinPreviewDialog)
