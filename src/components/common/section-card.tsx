import type { LucideIcon } from "lucide-react"
import type { ReactNode } from "react"
import { CardAction, CardDescription, CardHeader, CardTitle } from "@/components/ui/card"

/**
 * Header for a shadcn Card used as a page section: optional icon tile, title, description,
 * and an optional control on the right (a button, a select, a value…).
 *
 *   <Card>
 *     <SectionCardHeader icon={Cpu} title="Memory" description="…" />
 *     <CardContent>…</CardContent>
 *   </Card>
 */
export function SectionCardHeader({
	icon: Icon,
	title,
	description,
	action,
}: {
	icon?: LucideIcon
	title: string
	description?: ReactNode
	action?: ReactNode
}) {
	return (
		<CardHeader>
			<div className="flex min-w-0 items-start gap-2.5">
				{Icon && (
					<div className="mt-0.5 flex size-7 shrink-0 items-center justify-center rounded-lg bg-muted text-muted-foreground">
						<Icon className="size-3.5" />
					</div>
				)}
				<div className="flex min-w-0 flex-col gap-1">
					<CardTitle>{title}</CardTitle>
					{description && <CardDescription>{description}</CardDescription>}
				</div>
			</div>
			{action && <CardAction>{action}</CardAction>}
		</CardHeader>
	)
}
