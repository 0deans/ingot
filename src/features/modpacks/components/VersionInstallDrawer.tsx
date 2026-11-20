import { Download, X } from "lucide-react"
import { useTranslation } from "react-i18next"
import type { ModpackItem } from "@/features/modpacks/components/ModpackCard"
import { ENGINE_LABEL_KEYS } from "@/features/play/types"
import { formatRelativeLastPlayed } from "@/shared/stores/useInstancesStore"
import { Button } from "@/shared/ui/button"
import {
	Drawer,
	DrawerContent,
	DrawerDescription,
	DrawerHeader,
	DrawerTitle,
} from "@/shared/ui/drawer"

type ReleaseChannelKey =
	| "modpacks.channels.Release"
	| "modpacks.channels.Beta"
	| "modpacks.channels.LTS"

interface VersionEntry {
	label: string
	channelKey: ReleaseChannelKey
	date: string
}

interface VersionInstallDrawerProps {
	open: boolean
	pack: ModpackItem
	onClose: () => void
	onInstallVersion: (versionLabel: string) => void
}

const VersionInstallDrawer = ({
	open,
	pack,
	onClose,
	onInstallVersion,
}: VersionInstallDrawerProps) => {
	const { t } = useTranslation()

	const handleOpenChange = (nextOpen: boolean) => {
		if (!nextOpen) {
			onClose()
		}
	}

	const installTitle = t("modpacks.installTitle", { name: pack.name })
	const dismissAria = t("modpacks.dismissVersionDrawerAria")
	const versions: readonly VersionEntry[] = [
		{ label: `${pack.version}-latest`, channelKey: "modpacks.channels.Release", date: "Today" },
		{ label: `${pack.version}-rc2`, channelKey: "modpacks.channels.Beta", date: "4 days ago" },
		{ label: `${pack.version}-lts`, channelKey: "modpacks.channels.LTS", date: "2 weeks ago" },
	]

	return (
		<Drawer open={open} onOpenChange={handleOpenChange} showSwipeHandle>
			<DrawerContent
				aria-label={installTitle}
				className="ingot-glass-elevated mx-auto w-full max-w-md rounded-t-2xl border-border shadow-2xl"
			>
				<DrawerHeader className="flex-row items-center justify-between gap-3 border-b border-border pb-3 text-left">
					<div className="min-w-0 flex-1">
						<DrawerTitle className="truncate text-sm font-semibold text-foreground">
							{installTitle}
						</DrawerTitle>
						<DrawerDescription className="text-[11px] text-muted-foreground">
							{pack.loader} · {t(ENGINE_LABEL_KEYS[pack.engine])}
						</DrawerDescription>
					</div>
					<Button
						type="button"
						variant="ghost"
						size="icon-sm"
						onClick={onClose}
						aria-label={dismissAria}
						className="shrink-0 cursor-pointer text-muted-foreground hover:text-foreground"
					>
						<X className="size-4" />
					</Button>
				</DrawerHeader>

				<div className="flex flex-col gap-2 p-4">
					{versions.map((ver) => {
						return (
							<div
								key={ver.label}
								className="flex items-center justify-between gap-3 rounded-xl border border-border bg-foreground/4 px-3 py-2.5"
							>
								<div className="min-w-0 flex-1">
									<p className="truncate font-mono text-xs font-semibold text-foreground">
										{ver.label}
									</p>
									<p className="text-[10.5px] text-muted-foreground">
										{t(ver.channelKey)} · {formatRelativeLastPlayed(ver.date, t)}
									</p>
								</div>
								<Button
									type="button"
									size="sm"
									onClick={() => {
										onInstallVersion(ver.label)
										onClose()
									}}
									className="shrink-0 cursor-pointer px-2.5 text-[11px] font-semibold"
								>
									<Download className="size-3" />
									<span>{t("common.install")}</span>
								</Button>
							</div>
						)
					})}
				</div>
			</DrawerContent>
		</Drawer>
	)
}

export default VersionInstallDrawer
