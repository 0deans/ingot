import i18n from "i18next"
import { toast } from "sonner"
import type {
	ContentScreenshot,
	ContentSearchResult,
	InstanceConfig,
	ModLoaderType,
	UnifiedContentDetails,
	UnifiedContentItem,
	UnifiedContentVersion,
} from "@/bindings"
import { rpc, unwrapRpc } from "@/lib/rpc"
import { instanceService } from "@/services/instance-service"

export type ContentSource = "all" | "modrinth" | "curseforge" | "ftb" | "technic" | "atlauncher"
export type ContentType = "all" | "modpack" | "mod" | "resourcepack" | "shader"
export type ContentSort = "downloads" | "relevance" | "updated" | "newest"

export interface SearchContentParams {
	source?: ContentSource
	projectType?: ContentType
	query?: string | null
	gameVersion?: string | null
	loader?: string | null
	sort?: ContentSort | null
	page?: number
	pageSize?: number
}

export const stripContentSourcePrefix = (projectId: string): string => {
	return projectId.replace(/^(mr|cf|ftb|technic|atl|modrinth|curseforge|atlauncher):/, "")
}

export const isModpackOnlySource = (source: ContentSource): boolean => {
	return source === "ftb" || source === "technic" || source === "atlauncher"
}

export const contentService = {
	async searchContent(params: SearchContentParams = {}): Promise<ContentSearchResult> {
		try {
			return await unwrapRpc(
				rpc.content.search_content(
					params.source ?? "all",
					params.projectType ?? "all",
					params.query?.trim() ? params.query.trim() : null,
					params.gameVersion?.trim() ? params.gameVersion.trim() : null,
					params.loader?.trim() ? params.loader.trim() : null,
					params.sort ?? "downloads",
					params.page ?? 0,
					params.pageSize ?? 24,
				),
			)
		} catch (error) {
			console.error("Failed to search content:", error)
			return {
				items: [],
				totalHits: 0,
				offset: 0,
				limit: params.pageSize ?? 24,
			}
		}
	},

	async getContentDetails(
		source: ContentSource | string,
		projectId: string,
	): Promise<UnifiedContentDetails> {
		const cleanId = stripContentSourcePrefix(projectId)
		try {
			return await unwrapRpc(rpc.content.get_content_details(source, cleanId))
		} catch (error) {
			console.error(`Failed to get content details for ${projectId}:`, error)
			throw error
		}
	},

	async installContentFile(
		instanceId: string,
		projectType: string,
		downloadUrl: string,
		filename: string,
	): Promise<string> {
		try {
			return await unwrapRpc(
				rpc.content.install_content_file(instanceId, projectType, downloadUrl, filename),
			)
		} catch (error) {
			console.error(`Failed to install content file ${filename}:`, error)
			throw error
		}
	},

	async installModpackInstance(
		name: string,
		source: ContentSource | string,
		downloadUrl: string,
		filename: string,
	): Promise<InstanceConfig> {
		try {
			return await unwrapRpc(
				rpc.content.install_modpack_instance(name, source, downloadUrl, filename),
			)
		} catch (error) {
			console.error(`Failed to install modpack ${name}:`, error)
			throw error
		}
	},

	startBackgroundModpackInstall(params: {
		name: string
		source: ContentSource | string
		downloadUrl: string
		filename: string
		gameVersion?: string
		loader?: ModLoaderType
		iconUrl?: string | null
	}): void {
		const trimmedName = params.name.trim()
		const trackingId = `installing:${trimmedName}`
		if (instanceService.isPendingInstall(trackingId)) {
			return
		}

		instanceService.registerPendingInstall({
			name: trimmedName,
			gameVersion: params.gameVersion,
			loader: params.loader,
			iconUrl: params.iconUrl,
		})

		toast.info(
			i18n.t("install.modpackStarted", {
				name: trimmedName,
				defaultValue: `Installing "${trimmedName}" in the background...`,
			}),
		)

		void (async () => {
			try {
				await this.installModpackInstance(
					trimmedName,
					params.source,
					params.downloadUrl,
					params.filename,
				)
				instanceService.finishPendingInstall(trackingId)
				await instanceService.refreshInstances()
				toast.success(
					i18n.t("install.modpackDone", {
						name: trimmedName,
					}),
				)
			} catch (err) {
				instanceService.finishPendingInstall(trackingId)
				const message = formatErrorMessage(err)
				toast.error(message)
			}
		})()
	},
}

const formatErrorMessage = (err: unknown): string => {
	if (!err) {
		return i18n.t("install.loadFailed")
	}
	if (typeof err === "string") {
		return err
	}
	if (err instanceof Error) {
		return err.message
	}
	if (
		typeof err === "object" &&
		err !== null &&
		"message" in err &&
		typeof err.message === "string"
	) {
		return err.message
	}
	return String(err)
}

export type {
	ContentScreenshot,
	ContentSearchResult,
	UnifiedContentDetails,
	UnifiedContentItem,
	UnifiedContentVersion,
}
