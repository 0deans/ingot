import { Link, useNavigate } from "@tanstack/react-router"
import { ArrowLeft, Download } from "lucide-react"
import { useState } from "react"
import { useTranslation } from "react-i18next"
import { MODPACK_CATALOG } from "@/features/modpacks/components/ModpackCard"
import ModpackGallery from "@/features/modpacks/components/ModpackGallery"
import VersionInstallDrawer from "@/features/modpacks/components/VersionInstallDrawer"
import NotFoundView from "@/shared/components/errors/NotFoundView"
import { useInstancesStore } from "@/shared/stores/useInstancesStore"
import { useProcessStore } from "@/shared/stores/useProcessStore"

interface ModpackDetailPageProps {
	projectId: string
}

const ModpackDetailPage = ({ projectId }: ModpackDetailPageProps) => {
	const { t } = useTranslation()
	const navigate = useNavigate()
	const selectOrCreateByName = useInstancesStore((s) => {
		return s.selectOrCreateByName
	})
	const spawnMockProcess = useProcessStore((s) => {
		return s.spawnMockProcess
	})

	const [drawerOpen, setDrawerOpen] = useState(false)

	const pack = MODPACK_CATALOG.find((item) => {
		return item.id === projectId
	})

	if (!pack) {
		return <NotFoundView />
	}

	const handleInstallVersion = (versionLabel: string) => {
		selectOrCreateByName({
			name: pack.name,
			version: versionLabel,
			loader: pack.loader,
			engine: pack.engine,
			colorTag: pack.engine === "Horizon (Rust)" ? "cyan" : "emerald",
		})
		spawnMockProcess(t("modpacks.installTitle", { name: pack.name }), "instance", pack.engine)
		void navigate({ to: "/" })
	}

	return (
		<div className="flex flex-col gap-4">
			<div className="flex flex-wrap items-center justify-between gap-3 rounded-2xl border border-border bg-card/65 p-4 backdrop-blur-md">
				<div className="flex min-w-0 items-center gap-3">
					<Link
						to="/modpacks"
						aria-label={t("modpacks.backToModpacksAria")}
						className="flex size-8 shrink-0 items-center justify-center rounded-xl border border-border bg-foreground/5 text-muted-foreground transition-colors hover:bg-foreground/10 hover:text-foreground"
					>
						<ArrowLeft className="size-4" />
					</Link>
					<div className="min-w-0">
						<h1 className="truncate text-base font-semibold text-foreground">{pack.name}</h1>
						<p className="truncate text-[11.5px] text-muted-foreground">
							{pack.provider} · {t("modpacks.byAuthor", { author: pack.author })} ·{" "}
							{t("modpacks.downloadsCount", { downloads: pack.downloads })}
						</p>
					</div>
				</div>

				<button
					type="button"
					onClick={() => {
						setDrawerOpen(true)
					}}
					className="flex h-9 cursor-pointer items-center gap-1.5 rounded-xl bg-primary px-4 text-xs font-semibold text-primary-foreground transition-opacity hover:opacity-90"
				>
					<Download className="size-3.5" />
					<span>{t("modpacks.chooseVersion")}</span>
				</button>
			</div>

			<p className="text-[13px] leading-relaxed text-foreground/85">{t(pack.summaryKey)}</p>

			<ModpackGallery packName={pack.name} accentClass={pack.accentClass} />

			<VersionInstallDrawer
				open={drawerOpen}
				pack={pack}
				onClose={() => {
					setDrawerOpen(false)
				}}
				onInstallVersion={handleInstallVersion}
			/>
		</div>
	)
}

export default ModpackDetailPage
