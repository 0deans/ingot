import { Check, Settings, ShieldCheck, Trash2, UserPlus } from "lucide-react"
import { memo, useState } from "react"
import AddAccountDialog from "@/components/accounts/add-account-dialog"
import { Button } from "@/components/ui/button"
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuGroup,
	DropdownMenuItem,
	DropdownMenuLabel,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import SkinAvatar from "@/components/ui/skin-avatar"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { useAccounts } from "@/services/account-service"

interface AccountSwitcherProps {
	compact?: boolean
	onOpenSettings?: () => void
}

const AccountSwitcher = ({ compact = false, onOpenSettings }: AccountSwitcherProps) => {
	const { accounts, activeAccount, setActiveAccount, removeAccount } = useAccounts()
	const [isAddOpen, setIsAddOpen] = useState(false)

	const handleSelectAccount = async (id: string) => {
		try {
			await setActiveAccount(id)
		} catch (error) {
			console.error("Failed to switch account:", error)
		}
	}

	const handleRemoveAccount = async (e: React.MouseEvent, id: string) => {
		e.stopPropagation()
		try {
			await removeAccount(id)
		} catch (error) {
			console.error("Failed to remove account:", error)
		}
	}

	return (
		<>
			<DropdownMenu>
				{compact ? (
					<Tooltip>
						<TooltipTrigger
							render={
								<DropdownMenuTrigger
									aria-label="User profile"
									className="group relative flex size-10 items-center justify-center rounded-xl border border-border/40 bg-zinc-900/40 outline-none transition-all hover:border-primary/50 hover:bg-zinc-800/80 focus-visible:ring-2 focus-visible:ring-primary/40"
								>
									<SkinAvatar
										username={activeAccount ? activeAccount.username : "Player"}
										skinUrl={activeAccount?.skinUrl}
										size={28}
									/>
									{activeAccount?.accountType === "ely" && (
										<span
											className="absolute -right-0.5 -bottom-0.5 size-2.5 rounded-full border-2 border-zinc-950 bg-emerald-500"
											title="Secured via Ely.by"
										/>
									)}
								</DropdownMenuTrigger>
							}
						/>
						<TooltipContent side="right">
							{activeAccount ? activeAccount.username : "Add Account"}
						</TooltipContent>
					</Tooltip>
				) : (
					<DropdownMenuTrigger
						render={
							<button
								type="button"
								className="group flex w-full items-center gap-2.5 rounded-xl border border-border/40 bg-zinc-900/60 p-2 text-left transition-colors hover:border-border/80 hover:bg-zinc-800/80 focus:outline-none"
							/>
						}
					>
						<SkinAvatar
							username={activeAccount ? activeAccount.username : "Player"}
							skinUrl={activeAccount?.skinUrl}
							size={34}
						/>
						<div className="flex min-w-0 flex-1 flex-col">
							<div className="flex items-center gap-1.5">
								<span className="truncate font-semibold text-foreground text-xs">
									{activeAccount ? activeAccount.username : "No Account"}
								</span>
								{activeAccount?.accountType === "ely" && (
									<span className="inline-flex items-center rounded-xs bg-emerald-500/10 px-1 py-0.2 text-[9px] text-emerald-400">
										Ely.by
									</span>
								)}
								{activeAccount?.accountType === "offline" && (
									<span className="inline-flex items-center rounded-xs bg-zinc-800 px-1 py-0.2 text-[9px] text-zinc-400">
										Offline
									</span>
								)}
							</div>
							<span className="flex items-center gap-1 text-[10px] text-muted-foreground">
								{activeAccount?.accountType === "ely" ? (
									<>
										<ShieldCheck className="size-2.5 text-emerald-500/80" />
										Secured in Vault
									</>
								) : (
									"Local Profile"
								)}
							</span>
						</div>
					</DropdownMenuTrigger>
				)}

				<DropdownMenuContent
					align="end"
					side={compact ? "right" : "top"}
					sideOffset={8}
					className="w-60 border-border/60 bg-zinc-950/95 p-1.5 shadow-xl backdrop-blur-md"
				>
					<DropdownMenuGroup>
						<DropdownMenuLabel className="flex items-center justify-between text-[11px]">
							<span>Minecraft Accounts</span>
							<span className="font-normal text-[10px] text-muted-foreground">
								{accounts.length} linked
							</span>
						</DropdownMenuLabel>

						{accounts.length === 0 ? (
							<div className="p-2 text-center text-muted-foreground text-xs">
								No accounts configured yet.
							</div>
						) : (
							accounts.map((acc) => {
								const isCurrent = activeAccount?.id === acc.id
								return (
									<DropdownMenuItem
										key={acc.id}
										onClick={() => handleSelectAccount(acc.id)}
										className="group/item flex items-center justify-between gap-2 py-1.5"
									>
										<div className="flex items-center gap-2">
											<SkinAvatar username={acc.username} skinUrl={acc.skinUrl} size={24} />
											<div className="flex flex-col">
												<span className="font-medium text-xs">{acc.username}</span>
												<div className="flex items-center gap-1">
													<span className="text-[9px] text-muted-foreground capitalize">
														{acc.accountType}
													</span>
													{acc.accountType === "ely" && (
														<ShieldCheck className="size-2.5 text-emerald-500" />
													)}
												</div>
											</div>
										</div>

										<div className="flex items-center gap-1">
											{isCurrent && <Check className="size-3.5 text-primary" />}
											<Button
												variant="ghost"
												size="icon-xs"
												className="opacity-0 transition-opacity hover:text-destructive group-hover/item:opacity-100"
												onClick={(e) => handleRemoveAccount(e, acc.id)}
												title="Remove account"
											>
												<Trash2 className="size-3" />
											</Button>
										</div>
									</DropdownMenuItem>
								)
							})
						)}
					</DropdownMenuGroup>

					<DropdownMenuSeparator />

					<DropdownMenuItem onClick={() => setIsAddOpen(true)} className="gap-2 text-xs">
						<UserPlus className="size-3.5" />
						<span>Add Account...</span>
					</DropdownMenuItem>

					{onOpenSettings && (
						<DropdownMenuItem onClick={onOpenSettings} className="gap-2 text-xs">
							<Settings className="size-3.5" />
							<span>Manage Accounts</span>
						</DropdownMenuItem>
					)}
				</DropdownMenuContent>
			</DropdownMenu>

			<AddAccountDialog open={isAddOpen} onOpenChange={setIsAddOpen} />
		</>
	)
}

AccountSwitcher.displayName = "AccountSwitcher"

export default memo(AccountSwitcher)
