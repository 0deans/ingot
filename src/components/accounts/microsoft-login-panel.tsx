import { AlertCircle, ExternalLink, ShieldCheck } from "lucide-react"
import { useEffect } from "react"
import { useTranslation } from "react-i18next"
import type { AccountProfile, PendingLoginStatus } from "@/bindings"
import { alertTone } from "@/components/common/alert-tones"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import { Spinner } from "@/components/ui/spinner"
import type { TranslationKey } from "@/i18n"
import { useMicrosoftLogin } from "./use-microsoft-login"

const pendingMessages: Record<PendingLoginStatus, TranslationKey> = {
	Ready: "addAccount.microsoftPending",
	Busy: "addAccount.microsoftPendingBusy",
	Expired: "addAccount.microsoftPendingExpired",
	Legacy: "addAccount.microsoftPendingExpired",
}

export function MicrosoftLoginPanel({
	onSuccess,
	onBusyChange,
}: {
	onSuccess: (account: AccountProfile) => void
	onBusyChange: (busy: boolean) => void
}) {
	const { t } = useTranslation()
	const login = useMicrosoftLogin(onSuccess)
	useEffect(() => {
		onBusyChange(login.busy)
	}, [login.busy, onBusyChange])
	useEffect(() => () => onBusyChange(false), [onBusyChange])
	return (
		<div className="grid gap-3.5 py-1">
			<Alert className={alertTone.success}>
				<ShieldCheck />
				<AlertDescription>{t("addAccount.microsoftNote")}</AlertDescription>
			</Alert>
			{login.pending && !login.deviceCode && (
				<div className="grid gap-3 rounded-lg border border-border/40 bg-card/40 px-3 py-4">
					<p className="text-muted-foreground text-sm">
						{t(pendingMessages[login.pending.status])}
					</p>
					<div className="flex gap-2">
						{login.pending.status === "Ready" && (
							<Button onClick={login.resume} disabled={login.disabled}>
								{t("addAccount.microsoftResume")}
							</Button>
						)}
						<Button variant="outline" onClick={login.discard} disabled={login.disabled}>
							{t("addAccount.microsoftDiscard")}
						</Button>
					</div>
				</div>
			)}
			{login.deviceCode ? (
				<div className="flex flex-col items-center gap-3 rounded-lg border border-border/40 bg-card/40 px-3 py-4 text-center">
					<p className="text-muted-foreground text-xs">{t("addAccount.microsoftEnterCode")}</p>
					<p className="select-all font-bold font-mono text-2xl text-foreground tracking-[0.2em]">
						{login.deviceCode.userCode}
					</p>
					<Button onClick={login.openPage} className="w-full gap-1.5">
						<ExternalLink className="size-3.5" />
						{t("addAccount.microsoftOpenPage")}
					</Button>
					<p className="flex items-center gap-1.5 text-muted-foreground text-xs">
						<Spinner className="size-3.5" />
						{t("addAccount.microsoftWaiting")}
					</p>
				</div>
			) : (
				<Button onClick={login.start} disabled={login.disabled} className="w-full font-medium">
					{login.busy ? (
						<>
							<Spinner className="mr-1.5 size-3.5" />
							{t("addAccount.connecting")}
						</>
					) : (
						t("addAccount.microsoftSignIn")
					)}
				</Button>
			)}
			{login.error && (
				<Alert variant="destructive">
					<AlertCircle />
					<AlertDescription>{login.error}</AlertDescription>
				</Alert>
			)}
		</div>
	)
}
