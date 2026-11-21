import { useCanGoBack, useNavigate, useRouter } from "@tanstack/react-router"
import { ArrowLeft } from "lucide-react"
import { useCallback, useMemo, useState } from "react"
import { useTranslation } from "react-i18next"
import { type AccountProfile } from "@/bindings"
import ElyAuthPanel from "@/features/accounts/components/ElyAuthPanel"
import MicrosoftAuthPanel from "@/features/accounts/components/MicrosoftAuthPanel"
import OfflineAuthPanel from "@/features/accounts/components/OfflineAuthPanel"
import SegmentedControl, { type SegmentedOption } from "@/shared/components/motion/SegmentedControl"
import { Button } from "@/shared/ui/button"
import { appleToast } from "@/shared/ui/sonner"

type AuthProviderId = "microsoft" | "elyBy" | "offline"

const renderProviderPanel = (
	provider: AuthProviderId,
	onSuccess: (profile: AccountProfile) => void,
) => {
	if (provider === "microsoft") {
		return <MicrosoftAuthPanel onSuccess={onSuccess} />
	}
	if (provider === "elyBy") {
		return <ElyAuthPanel onSuccess={onSuccess} />
	}
	return <OfflineAuthPanel onSuccess={onSuccess} />
}

const AddAccountPage = () => {
	const { t } = useTranslation()
	const navigate = useNavigate()
	const router = useRouter()
	const canGoBack = useCanGoBack()

	const [activeProvider, setActiveProvider] = useState<AuthProviderId>("microsoft")

	const providerOptions = useMemo<SegmentedOption<AuthProviderId>[]>(() => {
		return [
			{
				value: "microsoft",
				label: t("accounts.addPage.providers.microsoft"),
			},
			{
				value: "elyBy",
				label: t("accounts.addPage.providers.elyBy"),
			},
			{
				value: "offline",
				label: t("accounts.addPage.providers.offline"),
			},
		]
	}, [t])

	const handleBack = useCallback(() => {
		if (canGoBack) {
			router.history.back()
			return
		}
		void navigate({ to: "/settings/accounts" })
	}, [canGoBack, navigate, router])

	const handleAuthSuccess = useCallback(
		(profile: AccountProfile) => {
			appleToast.success(t("accounts.auth.successToast"), {
				description: profile.username,
			})
			void navigate({ to: "/settings/accounts" })
		},
		[navigate, t],
	)

	return (
		<div className="mx-auto flex w-full max-w-lg flex-col gap-4 sm:gap-5">
			<div className="flex items-start gap-3">
				<Button
					type="button"
					variant="outline"
					size="icon"
					aria-label={t("accounts.addPage.back")}
					onClick={handleBack}
					className="mt-0.5 shrink-0 cursor-pointer rounded-xl"
				>
					<ArrowLeft className="size-4" />
				</Button>
				<div className="min-w-0 flex-1">
					<h1 className="text-lg font-semibold tracking-tight text-foreground">
						{t("accounts.addPage.title")}
					</h1>
					<p className="text-xs text-muted-foreground">{t("accounts.addPage.subtitle")}</p>
				</div>
			</div>

			<SegmentedControl
				layoutId="add-account-provider-tabs"
				options={providerOptions}
				value={activeProvider}
				onChange={setActiveProvider}
				size="md"
				fullWidth
			/>

			<section
				role="tabpanel"
				className="rounded-2xl border border-border bg-card/65 p-4 backdrop-blur-md sm:p-6"
			>
				{renderProviderPanel(activeProvider, handleAuthSuccess)}
			</section>
		</div>
	)
}

export default AddAccountPage
