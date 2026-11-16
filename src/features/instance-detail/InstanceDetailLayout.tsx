import { Outlet, useRouterState } from "@tanstack/react-router"
import { Globe2, Package, Settings, Terminal, type LucideIcon } from "lucide-react"
import { useShallow } from "zustand/react/shallow"
import InstanceHeader from "@/features/instance-detail/components/InstanceHeader"
import RouteNavTabs, { type RouteNavTabItem } from "@/shared/components/motion/RouteNavTabs"
import NotFoundView from "@/shared/components/NotFoundView"
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
	icon: LucideIcon
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

	const tabItems: RouteNavTabItem[] = INSTANCE_TABS.map((tab) => {
		const targetSuffix = `/instances/${instanceId}/${tab.suffix}`
		return {
			key: tab.suffix,
			label: tab.label,
			icon: tab.icon,
			isActive: pathname === targetSuffix,
			link: { to: tab.to, params: { instanceId } },
		}
	})

	return (
		<div className="flex flex-col gap-4">
			<InstanceHeader
				instance={instance}
				isRunning={Boolean(activeProcess)}
				onToggleLaunch={handleToggleLaunch}
			/>

			<RouteNavTabs ariaLabel="Instance Sections" layoutId="instance-detail-tab" items={tabItems} />

			<Outlet />
		</div>
	)
}

export default InstanceDetailLayout
