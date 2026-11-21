import { queryOptions, useMutation, useQuery, useQueryClient } from "@tanstack/react-query"
import { STATIC_STALE_TIME_MS } from "@/shared/api/queryClient"
import { rpc } from "@/shared/api/rpc"

export const screenshotKeys = {
	all: ["screenshots"] as const,
	list: () => {
		return [...screenshotKeys.all, "list"] as const
	},
	thumbnail: (instanceId: string, fileName: string, modifiedAt: number) => {
		return [...screenshotKeys.all, "thumbnail", instanceId, fileName, modifiedAt] as const
	},
}

export const allScreenshotsQueryOptions = () => {
	return queryOptions({
		queryKey: screenshotKeys.list(),
		queryFn: () => {
			return rpc.instances.screenshots.get_all_screenshots()
		},
	})
}

export const screenshotThumbnailQueryOptions = (input: {
	instanceId: string
	fileName: string
	filePath: string
	modifiedAt: number
}) => {
	return queryOptions({
		queryKey: screenshotKeys.thumbnail(input.instanceId, input.fileName, input.modifiedAt),
		queryFn: () => {
			return rpc.instances.screenshots.get_screenshot_thumbnail(
				input.instanceId,
				input.fileName,
				input.filePath,
				input.modifiedAt,
			)
		},
		enabled: input.filePath.length > 0,
		staleTime: STATIC_STALE_TIME_MS,
	})
}

export const useAllScreenshots = () => {
	return useQuery(allScreenshotsQueryOptions())
}

export const useScreenshotThumbnail = (input: {
	instanceId: string
	fileName: string
	filePath: string
	modifiedAt: number
}) => {
	return useQuery(screenshotThumbnailQueryOptions(input))
}

export const useDeleteScreenshot = () => {
	const queryClient = useQueryClient()

	return useMutation({
		mutationFn: (input: { instanceId: string; fileName: string }) => {
			return rpc.instances.screenshots.delete_screenshot(input.instanceId, input.fileName)
		},
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: screenshotKeys.list() })
		},
	})
}

export const useOpenScreenshotsFolder = () => {
	return useMutation({
		mutationFn: (instanceId: string | null) => {
			return rpc.instances.screenshots.open_screenshots_folder(instanceId)
		},
	})
}

export const useRevealScreenshotFile = () => {
	return useMutation({
		mutationFn: (filePath: string) => {
			return rpc.instances.screenshots.reveal_screenshot_file(filePath)
		},
	})
}
