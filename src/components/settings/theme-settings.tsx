import { Monitor, Moon, Palette, Sun } from "lucide-react"
import { memo } from "react"
import { useTranslation } from "react-i18next"
import { SectionCardHeader } from "@/components/common/section-card"
import { Card, CardContent } from "@/components/ui/card"
import {
	Field,
	FieldContent,
	FieldDescription,
	FieldLabel,
	FieldTitle,
} from "@/components/ui/field"
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group"
import { type ThemeMode, useTheme } from "@/lib/theme"

const THEME_OPTIONS: {
	id: ThemeMode
	icon: typeof Monitor
}[] = [
	{ id: "auto", icon: Monitor },
	{ id: "dark", icon: Moon },
	{ id: "light", icon: Sun },
]

export const ThemeSettings = () => {
	const { t } = useTranslation()
	const { themeMode, setThemeMode } = useTheme()

	return (
		<Card>
			<SectionCardHeader
				icon={Palette}
				title={t("settings.theme.title")}
				description={t("settings.theme.description")}
			/>
			<CardContent className="flex flex-col gap-3">
				<RadioGroup
					value={themeMode}
					onValueChange={(mode) => setThemeMode(mode as ThemeMode)}
					className="grid-cols-1 sm:grid-cols-3"
				>
					{THEME_OPTIONS.map(({ id, icon: Icon }) => (
						<FieldLabel key={id} htmlFor={`theme-${id}`}>
							<Field orientation="horizontal">
								<Icon className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
								<FieldContent>
									<FieldTitle>{t(`settings.theme.${id}`)}</FieldTitle>
									<FieldDescription>{t(`settings.theme.${id}Desc`)}</FieldDescription>
								</FieldContent>
								<RadioGroupItem value={id} id={`theme-${id}`} />
							</Field>
						</FieldLabel>
					))}
				</RadioGroup>
			</CardContent>
		</Card>
	)
}

ThemeSettings.displayName = "ThemeSettings"

export default memo(ThemeSettings)
