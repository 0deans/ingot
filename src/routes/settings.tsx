import { createFileRoute } from "@tanstack/react-router"
import { Check, Plus, ShieldCheck, Trash2 } from "lucide-react"
import { memo, useState } from "react"
import AddAccountDialog from "@/components/accounts/add-account-dialog"
import MemoryAllocation from "@/components/settings/memory-allocation"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import SkinAvatar from "@/components/ui/skin-avatar"
import { useAccounts } from "@/services/account-service"

const SettingsPage = () => {
	const { accounts, setActiveAccount, removeAccount } = useAccounts()
	const [isAddOpen, setIsAddOpen] = useState(false)

	const handleSetActive = async (id: string) => {
		try {
			await setActiveAccount(id)
		} catch (error) {
			console.error("Failed to set active account:", error)
		}
	}

	const handleRemove = async (id: string) => {
		try {
			await removeAccount(id)
		} catch (error) {
			console.error("Failed to remove account:", error)
		}
	}

	return (
		<div className="flex max-w-3xl flex-1 flex-col gap-8 pb-8">
			<div>
				<h1 className="font-bold text-2xl text-foreground tracking-tight">Launcher Settings</h1>
				<p className="text-muted-foreground text-sm">
					Configure accounts, Java runtimes, RAM allocation, and launcher behavior.
				</p>
			</div>

			<div className="flex flex-col gap-6">
				{/* Accounts & Security */}
				<div className="flex flex-col gap-3 rounded-xl border border-border/40 bg-zinc-900/40 p-5">
					<div className="flex items-center justify-between">
						<div>
							<h3 className="font-semibold text-foreground text-sm">Accounts & Authentication</h3>
							<p className="text-muted-foreground text-xs">
								Connect your Ely.by or offline accounts. Sensitive tokens are secured in the OS
								Credential Vault.
							</p>
						</div>
						<Button size="sm" onClick={() => setIsAddOpen(true)} className="gap-1.5 text-xs">
							<Plus className="size-3.5" />
							Add Account
						</Button>
					</div>

					<div className="flex items-center gap-2 rounded-lg border border-emerald-500/20 bg-emerald-500/5 px-3 py-2 text-emerald-400 text-xs">
						<ShieldCheck className="size-4 shrink-0" />
						<span>
							Native OS Keyring active: Tokens are encrypted using Windows Credential Manager
							(DPAPI) / Keychain.
						</span>
					</div>

					<div className="mt-2 flex flex-col divide-y divide-border/30 rounded-lg border border-border/40 bg-zinc-950/60">
						{accounts.length === 0 ? (
							<div className="p-4 text-center text-muted-foreground text-xs">
								No accounts configured yet. Click "Add Account" to get started with Ely.by.
							</div>
						) : (
							accounts.map((acc) => (
								<div key={acc.id} className="flex items-center justify-between p-3.5">
									<div className="flex items-center gap-3">
										<SkinAvatar username={acc.username} skinUrl={acc.skinUrl} size={36} />
										<div className="flex flex-col">
											<div className="flex items-center gap-2">
												<span className="font-semibold text-foreground text-sm">
													{acc.username}
												</span>
												{acc.accountType === "ely" && (
													<span className="inline-flex items-center rounded-xs bg-emerald-500/15 px-1.5 py-0.5 font-medium text-[10px] text-emerald-400">
														Ely.by
													</span>
												)}
												{acc.accountType === "offline" && (
													<span className="inline-flex items-center rounded-xs bg-zinc-800 px-1.5 py-0.5 font-medium text-[10px] text-zinc-400">
														Offline
													</span>
												)}
												{acc.isActive && (
													<span className="inline-flex items-center gap-1 rounded-xs bg-primary/20 px-1.5 py-0.5 font-medium text-[10px] text-primary">
														<Check className="size-2.5" /> Active
													</span>
												)}
											</div>
											<span className="font-mono text-[11px] text-muted-foreground">
												UUID: {acc.uuid}
											</span>
										</div>
									</div>

									<div className="flex items-center gap-2">
										{!acc.isActive && (
											<Button variant="outline" size="xs" onClick={() => handleSetActive(acc.id)}>
												Set Active
											</Button>
										)}
										<Button
											variant="ghost"
											size="icon-xs"
											className="text-muted-foreground hover:text-destructive"
											onClick={() => handleRemove(acc.id)}
											title="Remove account"
										>
											<Trash2 className="size-3.5" />
										</Button>
									</div>
								</div>
							))
						)}
					</div>
				</div>

				{/* Java Runtime */}
				<div className="flex flex-col gap-2 rounded-xl border border-border/40 bg-zinc-900/40 p-5">
					<h3 className="font-semibold text-foreground text-sm">Default Java Runtime</h3>
					<p className="text-muted-foreground text-xs">
						Java 21 is required for Minecraft 1.20.5 and newer.
					</p>
					<div className="mt-2 flex gap-3">
						<Input
							defaultValue="C:\Program Files\Eclipse Adoptium\jdk-21.0.3.9-hotspot\bin\javaw.exe"
							className="font-mono text-xs"
						/>
						<Button variant="outline">Browse</Button>
					</div>
				</div>

				{/* Memory Allocation */}
				<MemoryAllocation />

				{/* Window & Launch Behavior */}
				<div className="flex flex-col gap-2 rounded-xl border border-border/40 bg-zinc-900/40 p-5">
					<h3 className="font-semibold text-foreground text-sm">Launcher Behavior</h3>
					<p className="text-muted-foreground text-xs">
						Choose what happens when an instance is launched.
					</p>
					<div className="mt-3 flex gap-2">
						<Button size="sm" variant="outline">
							Keep Launcher Open
						</Button>
						<Button size="sm" variant="outline">
							Hide Launcher to System Tray
						</Button>
					</div>
				</div>
			</div>

			<AddAccountDialog open={isAddOpen} onOpenChange={setIsAddOpen} />
		</div>
	)
}

SettingsPage.displayName = "SettingsPage"

const MemoizedSettingsPage = memo(SettingsPage)

export const Route = createFileRoute("/settings")({
	component: MemoizedSettingsPage,
})
