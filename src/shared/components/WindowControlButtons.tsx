import { Copy, Minus, Square, X } from "lucide-react"
import { useCallback, useEffect, useState } from "react"
import { useTranslation } from "react-i18next"
import {
	closeMainWindow,
	isTauriRuntime,
	minimizeMainWindow,
	queryMainWindowMaximized,
	toggleMaximizeMainWindow,
} from "@/shared/lib/tauriCommands"
import { subscribeWindowResized } from "@/shared/lib/tauriEvents"
import { cn } from "@/shared/lib/utils"

interface WindowControlButtonsProps {
	className?: string
}

const WindowControlButtons = ({ className }: WindowControlButtonsProps) => {
	const { t } = useTranslation()
	const [isMaximized, setIsMaximized] = useState(false)

	const handleMinimize = useCallback(() => {
		void minimizeMainWindow().catch((error: unknown) => {
			console.error("Failed to minimize window:", error)
		})
	}, [])

	const handleToggleMaximize = useCallback(() => {
		void toggleMaximizeMainWindow().catch((error: unknown) => {
			console.error("Failed to toggle maximize window:", error)
		})
	}, [])

	const handleClose = useCallback(() => {
		void closeMainWindow().catch((error: unknown) => {
			console.error("Failed to close window:", error)
		})
	}, [])

	useEffect(() => {
		if (!isTauriRuntime()) {
			return
		}

		let active = true
		let unlisten: (() => void) | undefined

		const syncMaximized = async () => {
			try {
				const maximized = await queryMainWindowMaximized()
				if (active) {
					setIsMaximized(maximized)
				}
			} catch (error: unknown) {
				console.error("Failed to query window maximized state:", error)
			}
		}

		void syncMaximized()

		void subscribeWindowResized(() => {
			void syncMaximized()
		})
			.then((dispose) => {
				if (active) {
					unlisten = dispose
				} else {
					dispose()
				}
			})
			.catch((error: unknown) => {
				console.error("Failed to subscribe to window resize events:", error)
			})

		return () => {
			active = false
			if (unlisten) {
				unlisten()
			}
		}
	}, [])

	return (
		<div className={cn("ml-1 flex h-10 items-center", className)}>
			<button
				type="button"
				onClick={handleMinimize}
				aria-label={t("shell.window.minimize")}
				className="flex h-10 w-10 cursor-pointer items-center justify-center text-muted-foreground transition-colors hover:bg-foreground/8 hover:text-foreground"
			>
				<Minus className="size-3.5" />
			</button>
			<button
				type="button"
				onClick={handleToggleMaximize}
				aria-label={isMaximized ? t("shell.window.restore") : t("shell.window.maximize")}
				className="flex h-10 w-10 cursor-pointer items-center justify-center text-muted-foreground transition-colors hover:bg-foreground/8 hover:text-foreground"
			>
				{isMaximized ? <Copy className="size-3" /> : <Square className="size-3" />}
			</button>
			<button
				type="button"
				onClick={handleClose}
				aria-label={t("shell.window.close")}
				className="flex h-10 w-10 cursor-pointer items-center justify-center text-muted-foreground transition-colors hover:bg-rose-600 hover:text-white"
			>
				<X className="size-3.5" />
			</button>
		</div>
	)
}

export default WindowControlButtons
