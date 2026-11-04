import { createFileRoute } from "@tanstack/react-router"
import { memo, useCallback } from "react"
import * as v from "valibot"
import InstanceContentView from "@/components/instance-content/instance-content-view"
import { instanceService, useRunningInstances } from "@/services/instance-service"

export const instanceContentTabSchema = v.picklist([
	"mods",
	"resourcepacks",
	"shaders",
	"datapacks",
])

export const instanceContentSearchSchema = v.object({
	tab: v.optional(v.fallback(instanceContentTabSchema, "mods"), "mods"),
	world: v.optional(v.string()),
})

export type InstanceContentSearchParams = v.InferOutput<typeof instanceContentSearchSchema>

const InstanceManagePage = () => {
	// 1. Context & Global
	const { id } = Route.useParams()
	const { tab } = Route.useSearch()
	const navigate = Route.useNavigate()
	const { runningMap } = useRunningInstances()

	// 2. Refs (none)

	// 3. State & Search
	const instances = Route.useLoaderData()
	const instance = instances.find((i) => i.id === id)
	const isRunning = Boolean(instance && runningMap.has(instance.id))

	// 4. Callbacks
	const handleTabChange = useCallback(
		(nextTab: "mods" | "resourcepacks" | "shaders" | "datapacks") => {
			void navigate({
				search: (prev) => {
					return {
						...prev,
						tab: nextTab,
					}
				},
				replace: true,
			})
		},
		[navigate],
	)

	const handleBack = useCallback(() => {
		void navigate({
			to: "/",
		})
	}, [navigate])

	const handleOpenDiscoverMods = useCallback(() => {
		void navigate({
			to: "/modpacks",
		})
	}, [navigate])

	if (!instance) {
		return (
			<div className="flex h-full flex-1 flex-col items-center justify-center gap-4 p-8">
				<p className="text-sm text-muted-foreground">Instance not found</p>
				<button type="button" onClick={handleBack} className="text-xs text-primary underline">
					Back to instances
				</button>
			</div>
		)
	}

	return (
		<InstanceContentView
			instance={instance}
			activeTab={tab}
			isInstanceRunning={isRunning}
			onTabChange={handleTabChange}
			onBack={handleBack}
			onOpenDiscoverMods={handleOpenDiscoverMods}
		/>
	)
}

InstanceManagePage.displayName = "InstanceManagePage"

const MemoizedInstanceManagePage = memo(InstanceManagePage)

export const Route = createFileRoute("/instances/$id")({
	validateSearch: instanceContentSearchSchema,
	loader: async () => {
		return instanceService.getInstances()
	},
	component: MemoizedInstanceManagePage,
})
