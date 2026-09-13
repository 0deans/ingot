export interface ElySkinItem {
	id: number
	skinUrl: string
	isSlim: boolean
	countWearers: number
	countCubes: number
	countViews: number
	tags: string[]
}

export interface ElySkinsCatalogResponse {
	items: ElySkinItem[]
	totalItems: number
	currentPage: number
	lastPage: number
}

export type SkinSortOption = "wearers" | "views" | "cubes" | "latest"
export type SkinModelFilter = "any" | "steve" | "slim"
