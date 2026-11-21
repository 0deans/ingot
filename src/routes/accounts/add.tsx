import { createFileRoute } from "@tanstack/react-router"
import AddAccountPage from "@/features/accounts/AddAccountPage"

export const Route = createFileRoute("/accounts/add")({
	component: AddAccountPage,
})
