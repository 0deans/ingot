import { openUrl } from "@tauri-apps/plugin-opener"
import { AlertCircle, Check, ExternalLink, ShieldCheck, User } from "lucide-react"
import { memo, useRef, useState } from "react"
import { useTranslation } from "react-i18next"
import type { MicrosoftDeviceCode } from "@/bindings"
import { alertTone } from "@/components/common/alert-tones"
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogHeader,
	DialogTitle,
} from "@/components/common/dialog"
import { ScrollArea } from "@/components/common/scroll-area"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Spinner } from "@/components/ui/spinner"
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { accountService } from "@/services/account-service"
import type { AccountProfile } from "@/types/account"

interface AddAccountDialogProps {
	open: boolean
	onOpenChange: (open: boolean) => void
	onAccountAdded?: (account: AccountProfile) => void
}

type TabType = "ely" | "offline" | "microsoft"

const AddAccountDialog = ({ open, onOpenChange, onAccountAdded }: AddAccountDialogProps) => {
	const { t } = useTranslation()
	const [activeTab, setActiveTab] = useState<TabType>("ely")
	const [username, setUsername] = useState("")
	const [password, setPassword] = useState("")
	const [offlineName, setOfflineName] = useState("")
	const [isLoading, setIsLoading] = useState(false)
	const [errorMessage, setErrorMessage] = useState<string | null>(null)
	const [successMessage, setSuccessMessage] = useState<string | null>(null)
	const [deviceCode, setDeviceCode] = useState<MicrosoftDeviceCode | null>(null)
	// Bumped on cancel, so a stopped Microsoft sign-in can no longer update the dialog
	const microsoftAttempt = useRef(0)

	const cancelMicrosoftLogin = () => {
		if (!deviceCode) return
		microsoftAttempt.current++
		setDeviceCode(null)
		accountService.cancelMicrosoftLogin().catch(console.error)
	}

	const resetForm = () => {
		cancelMicrosoftLogin()
		setUsername("")
		setPassword("")
		setOfflineName("")
		setErrorMessage(null)
		setSuccessMessage(null)
		setIsLoading(false)
	}

	const handleOpenChange = (nextOpen: boolean) => {
		if (!nextOpen) {
			resetForm()
		}
		onOpenChange(nextOpen)
	}

	const handleElyLogin = async () => {
		if (!username.trim() || !password) {
			setErrorMessage(t("addAccount.missingCredentials"))
			return
		}

		setIsLoading(true)
		setErrorMessage(null)

		try {
			const account = await accountService.elyLogin(username.trim(), password)
			setSuccessMessage(t("addAccount.welcomeBack", { name: account.username }))
			onAccountAdded?.(account)
			setTimeout(() => {
				handleOpenChange(false)
			}, 600)
		} catch (error) {
			setErrorMessage(typeof error === "string" ? error : t("addAccount.elyFailed"))
		} finally {
			setIsLoading(false)
		}
	}

	const handleMicrosoftLogin = async () => {
		const attempt = ++microsoftAttempt.current
		const isCurrent = () => attempt === microsoftAttempt.current
		setIsLoading(true)
		setErrorMessage(null)

		try {
			const code = await accountService.startMicrosoftLogin()
			if (!isCurrent()) return
			setDeviceCode(code)
			setIsLoading(false)

			const account = await accountService.finishMicrosoftLogin(code)
			if (!isCurrent()) return
			setDeviceCode(null)
			setSuccessMessage(t("addAccount.welcomeBack", { name: account.username }))
			onAccountAdded?.(account)
			setTimeout(() => {
				handleOpenChange(false)
			}, 600)
		} catch (error) {
			if (!isCurrent()) return
			setDeviceCode(null)
			setErrorMessage(typeof error === "string" ? error : t("addAccount.microsoftFailed"))
		} finally {
			if (isCurrent()) setIsLoading(false)
		}
	}

	const openMicrosoftPage = async () => {
		if (!deviceCode) return
		// Pre-copied so the user only has to paste it on the page
		await navigator.clipboard.writeText(deviceCode.userCode).catch(() => {})
		await openUrl(deviceCode.verificationUri).catch(console.error)
	}

	const handleOfflineLogin = async () => {
		if (!offlineName.trim()) {
			setErrorMessage(t("addAccount.missingNickname"))
			return
		}

		setIsLoading(true)
		setErrorMessage(null)

		try {
			const account = await accountService.addOfflineAccount(offlineName.trim())
			setSuccessMessage(t("addAccount.offlineCreated", { name: account.username }))
			onAccountAdded?.(account)
			setTimeout(() => {
				handleOpenChange(false)
			}, 500)
		} catch (error) {
			setErrorMessage(typeof error === "string" ? error : t("addAccount.offlineFailed"))
		} finally {
			setIsLoading(false)
		}
	}

	return (
		<Dialog open={open} onOpenChange={handleOpenChange}>
			<DialogContent className="border-border/60 bg-zinc-950/95 sm:max-w-md">
				<DialogHeader>
					<DialogTitle className="text-lg">{t("accounts.addAccount")}</DialogTitle>
					<DialogDescription>{t("addAccount.description")}</DialogDescription>
				</DialogHeader>

				<ScrollArea scrollFade className="min-h-0 w-full flex-1 pr-1">
					<div className="flex flex-col gap-4 py-1">
						{/* Account Type Selector */}
						<Tabs
							value={activeTab}
							onValueChange={(tab) => {
								if (tab !== "microsoft") cancelMicrosoftLogin()
								setActiveTab(tab as TabType)
								setErrorMessage(null)
							}}
						>
							<TabsList className="w-full">
								<TabsTrigger value="ely">Ely.by</TabsTrigger>
								<TabsTrigger value="offline">{t("accounts.offline")}</TabsTrigger>
								<TabsTrigger value="microsoft">Microsoft</TabsTrigger>
							</TabsList>
						</Tabs>

						{/* Ely.by Form */}
						{activeTab === "ely" && (
							<div className="grid gap-3.5 py-1">
								<Alert className={alertTone.success}>
									<ShieldCheck />
									<AlertDescription>{t("addAccount.tokensSecure")}</AlertDescription>
								</Alert>

								<div className="grid gap-1.5">
									<Label
										htmlFor="ely-username"
										className="font-medium text-muted-foreground text-xs"
									>
										{t("addAccount.usernameOrEmail")}
									</Label>
									<Input
										id="ely-username"
										placeholder={t("addAccount.usernamePlaceholder")}
										value={username}
										onChange={(e) => setUsername(e.target.value)}
										disabled={isLoading}
										autoFocus
									/>
								</div>

								<div className="grid gap-1.5">
									<Label
										htmlFor="ely-password"
										className="font-medium text-muted-foreground text-xs"
									>
										{t("addAccount.password")}
									</Label>
									<Input
										id="ely-password"
										type="password"
										placeholder="••••••••"
										value={password}
										onChange={(e) => setPassword(e.target.value)}
										disabled={isLoading}
										onKeyDown={(e) => {
											if (e.key === "Enter") handleElyLogin()
										}}
									/>
								</div>

								<Button
									size="default"
									onClick={handleElyLogin}
									disabled={isLoading || !username.trim() || !password}
									className="mt-1.5 w-full font-medium"
								>
									{isLoading ? (
										<>
											<Spinner className="mr-1.5 size-3.5" />
											{t("addAccount.connecting")}
										</>
									) : (
										t("addAccount.logIn")
									)}
								</Button>
							</div>
						)}

						{/* Offline Form */}
						{activeTab === "offline" && (
							<div className="grid gap-3.5 py-1">
								<div className="flex items-center gap-2 rounded-lg border border-border/40 bg-zinc-900/40 px-3 py-2 text-muted-foreground text-xs">
									<User className="size-4 shrink-0" />
									<span>{t("addAccount.offlineNote")}</span>
								</div>

								<div className="grid gap-1.5">
									<Label
										htmlFor="offline-name"
										className="font-medium text-muted-foreground text-xs"
									>
										{t("addAccount.nickname")}
									</Label>
									<Input
										id="offline-name"
										placeholder={t("addAccount.nicknamePlaceholder")}
										value={offlineName}
										onChange={(e) => setOfflineName(e.target.value)}
										disabled={isLoading}
										autoFocus
										onKeyDown={(e) => {
											if (e.key === "Enter") handleOfflineLogin()
										}}
									/>
								</div>

								<Button
									size="default"
									onClick={handleOfflineLogin}
									disabled={isLoading || !offlineName.trim()}
									className="mt-1.5 w-full font-medium"
								>
									{isLoading ? (
										<>
											<Spinner className="mr-1.5 size-3.5" />
											{t("addAccount.adding")}
										</>
									) : (
										t("addAccount.addProfile")
									)}
								</Button>
							</div>
						)}

						{/* Microsoft: device code sign-in */}
						{activeTab === "microsoft" && (
							<div className="grid gap-3.5 py-1">
								<Alert className={alertTone.success}>
									<ShieldCheck />
									<AlertDescription>{t("addAccount.microsoftNote")}</AlertDescription>
								</Alert>

								{deviceCode ? (
									<div className="flex flex-col items-center gap-3 rounded-lg border border-border/40 bg-zinc-900/40 px-3 py-4 text-center">
										<p className="text-muted-foreground text-xs">
											{t("addAccount.microsoftEnterCode")}
										</p>
										<p className="select-all font-bold font-mono text-2xl text-foreground tracking-[0.2em]">
											{deviceCode.userCode}
										</p>
										<Button size="default" onClick={openMicrosoftPage} className="w-full gap-1.5">
											<ExternalLink className="size-3.5" />
											{t("addAccount.microsoftOpenPage")}
										</Button>
										<p className="flex items-center gap-1.5 text-muted-foreground text-xs">
											<Spinner className="size-3.5" />
											{t("addAccount.microsoftWaiting")}
										</p>
									</div>
								) : (
									<Button
										size="default"
										onClick={handleMicrosoftLogin}
										disabled={isLoading}
										className="w-full font-medium"
									>
										{isLoading ? (
											<>
												<Spinner className="mr-1.5 size-3.5" />
												{t("addAccount.connecting")}
											</>
										) : (
											t("addAccount.microsoftSignIn")
										)}
									</Button>
								)}
							</div>
						)}

						{/* Error / Success Feedback */}
						{errorMessage && (
							<Alert variant="destructive">
								<AlertCircle />
								<AlertDescription>{errorMessage}</AlertDescription>
							</Alert>
						)}

						{successMessage && (
							<Alert className={alertTone.success}>
								<Check />
								<AlertDescription>{successMessage}</AlertDescription>
							</Alert>
						)}
					</div>
				</ScrollArea>
			</DialogContent>
		</Dialog>
	)
}

AddAccountDialog.displayName = "AddAccountDialog"

export default memo(AddAccountDialog)
