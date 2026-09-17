import { createFileRoute } from "@tanstack/react-router"
import { memo } from "react"
import * as v from "valibot"
import SkinCatalogView, { skinsQueryOptions } from "@/components/skins/skin-catalog-view"
import { accountService } from "@/services/account-service"

export const skinTabSchema = v.picklist(["catalog", "my-skins", "upload"])
export const skinSortSchema = v.picklist(["wearers", "views", "cubes", "latest"])
export const skinModelSchema = v.picklist(["any", "steve", "slim"])

export const skinsSearchSchema = v.object({
	q: v.optional(v.fallback(v.string(), ""), ""),
	tab: v.optional(v.fallback(skinTabSchema, "catalog"), "catalog"),
	sort: v.optional(v.fallback(skinSortSchema, "wearers"), "wearers"),
	model: v.optional(v.fallback(skinModelSchema, "any"), "any"),
	page: v.optional(v.fallback(v.number(), 1), 1),
	skinId: v.optional(v.number()),
})

export type SkinsSearchParams = v.InferOutput<typeof skinsSearchSchema>

const SkinsPage = () => {
	return <SkinCatalogView />
}

SkinsPage.displayName = "SkinsPage"

const MemoizedSkinsPage = memo(SkinsPage)

export const Route = createFileRoute("/skins")({
	validateSearch: skinsSearchSchema,
	loaderDeps: ({ search }) => ({ search }),
	loader: async ({ context: { queryClient }, deps: { search } }) => {
		const activeAcc = accountService.getCachedAccounts().find((a) => a.isActive)
		return queryClient.ensureQueryData(
			skinsQueryOptions({
				tab: search.tab ?? "catalog",
				page: search.page ?? 1,
				searchQuery: search.q ?? "",
				sort: search.sort ?? "wearers",
				model: search.model ?? "any",
				accountId: activeAcc?.id,
				accountUsername: activeAcc?.username,
				accountSkinUrl: activeAcc?.skinUrl,
			}),
		)
	},
	component: MemoizedSkinsPage,
})
