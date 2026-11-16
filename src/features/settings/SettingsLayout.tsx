import { Link, Outlet, useRouterState } from "@tanstack/react-router"
import { motion } from "motion/react"
import { cn } from "@/shared/lib/utils"

interface SettingsTabLink {
	to: "/settings" | "/settings/java" | "/settings/accounts" | "/settings/network"
	label: string
}

const SETTINGS_TABS: readonly SettingsTabLink[] = [
	{ to: "/settings", label: "Appearance" },
	{ to: "/settings/java", label: "Java & Memory" },
	{ to: "/settings/accounts", label: "Accounts" },
	{ to: "/settings/network", label: "Network" },
]

const SettingsLayout = () => {
	const pathname = useRouterState({
		select: (state) => {
			return state.location.pathname
		},
	})

	return (
		<div className="flex flex-col gap-5">
			<div className="flex flex-wrap items-center justify-between gap-3">
				<div>
					<h1 className="text-lg font-semibold tracking-tight text-foreground">Settings</h1>
					<p className="text-[12px] text-muted-foreground">
						Customize theme colors, desktop chrome layout, JVM memory, network, and profiles.
					</p>
				</div>

				<nav
					aria-label="Settings Sections"
					className="no-scrollbar inline-flex items-center gap-0.5 overflow-x-auto rounded-xl border border-border bg-sidebar/80 p-1"
				>
					{SETTINGS_TABS.map((item) => {
						const isActive =
							item.to === "/settings"
								? pathname === "/settings" || pathname === "/settings/"
								: pathname === item.to
						return (
							<Link
								key={item.to}
								to={item.to}
								className={cn(
									"relative flex h-7 items-center rounded-lg px-2.5 text-xs font-medium whitespace-nowrap",
									isActive ? "text-primary" : "text-muted-foreground hover:text-foreground",
								)}
							>
								{isActive ? (
									<motion.span
										layoutId="settings-subtab-pill"
										transition={{ duration: 0.24, ease: [0.22, 1, 0.36, 1] }}
										className="pointer-events-none absolute inset-0 rounded-lg border border-primary/30 bg-primary/15 shadow-xs"
									/>
								) : null}
								<span className="relative z-10">{item.label}</span>
							</Link>
						)
					})}
				</nav>
			</div>

			<Outlet />
		</div>
	)
}

export default SettingsLayout
