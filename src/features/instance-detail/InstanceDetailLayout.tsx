import { Link, Outlet, useRouterState } from "@tanstack/react-router"
import { Globe2, Package, Settings, Terminal } from "lucide-react"
import { motion } from "motion/react"
import { useShallow } from "zustand/react/shallow"
import InstanceHeader from "@/features/instance-detail/components/InstanceHeader"
import NotFoundView from "@/shared/components/NotFoundView"
import { cn } from "@/shared/lib/utils"
import { selectActiveAccount, useAccountStore } from "@/shared/stores/useAccountStore"
import { selectInstanceById, useInstancesStore } from "@/shared/stores/useInstancesStore"
import { useLauncherPrefsStore } from "@/shared/stores/useLauncherPrefsStore"
import { findProcessByName, useProcessStore } from "@/shared/stores/useProcessStore"

interface InstanceDetailLayoutProps {
	instanceId: string
}

type InstanceTabSuffix = "mods" | "worlds" | "logs" | "settings"

interface InstanceTabItem {
	to:
		| "/instances/$instanceId/mods"
		| "/instances/$instanceId/worlds"
		| "/instances/$instanceId/logs"
		| "/instances/$instanceId/settings"
	suffix: InstanceTabSuffix
	label: string
	icon: typeof Package
}

const INSTANCE_TABS: readonly InstanceTabItem[] = [
	{ to: "/instances/$instanceId/mods", suffix: "mods", label: "Mods", icon: Package },
	{ to: "/instances/$instanceId/worlds", suffix: "worlds", label: "Worlds", icon: Globe2 },
	{ to: "/instances/$instanceId/logs", suffix: "logs", label: "Live Logs", icon: Terminal },
	{
		to: "/instances/$instanceId/settings",
		suffix: "settings",
		label: "JVM & Settings",
		icon: Settings,
	},
]

const InstanceDetailLayout = ({ instanceId }: InstanceDetailLayoutProps) => {
	const pathname = useRouterState({
		select: (state) => {
			return state.location.pathname
		},
	})
	const instance = useInstancesStore(selectInstanceById(instanceId))
	const { runningProcesses, launchInstance, stopProcess } = useProcessStore(
		useShallow((s) => {
			return {
				runningProcesses: s.runningProcesses,
				launchInstance: s.launchInstance,
				stopProcess: s.stopProcess,
			}
		}),
	)
	const activeAccount = useAccountStore(selectActiveAccount)
	const ramGb = useLauncherPrefsStore((s) => {
		return s.ramGb
	})

	if (!instance) {
		return <NotFoundView />
	}

	const activeProcess = findProcessByName(runningProcesses, instance.name)

	const handleToggleLaunch = () => {
		if (activeProcess) {
			stopProcess(activeProcess.id)
			return
		}
		launchInstance({
			instance,
			username: activeAccount.username,
			ramGb,
		})
	}

	return (
		<div className="flex flex-col gap-4">
			<InstanceHeader
				instance={instance}
				isRunning={Boolean(activeProcess)}
				onToggleLaunch={handleToggleLaunch}
			/>

			<nav
				aria-label="Instance Sections"
				className="inline-flex w-fit items-center gap-1 rounded-xl border border-border bg-sidebar/80 p-1"
			>
				{INSTANCE_TABS.map((tab) => {
					const Icon = tab.icon
					const targetSuffix = `/instances/${instanceId}/${tab.suffix}`
					const isActive = pathname === targetSuffix
					return (
						<Link
							key={tab.suffix}
							to={tab.to}
							params={{ instanceId }}
							className={cn(
								"relative flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-medium",
								isActive ? "text-primary" : "text-muted-foreground hover:text-foreground",
							)}
						>
							{isActive ? (
								<motion.span
									layoutId="instance-detail-tab"
									transition={{ duration: 0.24, ease: [0.22, 1, 0.36, 1] }}
									className="absolute inset-0 rounded-lg border border-primary/30 bg-primary/15"
								/>
							) : null}
							<Icon className="relative z-10 size-3.5" />
							<span className="relative z-10">{tab.label}</span>
						</Link>
					)
				})}
			</nav>

			<Outlet />
		</div>
	)
}

export default InstanceDetailLayout
