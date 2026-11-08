import { createFileRoute } from "@tanstack/react-router"
import { memo, useCallback } from "react"
import * as v from "valibot"
import ExportInstanceView from "@/components/instances/export-instance-view"
import { instanceService, useRunningInstances } from "@/services/instance-service"

export const instanceExportSearchSchema = v.object({
	from: v.optional(v.picklist(["home", "manage"])),
})

export type InstanceExportSearchParams = v.InferOutput<typeof instanceExportSearchSchema>

const InstanceExportPage = () => {
	// 1. Context & Global
	const { id } = Route.useParams()
	const { from } = Route.useSearch()
	const navigate = Route.useNavigate()
	const { runningMap } = useRunningInstances()

	// 2. Refs (none)

	// 3. State & Loader data
	const instances = Route.useLoaderData()
	const instance = instances.find((i) => {
		return i.id === id
	})
	const isRunning = Boolean(instance && runningMap.has(instance.id))

	// 4. Memoization & Callbacks
	const handleBack = useCallback(() => {
		if (from === "manage") {
			void navigate({
				to: "/instances/$id",
				params: { id },
			})
		} else {
			void navigate({
				to: "/",
				search: (prev) => {
					return {
						...prev,
						logs: undefined,
						action: undefined,
						edit: undefined,
						delete: undefined,
						duplicate: undefined,
					}
				},
			})
		}
	}, [navigate, id, from])

	const handleBackToHome = useCallback(() => {
		void navigate({
			to: "/",
			search: (prev) => {
				return {
					...prev,
					logs: undefined,
					action: undefined,
					edit: undefined,
					delete: undefined,
					duplicate: undefined,
				}
			},
		})
	}, [navigate])

	if (!instance) {
		return (
			<div className="flex h-full flex-1 flex-col items-center justify-center gap-4 p-8">
				<p className="text-sm text-muted-foreground">Instance not found</p>
				<button type="button" onClick={handleBackToHome} className="text-xs text-primary underline">
					Back to instances
				</button>
			</div>
		)
	}

	return (
		<ExportInstanceView instance={instance} isInstanceRunning={isRunning} onBack={handleBack} />
	)
}

InstanceExportPage.displayName = "InstanceExportPage"

const MemoizedInstanceExportPage = memo(InstanceExportPage)

export const Route = createFileRoute("/instances/$id/export")({
	validateSearch: instanceExportSearchSchema,
	loader: async () => {
		return instanceService.getInstances()
	},
	component: MemoizedInstanceExportPage,
})
