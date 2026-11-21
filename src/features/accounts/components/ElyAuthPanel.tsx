import { useForm } from "@tanstack/react-form"
import { Loader2, LogIn } from "lucide-react"
import { useEffect, useId, useRef, useState } from "react"
import { useTranslation } from "react-i18next"
import { type AccountProfile } from "@/bindings"
import { useElyLogin } from "@/features/settings/api/accountsApi"
import { getApiErrorMessage } from "@/shared/api/apiError"
import AnimatedCollapse from "@/shared/components/motion/AnimatedCollapse"
import { resolveFormError } from "@/shared/i18n/formI18n"
import { Button } from "@/shared/ui/button"
import { Input } from "@/shared/ui/input"
import { PasswordInput } from "@/shared/ui/password-input"
import { elyLoginSchema } from "../authSchemas"

interface ElyAuthPanelProps {
	onSuccess: (profile: AccountProfile) => void
}

const ElyAuthPanel = ({ onSuccess }: ElyAuthPanelProps) => {
	const { t } = useTranslation()
	const elyLoginMutation = useElyLogin()
	const usernameInputId = useId()
	const passwordInputId = useId()
	const usernameErrorId = useId()
	const passwordErrorId = useId()

	const isMountedRef = useRef(true)

	const [apiErrorMessage, setApiErrorMessage] = useState<string | null>(null)

	const form = useForm({
		defaultValues: {
			username: "",
			password: "",
		},
		validators: {
			onChange: elyLoginSchema,
			onSubmit: elyLoginSchema,
		},
		onSubmit: ({ value }) => {
			setApiErrorMessage(null)

			elyLoginMutation.mutate(
				{ username: value.username.trim(), password: value.password },
				{
					onSuccess: (profile) => {
						if (!isMountedRef.current) {
							return
						}
						form.setFieldValue("password", "")
						onSuccess(profile)
					},
					onError: (error) => {
						if (!isMountedRef.current) {
							return
						}
						setApiErrorMessage(getApiErrorMessage(error, t("accounts.auth.errorFallback")))
					},
				},
			)
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
					<LogIn className="size-4" />
				</div>
				<div className="flex min-w-0 flex-col gap-0.5">
					<h2 className="text-sm font-semibold text-foreground">{t("accounts.auth.ely.title")}</h2>
					<p className="text-xs leading-relaxed text-muted-foreground">
						{t("accounts.auth.ely.description")}
					</p>
				</div>
			</div>

			<div className="flex flex-col gap-3">
				<form.Field name="username">
					{(field) => {
						const shouldShowError =
							field.state.meta.isDirty ||
							field.state.meta.isTouched ||
							form.state.submissionAttempts > 0
						const fieldError = shouldShowError ? resolveFormError(t, field.state.meta.errors) : null
						const hasError = fieldError !== null

						return (
							<div className="flex flex-col gap-1.5">
								<label
									htmlFor={usernameInputId}
									className="text-[12px] font-medium text-foreground"
								>
									{t("accounts.auth.ely.usernameLabel")}
								</label>
								<Input
									id={usernameInputId}
									type="text"
									autoComplete="username"
									value={field.state.value}
									disabled={elyLoginMutation.isPending}
									aria-invalid={hasError}
									aria-describedby={hasError ? usernameErrorId : undefined}
									placeholder={t("accounts.auth.ely.usernamePlaceholder")}
									onBlur={field.handleBlur}
									onChange={(event) => {
										field.handleChange(event.target.value)
										if (apiErrorMessage !== null) {
											setApiErrorMessage(null)
										}
									}}
									className="h-9 rounded-xl"
								/>
								<AnimatedCollapse show={fieldError !== null}>
									<p id={usernameErrorId} role="alert" className="text-[11px] text-destructive">
										{fieldError}
									</p>
								</AnimatedCollapse>
							</div>
						)
					}}
				</form.Field>

				<form.Field name="password">
					{(field) => {
						const shouldShowError =
							field.state.meta.isDirty ||
							field.state.meta.isTouched ||
							form.state.submissionAttempts > 0
						const fieldError = shouldShowError ? resolveFormError(t, field.state.meta.errors) : null
						const hasError = fieldError !== null

						return (
							<div className="flex flex-col gap-1.5">
								<label
									htmlFor={passwordInputId}
									className="text-[12px] font-medium text-foreground"
								>
									{t("accounts.auth.ely.passwordLabel")}
								</label>
								<PasswordInput
									id={passwordInputId}
									autoComplete="current-password"
									value={field.state.value}
									disabled={elyLoginMutation.isPending}
									aria-invalid={hasError}
									aria-describedby={hasError ? passwordErrorId : undefined}
									placeholder={t("accounts.auth.ely.passwordPlaceholder")}
									onBlur={field.handleBlur}
									onChange={(event) => {
										field.handleChange(event.target.value)
										if (apiErrorMessage !== null) {
											setApiErrorMessage(null)
										}
									}}
									className="h-9 rounded-xl"
								/>
								<AnimatedCollapse show={fieldError !== null}>
									<p id={passwordErrorId} role="alert" className="text-[11px] text-destructive">
										{fieldError}
									</p>
								</AnimatedCollapse>
							</div>
						)
					}}
				</form.Field>
			</div>

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
								disabled={!canSubmit || elyLoginMutation.isPending}
								className="w-full cursor-pointer rounded-xl px-4 text-xs font-semibold sm:w-auto"
							>
								{elyLoginMutation.isPending ? (
									<Loader2 className="size-4 animate-spin" />
								) : (
									<LogIn className="size-4" />
								)}
								<span>{t("accounts.auth.ely.submit")}</span>
							</Button>
						)
					}}
				</form.Subscribe>
			</div>
		</form>
	)
}

export default ElyAuthPanel
