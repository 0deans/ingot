import { Check, Plus, ShieldCheck } from "lucide-react"
import { useTranslation } from "react-i18next"
import { useShallow } from "zustand/react/shallow"
import { cn } from "@/shared/lib/utils"
import {
	ACCOUNT_TYPE_LABEL_KEYS,
	getAccountInitials,
	useAccountStore,
} from "@/shared/stores/useAccountStore"
import { Button } from "@/shared/ui/button"

const AccountsSettingsPage = () => {
	const { t } = useTranslation()
	const { accounts, activeAccountId, setActiveAccountId, addAccount } = useAccountStore(
		useShallow((s) => {
			return {
				accounts: s.accounts,
				activeAccountId: s.activeAccountId,
				setActiveAccountId: s.setActiveAccountId,
				addAccount: s.addAccount,
			}
		}),
	)

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
					onClick={addAccount}
					className="cursor-pointer rounded-xl px-3 text-[12px] font-semibold"
				>
					<Plus className="size-3.5 shrink-0" />
					<span>{t("settings.accounts.addAccount")}</span>
				</Button>
			</div>

			<div className="grid grid-cols-1 gap-2 sm:grid-cols-2 lg:grid-cols-3">
				{accounts.map((acc) => {
					const isActive = acc.id === activeAccountId
					return (
						<button
							key={acc.id}
							type="button"
							onClick={() => {
								setActiveAccountId(acc.id)
							}}
							className={cn(
								"flex cursor-pointer items-center gap-2.5 rounded-xl border p-2.5 text-left transition-colors",
								isActive
									? "border-primary/45 bg-primary/12"
									: "border-border bg-foreground/3 hover:bg-foreground/6",
							)}
						>
							<div
								className={cn(
									"flex size-8 shrink-0 items-center justify-center rounded-lg bg-linear-to-br text-[11px] font-bold text-white",
									acc.avatarBg,
								)}
							>
								{getAccountInitials(acc.username)}
							</div>
							<div className="min-w-0 flex-1">
								<div className="flex items-center gap-1">
									<span className="truncate text-[12.5px] font-medium text-foreground">
										{acc.username}
									</span>
									{acc.type === "Microsoft" ? (
										<ShieldCheck className="size-3 shrink-0 text-primary" />
									) : null}
								</div>
								<span className="block truncate text-[10.5px] text-muted-foreground">
									{t(ACCOUNT_TYPE_LABEL_KEYS[acc.type])} · {acc.uuid.slice(0, 8)}
								</span>
							</div>
							{isActive ? <Check className="size-3.5 shrink-0 text-primary" /> : null}
						</button>
					)
				})}
			</div>
		</section>
	)
}

export default AccountsSettingsPage
