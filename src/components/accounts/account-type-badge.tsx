import { useTranslation } from "react-i18next"
import { Badge } from "@/components/ui/badge"
import { cn } from "@/lib/utils"
import type { AccountType } from "@/types/account"

/** Ely.by / Microsoft / Offline tag next to an account name */
export function AccountTypeBadge({ type, className }: { type: string; className?: string }) {
	const { t } = useTranslation()

	switch (type as AccountType) {
		case "ely":
			return (
				<Badge variant="outline" className={cn("border-primary/30 text-primary", className)}>
					Ely.by
				</Badge>
			)
		case "microsoft":
			return (
				<Badge variant="outline" className={cn("border-info/30 text-info", className)}>
					Microsoft
				</Badge>
			)
		case "offline":
			return (
				<Badge variant="secondary" className={className}>
					{t("accounts.offline")}
				</Badge>
			)
		default:
			return null
	}
}
