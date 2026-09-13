import {
	type ContentScreenshot,
	type ContentSearchResult,
	createTauRPCProxy,
	type InstanceConfig,
	type UnifiedContentDetails,
	type UnifiedContentItem,
	type UnifiedContentVersion,
} from "@/bindings"

const rpc = createTauRPCProxy()

export type ContentSource = "all" | "modrinth" | "curseforge"
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

export const contentService = {
	async searchContent(params: SearchContentParams = {}): Promise<ContentSearchResult> {
		try {
			return await rpc.search_content(
				params.source ?? "all",
				params.projectType ?? "all",
				params.query?.trim() ? params.query.trim() : null,
				params.gameVersion?.trim() ? params.gameVersion.trim() : null,
				params.loader?.trim() ? params.loader.trim() : null,
				params.sort ?? "downloads",
				params.page ?? 0,
				params.pageSize ?? 24,
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
		source: ContentSource,
		projectId: string,
	): Promise<UnifiedContentDetails> {
		const cleanId = projectId.replace(/^(mr|cf|modrinth|curseforge):/, "")
		try {
			return await rpc.get_content_details(source, cleanId)
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
			return await rpc.install_content_file(instanceId, projectType, downloadUrl, filename)
		} catch (error) {
			console.error(`Failed to install content file ${filename}:`, error)
			throw error
		}
	},

	async installModpackInstance(
		name: string,
		source: ContentSource,
		downloadUrl: string,
		filename: string,
	): Promise<InstanceConfig> {
		try {
			return await rpc.install_modpack_instance(name, source, downloadUrl, filename)
		} catch (error) {
			console.error(`Failed to install modpack ${name}:`, error)
			throw error
		}
	},
}

export type {
	ContentScreenshot,
	ContentSearchResult,
	UnifiedContentDetails,
	UnifiedContentItem,
	UnifiedContentVersion,
}
