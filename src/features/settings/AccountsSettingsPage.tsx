import { useNavigate } from "@tanstack/react-router"
import {
	AlertCircle,
	Check,
	Loader2,
	Plus,
	RefreshCw,
	ShieldCheck,
	Trash2,
	User,
} from "lucide-react"
import { useCallback, useState } from "react"
import { useTranslation } from "react-i18next"
import {
	ACCOUNT_TYPE_LABEL_KEYS,
	getAccountInitials,
	type UiAccountProfile,
} from "@/features/accounts/accountModel"
import { useActiveAccount } from "@/features/accounts/useActiveAccount"
import { useRemoveAccount, useSetActiveAccount } from "@/features/settings/api/accountsApi"
import { getApiErrorMessage } from "@/shared/api/apiError"
import { cn } from "@/shared/lib/utils"
import { Button } from "@/shared/ui/button"
import { appleToast } from "@/shared/ui/sonner"

interface AccountCardProps {
	account: UiAccountProfile
	isActive: boolean
	isConfirmingRemove: boolean
	isRemoving: boolean
	isSwitching: boolean
	onSelect: (accountId: string) => void
	onRequestRemove: (accountId: string) => void
	onCancelRemove: () => void
	onConfirmRemove: (accountId: string) => void
}

const AccountCard = ({
	account,
	isActive,
	isConfirmingRemove,
	isRemoving,
	isSwitching,
	onSelect,
	onRequestRemove,
	onCancelRemove,
	onConfirmRemove,
}: AccountCardProps) => {
	const { t } = useTranslation()
	const isBusy = isRemoving || isSwitching

	return (
		<div
			className={cn(
				"flex items-center gap-2 rounded-xl border p-2.5",
				isActive
					? "border-primary/45 bg-primary/12"
					: "border-border bg-foreground/3 hover:bg-foreground/6",
			)}
		>
			<button
				type="button"
				disabled={isBusy}
				onClick={() => {
					onSelect(account.id)
				}}
				className="flex min-w-0 flex-1 cursor-pointer items-center gap-2.5 text-left disabled:cursor-default disabled:opacity-60"
			>
				<div
					className={cn(
						"flex size-8 shrink-0 items-center justify-center rounded-lg bg-linear-to-br text-[11px] font-bold text-white",
						account.avatarBg,
					)}
				>
					{getAccountInitials(account.username)}
				</div>
				<div className="min-w-0 flex-1">
					<div className="flex items-center gap-1">
						<span className="truncate text-[12.5px] font-medium text-foreground">
							{account.username}
						</span>
						{account.type === "Microsoft" ? (
							<ShieldCheck className="size-3 shrink-0 text-primary" />
						) : null}
					</div>
					<span className="block truncate text-[10.5px] text-muted-foreground">
						{t(ACCOUNT_TYPE_LABEL_KEYS[account.type])} · {account.uuid.slice(0, 8)}
					</span>
				</div>
				{isActive && !isConfirmingRemove ? (
					<Check className="size-3.5 shrink-0 text-primary" />
				) : null}
			</button>

			{isConfirmingRemove ? (
				<div className="flex shrink-0 items-center gap-1">
					<Button
						type="button"
						variant="destructive"
						size="sm"
						disabled={isRemoving}
						onClick={() => {
							onConfirmRemove(account.id)
						}}
						className="h-7 cursor-pointer gap-1 rounded-lg bg-destructive/15 px-2 text-[11px] font-medium text-destructive hover:bg-destructive/25 disabled:cursor-default disabled:opacity-50"
					>
						{isRemoving ? <Loader2 className="size-3 animate-spin" /> : null}
						<span>
							{isRemoving ? t("accounts.actions.removing") : t("accounts.actions.confirmRemove")}
						</span>
					</Button>
					<Button
						type="button"
						variant="ghost"
						size="sm"
						disabled={isRemoving}
						onClick={onCancelRemove}
						className="h-7 cursor-pointer rounded-lg px-2 text-[11px] font-medium text-muted-foreground hover:bg-foreground/8 hover:text-foreground disabled:cursor-default disabled:opacity-50"
					>
						{t("common.cancel")}
					</Button>
				</div>
			) : (
				<Button
					type="button"
					variant="ghost"
					size="icon"
					disabled={isBusy}
					aria-label={t("accounts.actions.removeAria", { username: account.username })}
					title={t("accounts.actions.remove")}
					onClick={() => {
						onRequestRemove(account.id)
					}}
					className="size-7 shrink-0 cursor-pointer rounded-lg text-muted-foreground hover:bg-destructive/15 hover:text-destructive disabled:cursor-default disabled:opacity-50"
				>
					<Trash2 className="size-3.5" />
				</Button>
			)}
		</div>
	)
}

