import { type ContentSearchResult, createTauRPCProxy, type UnifiedContentItem } from "@/bindings"

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
}

export type { ContentSearchResult, UnifiedContentItem }
