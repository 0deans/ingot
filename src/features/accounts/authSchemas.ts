import { z } from "zod"
import { i18nMessage } from "@/shared/i18n/formI18n"

export const OFFLINE_USERNAME_MIN_LENGTH = 3
export const OFFLINE_USERNAME_MAX_LENGTH = 16
export const OFFLINE_USERNAME_PATTERN = /^[A-Za-z0-9_]+$/

export const elyLoginSchema = z.object({
	username: z.string().trim().min(1, i18nMessage("accounts.auth.ely.usernameRequired")),
	password: z.string().min(1, i18nMessage("accounts.auth.ely.passwordRequired")),
})

export type ElyLoginValues = z.infer<typeof elyLoginSchema>

const offlineUsernameSchema = z
	.string()
	.trim()
	.min(OFFLINE_USERNAME_MIN_LENGTH, i18nMessage("accounts.auth.offline.usernameTooShort"))
	.max(OFFLINE_USERNAME_MAX_LENGTH, i18nMessage("accounts.auth.offline.usernameTooLong"))
	.regex(OFFLINE_USERNAME_PATTERN, i18nMessage("accounts.auth.offline.usernameInvalidChars"))

export const offlineAccountSchema = z.object({
	username: offlineUsernameSchema,
})

export const createOfflineAccountSchema = (existingUsernames: ReadonlySet<string>) => {
	return z.object({
		username: offlineUsernameSchema.refine((value) => {
			return !existingUsernames.has(value.toLowerCase())
		}, i18nMessage("accounts.auth.offline.usernameExists")),
	})
}

export type OfflineAccountValues = z.infer<typeof offlineAccountSchema>
