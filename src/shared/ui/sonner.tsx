import {
	CircleCheckIcon,
	InfoIcon,
	Loader2Icon,
	OctagonXIcon,
	TriangleAlertIcon,
} from "lucide-react"
import { useTheme } from "next-themes"
import { type CSSProperties } from "react"
import { Toaster as Sonner, type ToasterProps } from "sonner"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"
import { type CustomThemeBaseMode } from "@/shared/theme/customTheme"

const resolveSonnerTheme = (
	activeTheme: string | undefined,
	customBaseMode: CustomThemeBaseMode,
): ToasterProps["theme"] => {
	if (activeTheme === "light") {
		return "light"
	}
	if (activeTheme === "purple") {
		return "dark"
	}
	if (activeTheme === "custom") {
		return customBaseMode
	}
	return "dark"
}

const TOASTER_STYLE: CSSProperties & Record<string, string> = {
	"--normal-bg": "var(--popover)",
	"--normal-text": "var(--popover-foreground)",
	"--normal-border": "var(--border)",
	"--border-radius": "var(--radius)",
}

const Toaster = ({ ...props }: ToasterProps) => {
	const { theme } = useTheme()
	const customBaseMode = useLauncherPrefsStore((s) => {
		return s.customTheme.baseMode
	})

	const sonnerTheme = resolveSonnerTheme(theme, customBaseMode)

	return (
		<Sonner
			theme={sonnerTheme}
			className="toaster group"
			icons={{
				success: <CircleCheckIcon className="size-4" />,
				info: <InfoIcon className="size-4" />,
				warning: <TriangleAlertIcon className="size-4" />,
				error: <OctagonXIcon className="size-4" />,
				loading: <Loader2Icon className="size-4 animate-spin" />,
			}}
			style={TOASTER_STYLE}
			toastOptions={{
				classNames: {
					toast: "cn-toast",
				},
			}}
			{...props}
		/>
	)
}

export { Toaster }
