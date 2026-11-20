import { RotateCw } from "lucide-react"
import { useTranslation } from "react-i18next"
import WindowControlButtons from "@/shared/components/window/WindowControlButtons"
import { getSafeAppVersion } from "@/shared/lib/errorFormatting"

interface SafeTitlebarProps {
	statusLabel?: string
	onReload?: () => void
}

const SafeTitlebar = ({ statusLabel, onReload }: SafeTitlebarProps) => {
	const { t } = useTranslation()
	const appVersion = getSafeAppVersion()

	return (
		<header
			data-tauri-drag-region
			className="relative z-40 flex h-10 shrink-0 items-center justify-between gap-2 border-b border-border bg-sidebar/85 pr-0 pl-3 backdrop-blur-xl select-none"
		>
			<div data-tauri-drag-region className="flex shrink-0 items-center gap-2">
				<img
					src="/ingot.svg"
					alt={t("shell.logoAlt")}
					draggable={false}
					className="pointer-events-none size-5 shrink-0 object-contain"
				/>
				<span className="pointer-events-none text-[12.5px] font-semibold tracking-tight text-foreground">
					Ingot
				</span>
				{appVersion ? (
					<span className="pointer-events-none rounded-full border border-border bg-foreground/5 px-1.5 py-0.5 text-[9.5px] font-medium text-muted-foreground">
						v{appVersion}
					</span>
				) : null}
				{statusLabel ? (
					<span className="pointer-events-none rounded-full border border-rose-500/25 bg-rose-500/10 px-2 py-0.5 text-[10px] font-medium text-rose-500">
						{statusLabel}
					</span>
				) : null}
			</div>

			<div data-tauri-drag-region className="h-full min-w-0 flex-1" />

			<div data-tauri-drag-region className="flex shrink-0 items-center gap-1.5 pr-2 sm:pr-0">
				{onReload ? (
					<button
						type="button"
						onClick={onReload}
						className="inline-flex h-7 cursor-pointer items-center gap-1.5 rounded-full border border-border bg-foreground/5 px-2.5 text-[11px] font-medium text-foreground transition-colors hover:bg-foreground/10"
					>
						<RotateCw className="size-3" />
						<span>{t("shell.window.reload")}</span>
					</button>
				) : null}

				<WindowControlButtons className="hidden sm:flex" />
			</div>
		</header>
	)
}

export default SafeTitlebar
