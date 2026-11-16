import { createFileRoute } from "@tanstack/react-router"
import AccountsSettingsPage from "@/features/settings/AccountsSettingsPage"

export const Route = createFileRoute("/settings/accounts")({
	component: AccountsSettingsPage,
})
