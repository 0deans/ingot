import { type ParseKeys, type TFunction } from "i18next"

const I18N_MESSAGE_PREFIX = "\u0000i18n:"

export type I18nMessageParams = Record<string, string | number | boolean>

const isI18nMessageParams = (value: unknown): value is I18nMessageParams => {
	if (typeof value !== "object" || value === null || Array.isArray(value)) {
		return false
	}

	return Object.values(value).every((entry) => {
		return typeof entry === "string" || typeof entry === "number" || typeof entry === "boolean"
	})
}

export const i18nMessage = <K extends ParseKeys>(key: K, params?: I18nMessageParams): string => {
	if (params === undefined) {
		return key
	}

	return `${I18N_MESSAGE_PREFIX}${JSON.stringify({ key, params })}`
}

export const extractFirstFormError = (errors: ReadonlyArray<unknown>): string | null => {
	const firstError = errors[0]
	if (typeof firstError === "string") {
		return firstError
	}

	if (
		typeof firstError === "object" &&
		firstError !== null &&
		"message" in firstError &&
		typeof firstError.message === "string"
	) {
		return firstError.message
	}

	return null
}

export const resolveFormError = (t: TFunction, errors: ReadonlyArray<unknown>): string | null => {
	const raw = extractFirstFormError(errors)
	if (raw === null) {
		return null
	}

	if (raw.startsWith(I18N_MESSAGE_PREFIX)) {
		try {
			const parsed: unknown = JSON.parse(raw.slice(I18N_MESSAGE_PREFIX.length))
			if (
				typeof parsed === "object" &&
				parsed !== null &&
				"key" in parsed &&
				typeof parsed.key === "string" &&
				"params" in parsed &&
				isI18nMessageParams(parsed.params)
			) {
				return t(parsed.key, { ...parsed.params, defaultValue: parsed.key })
			}
		} catch {
			return raw
		}

		return raw
	}

	return t(raw, { defaultValue: raw })
}
