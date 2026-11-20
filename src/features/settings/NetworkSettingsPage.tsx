import { Sliders } from "lucide-react"
import { useState } from "react"
import { useTranslation } from "react-i18next"
import { cn } from "@/shared/lib/utils"

const NetworkSettingsPage = () => {
	const { t } = useTranslation()
	const [discordRpc, setDiscordRpc] = useState(true)
	const [closeOnLaunch, setCloseOnLaunch] = useState(false)
	const [concurrentDownloads, setConcurrentDownloads] = useState(8)

	return (
		<section className="flex flex-col gap-3 rounded-2xl border border-border bg-card/65 p-4 backdrop-blur-md">
			<div className="flex items-center gap-2">
				<Sliders className="size-4 text-primary" />
				<h2 className="text-[13.5px] font-semibold text-foreground">
					{t("settings.network.title")}
				</h2>
			</div>

			<div className="flex items-center justify-between gap-3 rounded-xl border border-border bg-foreground/3 px-3.5 py-2.5">
				<div className="min-w-0 flex-1">
					<p className="text-[12.5px] font-medium text-foreground">
						{t("settings.network.discordTitle")}
					</p>
					<p className="text-[11px] leading-relaxed text-muted-foreground">
						{t("settings.network.discordDescription")}
					</p>
				</div>
				<button
					type="button"
					role="switch"
					aria-checked={discordRpc}
					onClick={() => {
						setDiscordRpc((prev) => {
							return !prev
						})
					}}
					className={cn(
						"shrink-0 cursor-pointer rounded-full px-3 py-1 text-[11px] font-semibold whitespace-nowrap transition-colors",
						discordRpc
							? "bg-primary text-primary-foreground"
							: "bg-foreground/10 text-muted-foreground",
					)}
				>
					{discordRpc ? t("common.enabled") : t("common.disabled")}
				</button>
			</div>

			<div className="flex items-center justify-between gap-3 rounded-xl border border-border bg-foreground/3 px-3.5 py-2.5">
				<div className="min-w-0 flex-1">
					<p className="text-[12.5px] font-medium text-foreground">
						{t("settings.network.minimizeTitle")}
					</p>
					<p className="text-[11px] leading-relaxed text-muted-foreground">
						{t("settings.network.minimizeDescription")}
					</p>
				</div>
				<button
					type="button"
					role="switch"
					aria-checked={closeOnLaunch}
					onClick={() => {
						setCloseOnLaunch((prev) => {
							return !prev
						})
					}}
					className={cn(
						"shrink-0 cursor-pointer rounded-full px-3 py-1 text-[11px] font-semibold whitespace-nowrap transition-colors",
						closeOnLaunch
							? "bg-primary text-primary-foreground"
							: "bg-foreground/10 text-muted-foreground",
					)}
				>
					{closeOnLaunch ? t("common.enabled") : t("common.disabled")}
				</button>
			</div>

			<div className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-border bg-foreground/3 px-3.5 py-2.5">
				<div className="min-w-0 flex-1">
					<p className="text-[12.5px] font-medium text-foreground">
						{t("settings.network.workersTitle")}
					</p>
					<p className="text-[11px] leading-relaxed text-muted-foreground">
						{t("settings.network.workersDescription")}
					</p>
				</div>
				<div className="flex shrink-0 items-center gap-1.5">
					{[4, 8, 12].map((count) => {
						return (
							<button
								key={count}
								type="button"
								onClick={() => {
									setConcurrentDownloads(count)
								}}
								className={cn(
									"cursor-pointer rounded-lg px-2.5 py-1 text-[11px] font-semibold transition-colors",
									concurrentDownloads === count
										? "bg-primary text-primary-foreground"
										: "bg-foreground/8 text-foreground/80 hover:bg-foreground/14",
								)}
							>
								{count}
							</button>
						)
					})}
				</div>
			</div>
		</section>
	)
}

export default NetworkSettingsPage
