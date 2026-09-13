import { createFileRoute } from "@tanstack/react-router"
import { memo } from "react"
import SkinCatalogView from "@/components/skins/skin-catalog-view"

const SkinsPage = () => {
	return <SkinCatalogView />
}

SkinsPage.displayName = "SkinsPage"

const MemoizedSkinsPage = memo(SkinsPage)

export const Route = createFileRoute("/skins")({
	component: MemoizedSkinsPage,
})
