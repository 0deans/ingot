import { open as openDialog } from "@tauri-apps/plugin-dialog"
import {
	Check,
	ChevronDown,
	FileArchive,
	Folder,
	FolderClosed,
	FolderOpen,
	Upload,
} from "lucide-react"
import { useCallback, useMemo, useState } from "react"
import { useTranslation } from "react-i18next"
import { toast } from "sonner"
import type { InstanceConfig } from "@/bindings"
import InstanceIcon, { getColorTagDefinition } from "@/components/instances/instance-icon"
import { Button } from "@/components/ui/button"
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog"
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuGroup,
	DropdownMenuItem,
	DropdownMenuLabel,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import { InputGroup, InputGroupAddon, InputGroupInput } from "@/components/ui/input-group"
import { Label } from "@/components/ui/label"
import { Spinner } from "@/components/ui/spinner"
import { useCachedInstances } from "@/services/instance-service"
import { worldService } from "@/services/world-service"

interface ImportWorldDialogProps {
	open: boolean
	onOpenChange: (open: boolean) => void
	initialInstanceId?: string
	onSuccess: () => void
}

const ImportWorldDialog = ({
	open,
	onOpenChange,
	initialInstanceId,
	onSuccess,
}: ImportWorldDialogProps) => {
	const { t } = useTranslation()
	const instances = useCachedInstances()

	const defaultInstanceId =
		initialInstanceId && initialInstanceId !== "all" ? initialInstanceId : (instances[0]?.id ?? "")

	const [prevOpen, setPrevOpen] = useState(open)
	const [selectedInstanceId, setSelectedInstanceId] = useState<string>(defaultInstanceId)
	const [zipPath, setZipPath] = useState<string>("")
	const [isImporting, setIsImporting] = useState(false)

	if (open && !prevOpen) {
		setPrevOpen(true)
		setSelectedInstanceId(defaultInstanceId)
		setZipPath("")
	} else if (!open && prevOpen) {
		setPrevOpen(false)
	}

	const instanceMap = useMemo(() => {
		return new Map(instances.map((i) => [i.id, i]))
	}, [instances])

	const selectedInstance = useMemo(() => {
		return instanceMap.get(selectedInstanceId)
	}, [instanceMap, selectedInstanceId])

	const selectedColorDef = useMemo(() => {
		return getColorTagDefinition(selectedInstance?.colorTag)
	}, [selectedInstance?.colorTag])

	const instanceSections = useMemo(() => {
		const namedGroups = new Map<string, InstanceConfig[]>()
		const ungrouped: InstanceConfig[] = []

		for (const inst of instances) {
			const groupName = inst.group?.trim()
			if (groupName) {
				const list = namedGroups.get(groupName) ?? []
				list.push(inst)
				namedGroups.set(groupName, list)
			} else {
				ungrouped.push(inst)
			}
		}

		const sortedGroupNames = Array.from(namedGroups.keys()).sort((a, b) => {
			return a.localeCompare(b)
		})

		const sections: {
			key: string
			title: string | null
			isUngrouped: boolean
			items: InstanceConfig[]
		}[] = []

		if (sortedGroupNames.length === 0) {
			return [
				{
					key: "all",
					title: null,
					isUngrouped: true,
					items: instances,
				},
			]
		}

		for (const groupName of sortedGroupNames) {
			sections.push({
				key: `group:${groupName}`,
				title: groupName,
				isUngrouped: false,
				items: namedGroups.get(groupName) ?? [],
			})
		}

		if (ungrouped.length > 0) {
			sections.push({
				key: "__ungrouped__",
				title: t("instances.groups.ungrouped", { defaultValue: "Ungrouped" }),
				isUngrouped: true,
				items: ungrouped,
			})
		}

		return sections
	}, [instances, t])

	const handleSelectFile = useCallback(async () => {
		try {
			const selected = await openDialog({
				title: t("worlds.selectZip"),
				multiple: false,
				directory: false,
				filters: [{ name: "Minecraft World (.zip)", extensions: ["zip"] }],
			})
			if (selected && typeof selected === "string") {
				setZipPath(selected)
			}
		} catch (error) {
			console.error("Failed to open file picker:", error)
		}
	}, [t])

	const handleImport = useCallback(async () => {
		if (!selectedInstanceId || !zipPath) {
			return
		}
		try {
			setIsImporting(true)
			const imported = await worldService.importWorld(selectedInstanceId, zipPath)
			toast.success(t("worlds.importSuccess", { name: imported.displayName }))
			onOpenChange(false)
			onSuccess()
		} catch (error) {
			console.error("Failed to import world:", error)
			const message = error instanceof Error ? error.message : String(error)
			toast.error(message)
		} finally {
			setIsImporting(false)
		}
	}, [selectedInstanceId, zipPath, onOpenChange, onSuccess, t])

	const fileName = zipPath ? (zipPath.split(/[/\\]/).pop() ?? zipPath) : ""

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent className="max-w-md">
				<DialogHeader>
					<DialogTitle className="flex items-center gap-2">
						<Upload className="size-5 shrink-0 text-primary" />
						{t("worlds.importWorld")}
					</DialogTitle>
					<DialogDescription>{t("worlds.importWorldDesc")}</DialogDescription>
				</DialogHeader>

				<div className="space-y-4 py-2">
					<div className="space-y-1.5">
						<Label className="text-xs font-medium text-foreground">
							{t("worlds.selectInstance")}
						</Label>
						<DropdownMenu>
							<DropdownMenuTrigger
								render={
									<Button
										type="button"
										variant="outline"
										className="h-10 w-full justify-between gap-2.5 px-3 text-left font-normal"
									>
										{selectedInstance ? (
											<div className="flex min-w-0 flex-1 items-center gap-2.5">
												<InstanceIcon
													icon={selectedInstance.icon}
													loader={selectedInstance.loader}
													size={18}
													className="size-4.5 shrink-0 rounded"
												/>
												<div className="flex min-w-0 flex-1 items-center gap-2">
													{selectedColorDef && (
														<span
															className="size-2 shrink-0 rounded-full"
															style={{ backgroundColor: selectedColorDef.hex }}
															title={selectedColorDef.defaultLabel}
														/>
													)}
													<span className="truncate text-xs font-medium text-foreground">
														{selectedInstance.name}
													</span>
													<span className="rounded bg-muted px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground uppercase">
														{selectedInstance.loader} {selectedInstance.gameVersion}
													</span>
													{selectedInstance.group && (
														<span className="flex items-center gap-1 rounded border border-border/40 px-1.5 py-0.5 text-2xs text-muted-foreground">
															<Folder className="size-2.5" />
															{selectedInstance.group}
														</span>
													)}
												</div>
											</div>
										) : (
											<span className="text-xs text-muted-foreground">
												{t("worlds.selectInstance")}
											</span>
										)}
										<ChevronDown className="size-4 shrink-0 opacity-60" />
									</Button>
								}
							/>
							<DropdownMenuContent
								align="start"
								className="max-h-72 w-(--anchor-width) min-w-[320px] overflow-y-auto"
							>
								{instanceSections.map((section, idx) => {
									return (
										<DropdownMenuGroup key={section.key}>
											{section.title && (
												<>
													{idx > 0 && <DropdownMenuSeparator />}
													<DropdownMenuLabel className="flex items-center gap-1.5 px-2 py-1 text-3xs font-semibold tracking-wider text-muted-foreground/70 uppercase">
														{section.isUngrouped ? (
															<FolderClosed className="size-3" />
														) : (
															<Folder className="size-3" />
														)}
														<span>{section.title}</span>
														<span className="ml-auto text-3xs font-normal text-muted-foreground/50 lowercase">
															({section.items.length})
														</span>
													</DropdownMenuLabel>
												</>
											)}
											{section.items.map((inst) => {
												const isSelected = selectedInstanceId === inst.id
												const colorDef = getColorTagDefinition(inst.colorTag)
												return (
													<DropdownMenuItem
														key={inst.id}
														className={`flex items-center gap-2.5 py-2 text-xs ${
															isSelected ? "font-semibold text-primary" : ""
														}`}
														onClick={() => {
															return setSelectedInstanceId(inst.id)
														}}
													>
														<InstanceIcon
															icon={inst.icon}
															loader={inst.loader}
															size={18}
															className="size-4.5 shrink-0 rounded"
														/>
														<div className="flex min-w-0 flex-1 flex-col">
															<div className="flex min-w-0 items-center gap-1.5">
																{colorDef && (
																	<span
																		className="size-2 shrink-0 rounded-full"
																		style={{ backgroundColor: colorDef.hex }}
																		title={colorDef.defaultLabel}
																	/>
																)}
																<span className="truncate text-xs font-medium text-foreground">
																	{inst.name}
																</span>
															</div>
															<span className="font-mono text-[10px] text-muted-foreground">
																{inst.loader} {inst.gameVersion}
															</span>
														</div>
														{isSelected && <Check className="size-3.5 shrink-0 text-primary" />}
													</DropdownMenuItem>
												)
											})}
										</DropdownMenuGroup>
									)
								})}
							</DropdownMenuContent>
						</DropdownMenu>
					</div>

					<div className="space-y-1.5">
						<Label className="text-xs font-medium text-foreground">{t("worlds.selectZip")}</Label>
						<InputGroup className="h-10 w-full">
							<InputGroupAddon>
								<FileArchive className="size-4 text-muted-foreground" />
							</InputGroupAddon>
							<InputGroupInput
								readOnly
								value={fileName}
								placeholder={t("worlds.selectZip")}
								className="cursor-pointer font-mono text-xs"
								onClick={handleSelectFile}
							/>
							<InputGroupAddon align="inline-end">
								<Button
									type="button"
									variant="ghost"
									size="sm"
									className="h-8 gap-1.5 px-2.5 text-xs text-muted-foreground hover:text-foreground"
									onClick={handleSelectFile}
									disabled={isImporting}
								>
									<FolderOpen className="size-3.5" />
									Browse
								</Button>
							</InputGroupAddon>
						</InputGroup>
					</div>
				</div>

				<DialogFooter className="flex items-center justify-end gap-2 pt-2">
					<Button
						variant="outline"
						className="h-9 px-4 text-xs"
						onClick={() => {
							return onOpenChange(false)
						}}
						disabled={isImporting}
					>
						Cancel
					</Button>
					<Button
						className="h-9 gap-2 px-4 text-xs"
						onClick={handleImport}
						disabled={isImporting || !selectedInstanceId || !zipPath}
					>
						{isImporting ? (
							<>
								<Spinner className="mr-2 size-4" />
								{t("worlds.importing")}
							</>
						) : (
							<>
								<Upload className="size-4" />
								{t("worlds.importWorld")}
							</>
						)}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	)
}

ImportWorldDialog.displayName = "ImportWorldDialog"

export default ImportWorldDialog