const AccountsSettingsPage = () => {
	const { t } = useTranslation()
	const navigate = useNavigate()
	const { accounts, activeAccount, isLoading, isError, error, refetch } = useActiveAccount()
	const setActiveAccountMutation = useSetActiveAccount()
	const removeAccountMutation = useRemoveAccount()

	const [confirmingRemoveId, setConfirmingRemoveId] = useState<string | null>(null)

	const handleAddAccount = useCallback(() => {
		void navigate({ to: "/accounts/add" })
	}, [navigate])

	const handleSelectAccount = useCallback(
		(accountId: string) => {
			if (accountId === activeAccount?.id) {
				return
			}
			setActiveAccountMutation.mutate(accountId, {
				onError: (mutationError) => {
					appleToast.error(getApiErrorMessage(mutationError, t("accounts.auth.errorFallback")))
				},
			})
		},
		[activeAccount?.id, setActiveAccountMutation, t],
	)

	const handleRequestRemove = useCallback((accountId: string) => {
		setConfirmingRemoveId(accountId)
	}, [])

	const handleCancelRemove = useCallback(() => {
		setConfirmingRemoveId(null)
	}, [])

	const handleConfirmRemove = useCallback(
		(accountId: string) => {
			removeAccountMutation.mutate(accountId, {
				onSuccess: () => {
					setConfirmingRemoveId((current) => {
						if (current === accountId) {
							return null
						}
						return current
					})
				},
				onError: (mutationError) => {
					appleToast.error(getApiErrorMessage(mutationError, t("accounts.auth.errorFallback")))
				},
			})
		},
		[removeAccountMutation, t],
	)

	const handleRetry = useCallback(() => {
		void refetch()
	}, [refetch])

	const renderBody = () => {
		if (isLoading) {
			return (
				<div className="flex items-center justify-center gap-2 rounded-xl border border-border bg-foreground/2 px-4 py-8 text-[12.5px] text-muted-foreground">
					<Loader2 className="size-4 animate-spin text-primary" />
					<span>{t("accounts.actions.loading")}</span>
				</div>
			)
		}

		if (isError) {
			const errorMessage = getApiErrorMessage(error, t("accounts.actions.loadError"))
			return (
				<div className="flex flex-col items-center justify-center gap-3 rounded-xl border border-destructive/30 bg-destructive/5 px-4 py-7 text-center">
					<div className="flex size-9 items-center justify-center rounded-xl bg-destructive/12 text-destructive">
						<AlertCircle className="size-4.5" />
					</div>
					<div className="space-y-1">
						<p className="text-[13px] font-medium text-foreground">
							{t("accounts.actions.loadError")}
						</p>
						<p className="max-w-md text-[11.5px] text-muted-foreground">{errorMessage}</p>
					</div>
					<Button
						type="button"
						variant="outline"
						size="sm"
						onClick={handleRetry}
						className="cursor-pointer rounded-xl px-3 text-[12px]"
					>
						<RefreshCw className="size-3.5 shrink-0" />
						<span>{t("accounts.actions.retry")}</span>
					</Button>
				</div>
			)
		}

		if (accounts.length === 0) {
			return (
				<div className="flex flex-col items-center justify-center gap-3 rounded-xl border border-dashed border-border bg-foreground/2 px-4 py-8 text-center">
					<div className="flex size-10 items-center justify-center rounded-xl bg-foreground/6 text-muted-foreground">
						<User className="size-5" />
					</div>
					<div className="space-y-1">
						<p className="text-[13px] font-semibold text-foreground">{t("accounts.empty.title")}</p>
						<p className="max-w-sm text-[11.5px] leading-relaxed text-muted-foreground">
							{t("accounts.empty.description")}
						</p>
					</div>
					<Button
						type="button"
						onClick={handleAddAccount}
						className="cursor-pointer rounded-xl px-3.5 text-[12px] font-semibold"
					>
						<Plus className="size-3.5 shrink-0" />
						<span>{t("settings.accounts.addAccount")}</span>
					</Button>
				</div>
			)
		}

		return (
			<div className="grid grid-cols-1 gap-2 sm:grid-cols-2 lg:grid-cols-3">
				{accounts.map((acc) => {
					const isActive = activeAccount !== null && acc.id === activeAccount.id
					const isConfirming = confirmingRemoveId === acc.id
					const isRemovingThis =
						removeAccountMutation.isPending && removeAccountMutation.variables === acc.id
					const isSwitchingThis =
						setActiveAccountMutation.isPending && setActiveAccountMutation.variables === acc.id

					return (
						<AccountCard
							key={acc.id}
							account={acc}
							isActive={isActive}
							isConfirmingRemove={isConfirming}
							isRemoving={isRemovingThis}
							isSwitching={isSwitchingThis}
							onSelect={handleSelectAccount}
							onRequestRemove={handleRequestRemove}
							onCancelRemove={handleCancelRemove}
							onConfirmRemove={handleConfirmRemove}
						/>
					)
				})}
			</div>
		)
	}

	return (
		<section className="flex flex-col gap-3 rounded-2xl border border-border bg-card/65 p-4 backdrop-blur-md">
			<div className="flex flex-wrap items-center justify-between gap-3">
				<div className="min-w-0 flex-1">
					<h2 className="text-[13.5px] font-semibold text-foreground">
						{t("settings.accounts.title")}
					</h2>
					<p className="text-[11.5px] leading-relaxed text-muted-foreground">
						{t("settings.accounts.description")}
					</p>
				</div>
				<Button
					type="button"
					onClick={handleAddAccount}
					className="cursor-pointer rounded-xl px-3 text-[12px] font-semibold"
				>
					<Plus className="size-3.5 shrink-0" />
					<span>{t("settings.accounts.addAccount")}</span>
				</Button>
			</div>

			{renderBody()}
		</section>
	)
}

export default AccountsSettingsPage
