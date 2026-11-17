import { Download, X } from "lucide-react"
import { AnimatePresence, motion } from "motion/react"
import type { ModpackItem } from "@/features/modpacks/components/ModpackCard"

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
	const versions = [
		{ label: `${pack.version}-latest`, channel: "Release", date: "Today" },
		{ label: `${pack.version}-rc2`, channel: "Beta", date: "4 days ago" },
		{ label: `${pack.version}-lts`, channel: "LTS", date: "2 weeks ago" },
	]

	return (
		<AnimatePresence>
			{open ? (
				<div className="fixed inset-0 z-50 flex items-end justify-center sm:items-center sm:p-4">
					<button
						type="button"
						aria-label="Close version installer"
						onClick={onClose}
						className="fixed inset-0 cursor-default bg-black/60 backdrop-blur-xs"
					/>
					<motion.div
						role="dialog"
						aria-label={`Select version for ${pack.name}`}
						initial={{ opacity: 0, y: 16 }}
						animate={{ opacity: 1, y: 0 }}
						exit={{ opacity: 0, y: 16 }}
						transition={{ duration: 0.18, ease: [0.22, 1, 0.36, 1] }}
						className="ingot-glass-elevated relative z-10 w-full max-w-md rounded-t-2xl p-4 shadow-2xl sm:rounded-2xl"
					>
						<div className="mb-3 flex items-center justify-between gap-3 border-b border-border pb-2.5">
							<div className="min-w-0 flex-1">
								<h2 className="truncate text-sm font-semibold text-foreground">
									Install {pack.name}
								</h2>
								<p className="text-[11px] text-muted-foreground">
									{pack.loader} · {pack.engine}
								</p>
							</div>
							<button
								type="button"
								onClick={onClose}
								aria-label="Dismiss version drawer"
								className="shrink-0 cursor-pointer rounded-lg p-1 text-muted-foreground hover:bg-foreground/8 hover:text-foreground"
							>
								<X className="size-4" />
							</button>
						</div>

						<div className="flex flex-col gap-2">
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
												{ver.channel} · {ver.date}
											</p>
										</div>
										<button
											type="button"
											onClick={() => {
												onInstallVersion(ver.label)
												onClose()
											}}
											className="flex h-7 shrink-0 cursor-pointer items-center gap-1 rounded-lg bg-primary px-2.5 text-[11px] font-semibold whitespace-nowrap text-primary-foreground transition-opacity hover:opacity-90"
										>
											<Download className="size-3" />
											<span>Install</span>
										</button>
									</div>
								)
							})}
						</div>
					</motion.div>
				</div>
			) : null}
		</AnimatePresence>
	)
}

export default VersionInstallDrawer
