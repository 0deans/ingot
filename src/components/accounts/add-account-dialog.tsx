import { AlertCircle, Check, Loader2, ShieldCheck, User } from "lucide-react"
import { memo, useState } from "react"
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
import { accountService } from "@/services/account-service"
import type { AccountProfile } from "@/types/account"

interface AddAccountDialogProps {
	open: boolean
	onOpenChange: (open: boolean) => void
	onAccountAdded?: (account: AccountProfile) => void
}

type TabType = "ely" | "offline" | "microsoft"

const AddAccountDialog = ({ open, onOpenChange, onAccountAdded }: AddAccountDialogProps) => {
	const [activeTab, setActiveTab] = useState<TabType>("ely")
	const [username, setUsername] = useState("")
	const [password, setPassword] = useState("")
	const [offlineName, setOfflineName] = useState("")
	const [isLoading, setIsLoading] = useState(false)
	const [errorMessage, setErrorMessage] = useState<string | null>(null)
	const [successMessage, setSuccessMessage] = useState<string | null>(null)

	const resetForm = () => {
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
			setErrorMessage("Please enter both username/email and password.")
			return
		}

		setIsLoading(true)
		setErrorMessage(null)

		try {
			const account = await accountService.elyLogin(username.trim(), password)
			setSuccessMessage(`Welcome back, ${account.username}!`)
			onAccountAdded?.(account)
			setTimeout(() => {
				handleOpenChange(false)
			}, 600)
		} catch (error) {
			setErrorMessage(typeof error === "string" ? error : "Failed to connect to Ely.by")
		} finally {
			setIsLoading(false)
		}
	}

	const handleOfflineLogin = async () => {
		if (!offlineName.trim()) {
			setErrorMessage("Please enter a player nickname.")
			return
		}

		setIsLoading(true)
		setErrorMessage(null)

		try {
			const account = await accountService.addOfflineAccount(offlineName.trim())
			setSuccessMessage(`Created offline profile for ${account.username}`)
			onAccountAdded?.(account)
			setTimeout(() => {
				handleOpenChange(false)
			}, 500)
		} catch (error) {
			setErrorMessage(typeof error === "string" ? error : "Failed to create offline account")
		} finally {
			setIsLoading(false)
		}
	}

	return (
		<Dialog open={open} onOpenChange={handleOpenChange}>
			<DialogContent className="border-border/60 bg-zinc-950/95 sm:max-w-md">
				<DialogHeader>
					<DialogTitle className="text-lg">Add Account</DialogTitle>
					<DialogDescription>
						Connect your Ely.by account or configure an offline profile to play.
					</DialogDescription>
				</DialogHeader>

				{/* Account Type Selector */}
				<div className="flex rounded-lg border border-border/40 bg-zinc-900/60 p-1">
					<button
						type="button"
						onClick={() => {
							setActiveTab("ely")
							setErrorMessage(null)
						}}
						className={`flex-1 rounded-md py-1.5 font-medium text-xs transition-all ${
							activeTab === "ely"
								? "bg-primary text-primary-foreground shadow-xs"
								: "text-muted-foreground hover:text-foreground"
						}`}
					>
						Ely.by
					</button>
					<button
						type="button"
						onClick={() => {
							setActiveTab("offline")
							setErrorMessage(null)
						}}
						className={`flex-1 rounded-md py-1.5 font-medium text-xs transition-all ${
							activeTab === "offline"
								? "bg-primary text-primary-foreground shadow-xs"
								: "text-muted-foreground hover:text-foreground"
						}`}
					>
						Offline
					</button>
					<button
						type="button"
						onClick={() => {
							setActiveTab("microsoft")
							setErrorMessage(null)
						}}
						className={`flex-1 rounded-md py-1.5 font-medium text-xs transition-all ${
							activeTab === "microsoft"
								? "bg-primary text-primary-foreground shadow-xs"
								: "text-muted-foreground hover:text-foreground"
						}`}
					>
						Microsoft
					</button>
				</div>

				{/* Ely.by Form */}
				{activeTab === "ely" && (
					<div className="grid gap-3.5 py-1">
						<div className="flex items-center gap-2 rounded-lg border border-emerald-500/20 bg-emerald-500/10 px-3 py-2 text-emerald-400 text-xs">
							<ShieldCheck className="size-4 shrink-0" />
							<span>Tokens are securely encrypted in your OS Credential Vault.</span>
						</div>

						<div className="grid gap-1.5">
							<label htmlFor="ely-username" className="font-medium text-muted-foreground text-xs">
								Username or Email
							</label>
							<Input
								id="ely-username"
								placeholder="e.g. notch or player@ely.by"
								value={username}
								onChange={(e) => setUsername(e.target.value)}
								disabled={isLoading}
								autoFocus
							/>
						</div>

						<div className="grid gap-1.5">
							<label htmlFor="ely-password" className="font-medium text-muted-foreground text-xs">
								Password
							</label>
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
					</div>
				)}

				{/* Offline Form */}
				{activeTab === "offline" && (
					<div className="grid gap-3.5 py-1">
						<div className="flex items-center gap-2 rounded-lg border border-border/40 bg-zinc-900/40 px-3 py-2 text-muted-foreground text-xs">
							<User className="size-4 shrink-0" />
							<span>Offline accounts play without an online session token.</span>
						</div>

						<div className="grid gap-1.5">
							<label htmlFor="offline-name" className="font-medium text-muted-foreground text-xs">
								Player Nickname
							</label>
							<Input
								id="offline-name"
								placeholder="e.g. Steve"
								value={offlineName}
								onChange={(e) => setOfflineName(e.target.value)}
								disabled={isLoading}
								autoFocus
								onKeyDown={(e) => {
									if (e.key === "Enter") handleOfflineLogin()
								}}
							/>
						</div>
					</div>
				)}

				{/* Microsoft Info */}
				{activeTab === "microsoft" && (
					<div className="flex flex-col items-center justify-center gap-2 py-4 text-center">
						<p className="font-medium text-foreground text-sm">Microsoft / Xbox Live Login</p>
						<p className="max-w-xs text-muted-foreground text-xs">
							Official Mojang Microsoft account OAuth integration will be available in an upcoming
							release.
						</p>
					</div>
				)}

				{/* Error / Success Feedback */}
				{errorMessage && (
					<div className="flex items-center gap-2 rounded-lg border border-destructive/30 bg-destructive/10 px-3 py-2 text-destructive text-xs">
						<AlertCircle className="size-4 shrink-0" />
						<span>{errorMessage}</span>
					</div>
				)}

				{successMessage && (
					<div className="flex items-center gap-2 rounded-lg border border-emerald-500/30 bg-emerald-500/10 px-3 py-2 text-emerald-400 text-xs">
						<Check className="size-4 shrink-0" />
						<span>{successMessage}</span>
					</div>
				)}

				<DialogFooter>
					<Button
						variant="outline"
						size="sm"
						onClick={() => handleOpenChange(false)}
						disabled={isLoading}
					>
						Cancel
					</Button>
					{activeTab === "ely" && (
						<Button
							size="sm"
							onClick={handleElyLogin}
							disabled={isLoading || !username.trim() || !password}
						>
							{isLoading ? (
								<>
									<Loader2 className="mr-1.5 size-3.5 animate-spin" />
									Connecting...
								</>
							) : (
								"Log In to Ely.by"
							)}
						</Button>
					)}
					{activeTab === "offline" && (
						<Button
							size="sm"
							onClick={handleOfflineLogin}
							disabled={isLoading || !offlineName.trim()}
						>
							{isLoading ? (
								<>
									<Loader2 className="mr-1.5 size-3.5 animate-spin" />
									Adding...
								</>
							) : (
								"Add Profile"
							)}
						</Button>
					)}
				</DialogFooter>
			</DialogContent>
		</Dialog>
	)
}

AddAccountDialog.displayName = "AddAccountDialog"

export default memo(AddAccountDialog)
