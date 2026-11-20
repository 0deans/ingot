import { Sliders } from "lucide-react"
import { useState } from "react"
import { useTranslation } from "react-i18next"
import { Button } from "@/shared/ui/button"
import { Switch } from "@/shared/ui/switch"

const WORKER_OPTIONS = [4, 8, 12]

const IntegrationsNetworkCard = () => {
	const { t } = useTranslation()
	const [discordRpc, setDiscordRpc] = useState(true)
	const [concurrentDownloads, setConcurrentDownloads] = useState(8)

	return (
		<section className="@container flex flex-col gap-3 rounded-2xl border border-border bg-card/65 p-3.5 backdrop-blur-md sm:p-4">
			<div className="flex items-center gap-2.5">
				<div className="flex size-7 shrink-0 items-center justify-center rounded-md border border-border bg-foreground/5 text-primary">
					<Sliders className="size-4" />
				</div>
				<h2 className="text-[13.5px] font-semibold text-foreground">
					{t("settings.network.title")}
				</h2>
			</div>

			<div className="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-border bg-foreground/3 px-3.5 py-2.5">
				<div className="min-w-0 flex-1">
					<p className="text-[12.5px] font-medium text-foreground">
						{t("settings.network.discordTitle")}
					</p>
					<p className="text-[11px] leading-relaxed text-muted-foreground">
						{t("settings.network.discordDescription")}
					</p>
				</div>
				<Switch
					checked={discordRpc}
					onCheckedChange={setDiscordRpc}
					aria-label={t("settings.network.discordTitle")}
				/>
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
					{WORKER_OPTIONS.map((count) => {
						const isSelected = concurrentDownloads === count
						return (
							<Button
								key={count}
								type="button"
								variant={isSelected ? "default" : "secondary"}
								size="xs"
								onClick={() => {
									setConcurrentDownloads(count)
								}}
								className="cursor-pointer rounded-md px-2.5 text-[11px] font-semibold"
							>
								{count}
							</Button>
						)
					})}
				</div>
			</div>
		</section>
	)
}

export default IntegrationsNetworkCard
