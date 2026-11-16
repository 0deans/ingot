import { useTheme } from "next-themes"
import { useEffect } from "react"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"
import {
	applyCustomThemeVariables,
	clearCustomThemeVariables,
} from "@/shared/theme/applyCustomTheme"

const CustomThemeSync = () => {
	const { theme } = useTheme()
	const customTheme = useLauncherPrefsStore((s) => {
		return s.customTheme
	})

	useEffect(() => {
		if (theme === "custom") {
			applyCustomThemeVariables(customTheme)
			return
		}
		clearCustomThemeVariables()
	}, [customTheme, theme])

	return null
}

export default CustomThemeSync
