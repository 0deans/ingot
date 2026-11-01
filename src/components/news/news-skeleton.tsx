import { memo } from "react"
import { Skeleton } from "@/components/ui/skeleton"

export interface NewsSkeletonGridProps {
	count?: number
}

const NewsSkeletonGrid = ({ count = 6 }: NewsSkeletonGridProps) => {
	const placeholders = Array.from({ length: count }, (_, i) => {
		return i
	})

	return (
		<div className="grid grid-cols-1 gap-5 sm:grid-cols-2 lg:grid-cols-3">
			{placeholders.map((idx) => {
				return (
					<div
						key={idx}
						className="flex flex-col overflow-hidden rounded-2xl border border-border/40 bg-card/40 p-4"
					>
						<Skeleton className="aspect-video w-full rounded-xl" />
						<div className="mt-4 flex flex-col gap-2.5">
							<div className="flex items-center justify-between">
								<Skeleton className="h-5 w-16 rounded-full" />
								<Skeleton className="h-4 w-20" />
							</div>
							<Skeleton className="h-5 w-3/4" />
							<Skeleton className="h-3 w-full" />
							<Skeleton className="h-3 w-4/5" />
						</div>
						<div className="mt-5 flex items-center justify-between border-t border-border/30 pt-3">
							<Skeleton className="h-8 w-24 rounded-lg" />
							<Skeleton className="h-8 w-28 rounded-lg" />
						</div>
					</div>
				)
			})}
		</div>
	)
}

NewsSkeletonGrid.displayName = "NewsSkeletonGrid"

export default memo(NewsSkeletonGrid)
