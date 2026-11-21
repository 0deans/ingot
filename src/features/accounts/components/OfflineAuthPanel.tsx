import { useForm } from "@tanstack/react-form"
import { Loader2, UserPlus } from "lucide-react"
import { useEffect, useId, useMemo, useRef, useState } from "react"
import { useTranslation } from "react-i18next"
import { type AccountProfile } from "@/bindings"
import { useActiveAccount } from "@/features/accounts/useActiveAccount"
import { useAddOfflineAccount } from "@/features/settings/api/accountsApi"
import { getApiErrorMessage } from "@/shared/api/apiError"
import AnimatedCollapse from "@/shared/components/motion/AnimatedCollapse"
import { resolveFormError } from "@/shared/i18n/formI18n"
import { Button } from "@/shared/ui/button"
import { Input } from "@/shared/ui/input"
import { createOfflineAccountSchema, OFFLINE_USERNAME_MAX_LENGTH } from "../authSchemas"

interface OfflineAuthPanelProps {
	onSuccess: (profile: AccountProfile) => void
}

const OfflineAuthPanel = ({ onSuccess }: OfflineAuthPanelProps) => {
	const { t } = useTranslation()
	const { accounts } = useActiveAccount()
	const addOfflineMutation = useAddOfflineAccount()
	const usernameInputId = useId()
	const hintId = useId()
	const usernameErrorId = useId()

	const isMountedRef = useRef(true)

	const [apiErrorMessage, setApiErrorMessage] = useState<string | null>(null)

	const existingOfflineUsernames = useMemo(() => {
		const offlineUsernames = accounts
			.filter((account) => {
				return account.type === "Offline"
			})
			.map((account) => {
				return account.username.toLowerCase()
			})

		return new Set(offlineUsernames)
	}, [accounts])

	const schema = useMemo(() => {
		return createOfflineAccountSchema(existingOfflineUsernames)
	}, [existingOfflineUsernames])

	const form = useForm({
		defaultValues: {
			username: "",
		},
		validators: {
			onChange: schema,
			onSubmit: schema,
		},
		onSubmit: ({ value }) => {
			setApiErrorMessage(null)

			addOfflineMutation.mutate(value.username.trim(), {
				onSuccess: (profile) => {
					if (!isMountedRef.current) {
						return
					}
					onSuccess(profile)
				},
				onError: (error) => {
					if (!isMountedRef.current) {
						return
					}
					setApiErrorMessage(getApiErrorMessage(error, t("accounts.auth.errorFallback")))
				},
			})
		},
	})

	useEffect(() => {
		isMountedRef.current = true
		return () => {
			isMountedRef.current = false
		}
	}, [])

	return (
		<form
			onSubmit={(event) => {
				event.preventDefault()
				event.stopPropagation()
				void form.handleSubmit()
			}}
			className="flex flex-col gap-4"
		>
			<div className="flex items-start gap-3">
				<div className="flex size-9 shrink-0 items-center justify-center rounded-xl border border-border/70 bg-background/60 text-primary">
					<UserPlus className="size-4" />
				</div>
				<div className="flex min-w-0 flex-col gap-0.5">
					<h2 className="text-sm font-semibold text-foreground">
						{t("accounts.auth.offline.title")}
					</h2>
					<p className="text-xs leading-relaxed text-muted-foreground">
						{t("accounts.auth.offline.description")}
					</p>
				</div>
			</div>

			<form.Field name="username">
				{(field) => {
					const shouldShowError =
						field.state.meta.isDirty ||
						field.state.meta.isTouched ||
						form.state.submissionAttempts > 0
					const fieldError = shouldShowError ? resolveFormError(t, field.state.meta.errors) : null
					const hasError = fieldError !== null || apiErrorMessage !== null

					return (
						<div className="flex flex-col gap-1.5">
							<label htmlFor={usernameInputId} className="text-[12px] font-medium text-foreground">
								{t("accounts.auth.offline.usernameLabel")}
							</label>
							<Input
								id={usernameInputId}
								type="text"
								maxLength={OFFLINE_USERNAME_MAX_LENGTH}
								autoComplete="off"
								aria-describedby={fieldError !== null ? `${hintId} ${usernameErrorId}` : hintId}
								aria-invalid={hasError}
								value={field.state.value}
								disabled={addOfflineMutation.isPending}
								placeholder={t("accounts.auth.offline.usernamePlaceholder")}
								onBlur={field.handleBlur}
								onChange={(event) => {
									field.handleChange(event.target.value)
									if (apiErrorMessage !== null) {
										setApiErrorMessage(null)
									}
								}}
								className="h-9 rounded-xl"
							/>
							<p id={hintId} className="text-[11px] text-muted-foreground">
								{t("accounts.auth.offline.usernameHint")}
							</p>
							<AnimatedCollapse show={fieldError !== null}>
								<p id={usernameErrorId} role="alert" className="text-[11px] text-destructive">
									{fieldError}
								</p>
							</AnimatedCollapse>
						</div>
					)
				}}
			</form.Field>

			<AnimatedCollapse show={apiErrorMessage !== null}>
				<p
					role="alert"
					className="rounded-xl border border-destructive/30 bg-destructive/10 px-3 py-2 text-[12px] text-destructive"
				>
					{apiErrorMessage}
				</p>
			</AnimatedCollapse>

			<div>
				<form.Subscribe
					selector={(state) => {
						return state.canSubmit
					}}
				>
					{(canSubmit) => {
						return (
							<Button
								type="submit"
								size="lg"
								disabled={!canSubmit || addOfflineMutation.isPending}
								className="w-full cursor-pointer rounded-xl px-4 text-xs font-semibold sm:w-auto"
							>
								{addOfflineMutation.isPending ? (
									<Loader2 className="size-4 animate-spin" />
								) : (
									<UserPlus className="size-4" />
								)}
								<span>{t("accounts.auth.offline.submit")}</span>
							</Button>
						)
					}}
				</form.Subscribe>
			</div>
		</form>
	)
}

export default OfflineAuthPanel
