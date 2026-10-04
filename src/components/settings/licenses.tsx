import { openUrl } from "@tauri-apps/plugin-opener"
import type { TFunction } from "i18next"
import { ExternalLink, Scale } from "lucide-react"
import { useState } from "react"
import { useTranslation } from "react-i18next"
import { SectionCardHeader } from "@/components/common/section-card"
import { FadeScroll } from "@/components/servers/shared/primitives"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card } from "@/components/ui/card"
import { Dialog, DialogContent, DialogTitle } from "@/components/ui/dialog"
import { Item, ItemActions, ItemContent, ItemDescription, ItemTitle } from "@/components/ui/item"
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
				<FadeScroll className="min-h-0 flex-1">
					<div className="flex flex-col gap-5 px-5 pb-5">
						{GROUPS.map((group) => (
							<section key={group.title} className="flex flex-col gap-2">
								<div>
									<h3 className="font-semibold text-foreground text-sm">{t(group.title)}</h3>
									<p className="text-2xs text-muted-foreground">{t(group.note)}</p>
								</div>
								<ul className="flex flex-col divide-y divide-border/70 border border-border/70">
									{group.items.map((item) => (
										<li key={item.name}>
											<Item
												render={
													<button
														type="button"
														onClick={() => openUrl(item.source).catch(console.error)}
													/>
												}
												className="rounded-none text-left hover:bg-muted/50"
											>
												<ItemContent>
													<ItemTitle>{text(t, item.name)}</ItemTitle>
													<ItemDescription>{t(item.role)}</ItemDescription>
												</ItemContent>
												<ItemActions>
													<Badge variant="secondary" className="font-mono">
														{text(t, item.license)}
													</Badge>
													<ExternalLink className="size-3.5 text-muted-foreground" />
												</ItemActions>
											</Item>
										</li>
									))}
								</ul>
							</section>
						))}
						<p className="text-2xs text-muted-foreground/60 leading-relaxed">
							{t("settings.licenses.trademark")}
						</p>
					</div>
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
		<Card>
			<SectionCardHeader
				icon={Scale}
				title={t("settings.licenses.title")}
				description={t("settings.licenses.description")}
				action={
					<Button variant="outline" size="sm" onClick={() => setOpen(true)}>
						{t("common.view")}
					</Button>
				}
			/>
			<LicensesDialog open={open} onOpenChange={setOpen} />
		</Card>
	)
}
