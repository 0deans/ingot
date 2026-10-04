import { openUrl } from "@tauri-apps/plugin-opener"
import { useEffect, useRef, useState } from "react"
import { useTranslation } from "react-i18next"
import type { AccountProfile, MicrosoftDeviceCode, PendingMicrosoftLogin } from "@/bindings"
import { accountService } from "@/services/account-service"

export function useMicrosoftLogin(onSuccess: (account: AccountProfile) => void) {
	const { t } = useTranslation()
	const [deviceCode, setDeviceCode] = useState<MicrosoftDeviceCode | null>(null)
	const [pending, setPending] = useState<PendingMicrosoftLogin | null>(null)
	const [phase, setPhase] = useState<"idle" | "working" | "complete">("idle")
	const busy = phase === "working"
	const [error, setError] = useState<string | null>(null)
	const generation = useRef(0)
	const active = useRef(false)
	const fallback = useRef(t("addAccount.microsoftFailed"))
	useEffect(() => {
		fallback.current = t("addAccount.microsoftFailed")
	}, [t])

	useEffect(() => {
		const attempt = ++generation.current
		accountService.getPendingMicrosoftLogin().then(
			(value) => {
				if (attempt === generation.current) setPending(value)
			},
			(failure: unknown) => {
				if (attempt === generation.current)
					setError(typeof failure === "string" ? failure : fallback.current)
			},
		)
		return () => {
			generation.current++
			if (active.current) {
				active.current = false
				accountService.cancelMicrosoftLogin().catch(console.error)
			}
		}
	}, [])

	const run = async (login: () => Promise<AccountProfile>) => {
		if (active.current || phase !== "idle") return
		const attempt = ++generation.current
		active.current = true
		setPhase("working")
		setError(null)
		try {
			const account = await login()
			if (attempt !== generation.current) return
			active.current = false
			setDeviceCode(null)
			setPending(null)
			setPhase("complete")
			onSuccess(account)
		} catch (failure) {
			if (attempt !== generation.current) return
			active.current = false
			setDeviceCode(null)
			setError(typeof failure === "string" ? failure : t("addAccount.microsoftFailed"))
			// Keep interrupted grants available after closing this dialog. A failed
			// inspection must not turn a storage error into "no pending sign-in".
			try {
				const value = await accountService.getPendingMicrosoftLogin()
				if (attempt === generation.current) setPending(value)
			} catch (inspectionFailure) {
				if (attempt === generation.current)
					setError(
						typeof inspectionFailure === "string"
							? inspectionFailure
							: t("addAccount.microsoftFailed"),
					)
			}
		} finally {
			if (attempt === generation.current)
				setPhase((value) => (value === "complete" ? value : "idle"))
		}
	}

	const start = () =>
		run(async () => {
			setPending(null)
			const attempt = generation.current
			const code = await accountService.startMicrosoftLogin()
			if (attempt !== generation.current) throw new Error("Sign-in cancelled")
			setDeviceCode(code)
			return accountService.finishMicrosoftLogin(code)
		})

	const resume = () => {
		if (pending?.status !== "Ready") return
		const ticket = pending.ticket
		return run(() => accountService.resumeMicrosoftLogin(ticket))
	}

	const discard = async () => {
		if (!pending || busy) return
		const attempt = ++generation.current
		setPhase("working")
		setError(null)
		try {
			await accountService.discardPendingMicrosoftLogin(pending.ticket)
			if (attempt === generation.current) setPending(null)
		} catch (failure) {
			if (attempt === generation.current)
				setError(typeof failure === "string" ? failure : t("addAccount.microsoftFailed"))
		} finally {
			if (attempt === generation.current) setPhase("idle")
		}
	}

	const openPage = async () => {
		if (!deviceCode) return
		await navigator.clipboard.writeText(deviceCode.userCode).catch(() => {})
		await openUrl(deviceCode.verificationUri).catch(console.error)
	}

	return {
		deviceCode,
		pending,
		busy,
		disabled: phase !== "idle",
		error,
		start,
		resume,
		discard,
		openPage,
	}
}
