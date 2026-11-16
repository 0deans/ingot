import { createFileRoute } from "@tanstack/react-router"
import SkinsPage from "@/features/skins/SkinsPage"

export const Route = createFileRoute("/skins")({
	component: SkinsPage,
})
