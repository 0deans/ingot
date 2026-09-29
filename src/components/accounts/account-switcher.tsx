import { Check, Eye, Settings, ShieldCheck, Trash2, UserPlus } from "lucide-react"
import { memo, useState } from "react"
import { useTranslation } from "react-i18next"
import { AccountTypeBadge } from "@/components/accounts/account-type-badge"
import AddAccountDialog from "@/components/accounts/add-account-dialog"
import SkinPreviewDialog from "@/components/accounts/skin-preview-dialog"
import SkinAvatar from "@/components/common/skin-avatar"
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
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { useAccounts } from "@/services/account-service"
import type { AccountProfile } from "@/types/account"

interface AccountSwitcherProps {
	compact?: boolean
	onOpenSettings?: () => void
}

const AccountSwitcher = ({ compact = false, onOpenSettings }: AccountSwitcherProps) => {
	const { t } = useTranslation()
	const { accounts, activeAccount, setActiveAccount, removeAccount } = useAccounts()
	const [isAddOpen, setIsAddOpen] = useState(false)
	const [previewAccount, setPreviewAccount] = useState<AccountProfile | null>(null)

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
									aria-label={t("accounts.userProfile")}
									className="group relative flex size-10 items-center justify-center rounded-lg outline-none transition-transform hover:scale-105 focus-visible:ring-2 focus-visible:ring-primary/40"
								>
									<SkinAvatar
										username={activeAccount ? activeAccount.username : t("accounts.player")}
										skinUrl={activeAccount?.skinUrl}
										size={32}
										className="transition-all group-hover:ring-2 group-hover:ring-primary/40"
									/>
									{activeAccount?.accountType === "ely" && (
										<span
											className="absolute right-0.5 bottom-0.5 size-2.5 rounded-full border-2 border-zinc-950 bg-emerald-500"
											title={t("accounts.securedViaEly")}
										/>
									)}
								</DropdownMenuTrigger>
							}
						/>
						<TooltipContent side="right">
							{activeAccount ? activeAccount.username : t("accounts.addAccount")}
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
							username={activeAccount ? activeAccount.username : t("accounts.player")}
							skinUrl={activeAccount?.skinUrl}
							size={34}
						/>
						<div className="flex min-w-0 flex-1 flex-col">
							<div className="flex items-center gap-1.5">
								<span className="truncate font-semibold text-foreground text-xs">
									{activeAccount ? activeAccount.username : t("accounts.noAccount")}
								</span>
								{activeAccount && <AccountTypeBadge type={activeAccount.accountType} />}
							</div>
							<span className="flex items-center gap-1 text-[10px] text-muted-foreground">
								{activeAccount?.accountType === "ely" ||
								activeAccount?.accountType === "microsoft" ? (
									<>
										<ShieldCheck className="size-2.5 text-emerald-500/80" />
										{t("accounts.securedInVault")}
									</>
								) : (
									t("accounts.localProfile")
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
							<span>{t("accounts.minecraftAccounts")}</span>
							<span className="font-normal text-[10px] text-muted-foreground">
								{t("accounts.linkedCount", { count: accounts.length })}
							</span>
						</DropdownMenuLabel>

						{accounts.length === 0 ? (
							<div className="p-2 text-center text-muted-foreground text-xs">
								{t("accounts.noAccountsYet")}
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
													{acc.accountType !== "offline" && (
														<ShieldCheck className="size-2.5 text-emerald-500" />
													)}
												</div>
											</div>
										</div>

										<div className="flex items-center gap-1">
											<Button
												variant="ghost"
												size="icon-xs"
												className="any-pointer-coarse:opacity-100 opacity-0 transition-opacity hover:text-primary focus-visible:opacity-100 group-hover/item:opacity-100 group-data-highlighted/item:opacity-100"
												onClick={(e) => {
													e.stopPropagation()
													setPreviewAccount(acc)
												}}
												title={t("accounts.previewSkin")}
											>
												<Eye className="size-3" />
											</Button>
											{isCurrent && <Check className="size-3.5 text-primary" />}
											<Button
												variant="ghost"
												size="icon-xs"
												className="any-pointer-coarse:opacity-100 opacity-0 transition-opacity hover:text-destructive focus-visible:opacity-100 group-hover/item:opacity-100 group-data-highlighted/item:opacity-100"
												onClick={(e) => handleRemoveAccount(e, acc.id)}
												title={t("accounts.removeAccount")}
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
						<span>{t("accounts.addAccountEllipsis")}</span>
					</DropdownMenuItem>

					{onOpenSettings && (
						<DropdownMenuItem onClick={onOpenSettings} className="gap-2 text-xs">
							<Settings className="size-3.5" />
							<span>{t("accounts.manageAccounts")}</span>
						</DropdownMenuItem>
					)}
				</DropdownMenuContent>
			</DropdownMenu>

			<AddAccountDialog open={isAddOpen} onOpenChange={setIsAddOpen} />
			<SkinPreviewDialog
				account={previewAccount}
				open={Boolean(previewAccount)}
				onOpenChange={(open) => !open && setPreviewAccount(null)}
			/>
		</>
	)
}

AccountSwitcher.displayName = "AccountSwitcher"

export default memo(AccountSwitcher)
