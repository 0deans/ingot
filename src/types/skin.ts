export interface UserUploadedSkin {
	id: string
	name: string
	dataUrl: string
	isSlim: boolean
	uploadedAt: number
}

export interface ElySkinItem {
	id: number
	skinUrl: string
	isSlim: boolean
	countWearers: number
	countCubes: number
	countViews: number
	tags: string[]
	// Optional fields for custom/user uploaded skins
	dataUrl?: string
	isCustom?: boolean
	name?: string
	uploadedAt?: number
	provider?: SkinCatalogProvider
	capeUrl?: string | null
}

export interface ElySkinsCatalogResponse {
	items: ElySkinItem[]
	totalItems: number
	currentPage: number
	lastPage: number
}

export type SkinCatalogProvider = "ely" | "player"
export type SkinSortOption = "wearers" | "views" | "cubes" | "latest"
export type SkinModelFilter = "any" | "steve" | "slim"
