import { AlertTriangle, CheckCircle2, Globe, Package } from "lucide-react"
import { memo } from "react"
import { useTranslation } from "react-i18next"
import type { InstalledContent } from "@/bindings"
import { Badge } from "@/components/ui/badge"
import { ScrollArea } from "@/components/ui/scroll-area"
import { Separator } from "@/components/ui/separator"
import {
	Sheet,
	SheetContent,
	SheetDescription,
	SheetHeader,
	SheetTitle,
} from "@/components/ui/sheet"
import { formatBytes, formatDate } from "@/lib/format"

export interface ContentDetailsSheetProps {
	item: InstalledContent | null
	isOpen: boolean
	onClose: () => void
}

const ContentDetailsSheet = ({ item, isOpen, onClose }: ContentDetailsSheetProps) => {
	const { t } = useTranslation()

	if (!item) {
		return null
	}

	const isEnabled = item.state === "enabled"
	const iconSrc = item.metadata.iconDataUrl
	const hasDependencies = item.metadata.dependencies.length > 0
	const authors = item.metadata.authors.join(", ")

	return (
		<Sheet open={isOpen} onOpenChange={onClose}>
			<SheetContent className="flex w-full flex-col sm:max-w-md">
				<SheetHeader className="gap-2">
					<div className="flex items-center gap-3">
						<div className="flex size-12 shrink-0 items-center justify-center overflow-hidden rounded-lg border border-border/60 bg-muted/50">
							{iconSrc ? (
								<img src={iconSrc} alt={item.metadata.name} className="size-full object-contain" />
							) : (
								<Package className="size-6 text-muted-foreground" />
							)}
						</div>
						<div className="min-w-0 flex-1">
							<SheetTitle className="truncate text-base font-semibold">
								{item.metadata.name}
							</SheetTitle>
							<SheetDescription className="flex items-center gap-2 text-xs">
								<Badge variant={isEnabled ? "default" : "secondary"}>
									{isEnabled ? t("instanceContent.enabled") : t("instanceContent.disabled")}
								</Badge>
								{item.metadata.version && <Badge variant="outline">{item.metadata.version}</Badge>}
							</SheetDescription>
						</div>
					</div>
				</SheetHeader>

				<Separator />

				<ScrollArea className="flex-1 pr-2">
					<div className="flex flex-col gap-4 text-xs">
						{/* Description */}
						{item.metadata.description && (
							<div className="flex flex-col gap-1">
								<span className="font-semibold text-muted-foreground">
									{t("instanceContent.description")}
								</span>
								<p className="leading-relaxed whitespace-pre-wrap text-foreground">
									{item.metadata.description}
								</p>
							</div>
						)}

						{/* File Details */}
						<div className="flex flex-col gap-2 rounded-md border border-border/50 bg-muted/20 p-3">
							<div className="flex justify-between">
								<span className="text-muted-foreground">{t("instanceContent.fileName")}</span>
								<span className="font-mono text-foreground">{item.fileName}</span>
							</div>
							{item.metadata.modId && (
								<div className="flex justify-between">
									<span className="text-muted-foreground">{t("instanceContent.modId")}</span>
									<span className="font-mono text-foreground">{item.metadata.modId}</span>
								</div>
							)}
							<div className="flex justify-between">
								<span className="text-muted-foreground">{t("instanceContent.fileSize")}</span>
								<span className="text-foreground">{formatBytes(item.sizeBytes)}</span>
							</div>
							{item.modifiedAt > 0 && (
								<div className="flex justify-between">
									<span className="text-muted-foreground">{t("instanceContent.lastModified")}</span>
									<span className="text-foreground">{formatDate(item.modifiedAt * 1000)}</span>
								</div>
							)}
							{authors && (
								<div className="flex justify-between">
									<span className="text-muted-foreground">{t("instanceContent.authors")}</span>
									<span className="text-foreground">{authors}</span>
								</div>
							)}
						</div>

						{/* Origin Details */}
						{item.origin.kind !== "unknown" && (
							<div className="flex flex-col gap-1.5">
								<span className="font-semibold text-muted-foreground">
									{t("instanceContent.origin")}
								</span>
								<div className="flex items-center gap-2 rounded-md border border-border/50 p-2.5">
									<Globe className="size-4 text-muted-foreground" />
									<div className="flex-1">
										<p className="font-medium text-foreground">{item.origin.title}</p>
										<p className="text-[11px] text-muted-foreground">{item.origin.source}</p>
									</div>
								</div>
							</div>
						)}

						{/* Dependencies */}
						{hasDependencies && (
							<div className="flex flex-col gap-2">
								<span className="font-semibold text-muted-foreground">
									{t("instanceContent.dependencies")}
								</span>
								<div className="flex flex-col gap-1.5">
									{item.metadata.dependencies.map((dep) => {
										return (
											<div
												key={dep.id}
												className="flex items-center justify-between rounded-md border border-border/40 p-2"
											>
												<div className="flex items-center gap-2">
													{dep.required ? (
														<CheckCircle2 className="size-3.5 text-muted-foreground" />
													) : (
														<AlertTriangle className="size-3.5 text-muted-foreground" />
													)}
													<span className="font-mono text-foreground">{dep.id}</span>
												</div>
												<Badge variant="outline" className="text-[10px]">
													{dep.required
														? t("instanceContent.required")
														: t("instanceContent.optional")}
												</Badge>
											</div>
										)
									})}
								</div>
							</div>
						)}
					</div>
				</ScrollArea>
			</SheetContent>
		</Sheet>
	)
}

ContentDetailsSheet.displayName = "ContentDetailsSheet"

export default memo(ContentDetailsSheet)
