import { openUrl } from "@tauri-apps/plugin-opener"
import type { TFunction } from "i18next"
import { ExternalLink, Scale } from "lucide-react"
import { useState } from "react"
import { useTranslation } from "react-i18next"
import { FadeScroll } from "@/components/servers/shared/primitives"
import { Button } from "@/components/ui/button"
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog"
import type { TranslationKey } from "@/i18n"

interface Component {
	name: string
	/** What Ingot uses it for */
	role: TranslationKey
	license: string
	source: string
}

interface Group {
	title: TranslationKey
	note: TranslationKey
	items: Component[]
}

/** Texts starting with "licenses." are locale keys; the rest (names, SPDX ids) stay as they are */
const text = (t: TFunction, s: string) => (s.startsWith("licenses.") ? t(s as TranslationKey) : s)

/**
 * Third-party software Ingot ships or downloads and runs. Keep in sync with
 * src-tauri/android/jniLibs/README.md (bundled files) and the download code.
 */
const GROUPS: Group[] = [
	{
		title: "licenses.groups.android.title",
		note: "licenses.groups.android.note",
		items: [
			{
				name: "PRoot",
				role: "licenses.roles.proot",
				license: "GPL-2.0",
				source: "https://github.com/termux/proot",
			},
			{
				name: "talloc",
				role: "licenses.roles.talloc",
				license: "LGPL-3.0-or-later",
				source: "https://talloc.samba.org",
			},
			{
				name: "libandroid-shmem",
				role: "licenses.roles.shmem",
				license: "BSD-3-Clause",
				source: "https://github.com/termux/libandroid-shmem",
			},
		],
	},
	{
		title: "licenses.groups.downloaded.title",
		note: "licenses.groups.downloaded.note",
		items: [
			{
				name: "Eclipse Temurin (OpenJDK)",
				role: "licenses.roles.temurin",
				license: "GPL-2.0 with Classpath Exception",
				source: "https://adoptium.net",
			},
			{
				name: "Alpine Linux",
				role: "licenses.roles.alpine",
				license: "licenses.various",
				source: "https://alpinelinux.org",
			},
			{
				name: "Pumpkin",
				role: "licenses.roles.pumpkin",
				license: "GPL-3.0",
				source: "https://github.com/Pumpkin-MC/Pumpkin",
			},
			{
				name: "playit.gg agent",
				role: "licenses.roles.playit",
				license: "BSD-2-Clause",
				source: "https://github.com/playit-cloud/playit-agent",
			},
		],
	},
	{
		title: "licenses.groups.servers.title",
		note: "licenses.groups.servers.note",
		items: [
			{
				name: "Paper",
				role: "licenses.roles.paper",
				license: "GPL-3.0",
				source: "https://github.com/PaperMC/Paper",
			},
			{
				name: "Folia",
				role: "licenses.roles.folia",
				license: "GPL-3.0",
				source: "https://github.com/PaperMC/Folia",
			},
			{
				name: "Purpur",
				role: "licenses.roles.purpur",
				license: "MIT",
				source: "https://github.com/PurpurMC/Purpur",
			},
			{
				name: "Fabric Loader",
				role: "licenses.roles.fabric",
				license: "Apache-2.0",
				source: "https://github.com/FabricMC/fabric-loader",
			},
			{
				name: "licenses.names.minecraftServer",
				role: "licenses.roles.vanilla",
				license: "Minecraft EULA",
				source: "https://aka.ms/MinecraftEULA",
			},
		],
	},
]

/** Lists the third-party software Ingot ships or downloads, with licenses and sources */
export function LicensesDialog({
	open,
	onOpenChange,
}: {
	open: boolean
	onOpenChange: (open: boolean) => void
}) {
	const { t } = useTranslation()
	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent className="flex max-h-[85vh] flex-col gap-3 p-0 sm:max-w-lg">
				<div className="px-5 pt-5">
					<DialogTitle className="text-base">{t("settings.licenses.title")}</DialogTitle>
					<p className="mt-1 text-muted-foreground text-xs leading-relaxed">
						{t("settings.licenses.dialogSubtitle")}
					</p>
				</div>
				<FadeScroll className="flex min-h-0 flex-col gap-5 px-5 pb-5">
					{GROUPS.map((group) => (
						<section key={group.title} className="flex flex-col gap-2">
							<div>
								<h3 className="font-semibold text-foreground text-sm">{t(group.title)}</h3>
								<p className="text-[11px] text-muted-foreground">{t(group.note)}</p>
							</div>
							<ul className="flex flex-col divide-y divide-border/70 border border-border/70">
								{group.items.map((item) => (
									<li key={item.name}>
										<button
											type="button"
											onClick={() => openUrl(item.source).catch(console.error)}
											className="flex w-full items-center gap-3 px-3 py-2.5 text-left transition-colors hover:bg-card/60"
										>
											<div className="min-w-0 flex-1">
												<p className="truncate font-medium text-foreground text-sm">
													{text(t, item.name)}
												</p>
												<p className="truncate text-[11px] text-muted-foreground">{t(item.role)}</p>
											</div>
											<span className="shrink-0 bg-card px-1.5 py-0.5 font-mono text-[10px] text-foreground/80">
												{text(t, item.license)}
											</span>
											<ExternalLink className="size-3.5 shrink-0 text-muted-foreground/60" />
										</button>
									</li>
								))}
							</ul>
						</section>
					))}
					<p className="text-[11px] text-muted-foreground/60 leading-relaxed">
						{t("settings.licenses.trademark")}
					</p>
				</FadeScroll>
			</DialogContent>
		</Dialog>
	)
}

/** Settings card that opens the licenses list */
export function LicensesSettings() {
	const { t } = useTranslation()
	const [open, setOpen] = useState(false)
	return (
		<div className="flex items-center justify-between gap-4 border border-border/30 bg-background/40 p-4">
			<div className="flex min-w-0 items-center gap-3">
				<Scale className="size-4 shrink-0 text-muted-foreground" />
				<div className="min-w-0">
					<p className="font-medium text-sm">{t("settings.licenses.title")}</p>
					<p className="text-muted-foreground text-xs">{t("settings.licenses.description")}</p>
				</div>
			</div>
			<Button variant="outline" size="sm" onClick={() => setOpen(true)} className="shrink-0">
				{t("common.view")}
			</Button>
			<LicensesDialog open={open} onOpenChange={setOpen} />
		</div>
	)
}
