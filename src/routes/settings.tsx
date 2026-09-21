import {
	closestCenter,
	DndContext,
	type DragEndEvent,
	DragOverlay,
	type DragStartEvent,
	type DropAnimation,
	defaultDropAnimationSideEffects,
	KeyboardSensor,
	PointerSensor,
	useSensor,
	useSensors,
} from "@dnd-kit/core"
import { restrictToParentElement, restrictToVerticalAxis } from "@dnd-kit/modifiers"
import {
	arrayMove,
	SortableContext,
	sortableKeyboardCoordinates,
	verticalListSortingStrategy,
} from "@dnd-kit/sortable"
import { createFileRoute } from "@tanstack/react-router"
import { AppWindow, Layers, Minimize2, Plus, Power, ShieldCheck } from "lucide-react"
import { memo, useMemo, useState } from "react"
import { createPortal } from "react-dom"
import * as v from "valibot"
import AddAccountDialog from "@/components/accounts/add-account-dialog"
import SkinPreviewDialog from "@/components/accounts/skin-preview-dialog"
import {
	AccountCardContent,
	SortableAccountItem,
} from "@/components/accounts/sortable-account-item"
import MemoryAllocation from "@/components/settings/memory-allocation"
import SyncSettings from "@/components/settings/sync-settings"
import WindowSettings from "@/components/settings/window-settings"
import { Button } from "@/components/ui/button"
import { ScrollArea } from "@/components/ui/scroll-area"
import { cn } from "@/lib/utils"
import { useAccounts } from "@/services/account-service"
import { type LauncherBehavior, useLauncherBehavior } from "@/services/settings-service"

export const settingsDialogSchema = v.picklist(["add-account"])

export const settingsSearchSchema = v.object({
	dialog: v.optional(settingsDialogSchema),
	preview: v.optional(v.string()),
})

export type SettingsSearchParams = v.InferOutput<typeof settingsSearchSchema>

const dropAnimationConfig: DropAnimation = {
	duration: 200,
	easing: "ease",
	sideEffects: defaultDropAnimationSideEffects({
		styles: {
			active: {
				opacity: "0",
			},
		},
	}),
}

const modifiers = [restrictToVerticalAxis, restrictToParentElement]

const LAUNCHER_BEHAVIOR_OPTIONS: {
	id: LauncherBehavior
	label: string
	description: string
	icon: typeof AppWindow
}[] = [
	{
		id: "keepOpen",
		label: "Keep Open",
		description: "Stay open in the background while playing",
		icon: AppWindow,
	},
	{
		id: "hideToTray",
		label: "Hide to Tray",
		description: "Minimize to tray and restore on game exit",
		icon: Minimize2,
	},
	{
		id: "close",
		label: "Close Launcher",
		description: "Close Ingot completely after game starts",
		icon: Power,
	},
]

const SettingsPage = () => {
	const search = Route.useSearch()
	const navigate = Route.useNavigate()

	const isAddOpen = search.dialog === "add-account"
	const previewAccountId = search.preview

	const { accounts, setActiveAccount, removeAccount, reorderAccounts } = useAccounts()
	const { behavior, setLauncherBehavior } = useLauncherBehavior()
	const [activeId, setActiveId] = useState<string | null>(null)

	const previewAccount = useMemo(
		() => (previewAccountId ? (accounts.find((a) => a.id === previewAccountId) ?? null) : null),
		[accounts, previewAccountId],
	)

	const activeAccount = useMemo(
		() => accounts.find((acc) => acc.id === activeId) || null,
		[accounts, activeId],
	)

	const sensors = useSensors(
		useSensor(PointerSensor, {
			activationConstraint: {
				distance: 5,
			},
		}),
		useSensor(KeyboardSensor, {
			coordinateGetter: sortableKeyboardCoordinates,
		}),
	)

	const handleDragStart = (event: DragStartEvent) => {
		setActiveId(String(event.active.id))
	}

	const handleDragEnd = (event: DragEndEvent) => {
		const { active, over } = event

		if (over && active.id !== over.id) {
			const oldIndex = accounts.findIndex((acc) => acc.id === active.id)
			const newIndex = accounts.findIndex((acc) => acc.id === over.id)
			if (oldIndex !== -1 && newIndex !== -1) {
				const newOrder = arrayMove(accounts, oldIndex, newIndex)
				reorderAccounts(newOrder.map((a) => a.id))
			}
		}

		setActiveId(null)
	}

	const handleDragCancel = () => {
		setActiveId(null)
	}

	const handleSetActive = async (id: string) => {
		try {
			await setActiveAccount(id)
		} catch (error) {
			console.error("Failed to set active account:", error)
		}
	}

	const handleRemove = async (id: string) => {
		try {
			await removeAccount(id)
		} catch (error) {
			console.error("Failed to remove account:", error)
		}
	}

	return (
		<ScrollArea className="size-full flex-1" scrollFade>
			<div className="mx-auto flex w-full max-w-5xl flex-1 flex-col gap-8 p-4 pb-12 sm:p-5 lg:p-6">
				<div>
					<h1 className="font-bold text-2xl text-foreground tracking-tight">Launcher Settings</h1>
					<p className="text-muted-foreground text-sm">
						Configure accounts, Java runtimes, RAM allocation, and launcher behavior.
					</p>
				</div>

				<div className="flex flex-col gap-6">
					{/* Accounts & Security */}
					<div className="flex flex-col gap-3 rounded-xl border border-border/40 bg-zinc-900/40 p-4 sm:p-5">
						<div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
							<div>
								<h3 className="font-semibold text-foreground text-sm">Accounts & Authentication</h3>
								<p className="text-muted-foreground text-xs">
									Connect your Ely.by or offline accounts. Sensitive tokens are secured in the OS
									Credential Vault.
								</p>
							</div>
							<div className="flex items-center gap-2">
								<Button
									size="sm"
									onClick={() =>
										navigate({
											search: (prev) => ({ ...prev, dialog: "add-account" }),
										})
									}
									className="gap-1.5 text-xs"
								>
									<Plus className="size-3.5" />
									Add Account
								</Button>
							</div>
						</div>

						<div className="flex items-center gap-2 rounded-lg border border-emerald-500/20 bg-emerald-500/5 px-3 py-2 text-emerald-400 text-xs">
							<ShieldCheck className="size-4 shrink-0" />
							<span>
								Native OS Keyring active: Tokens are encrypted using Windows Credential Manager
								(DPAPI) / Keychain.
							</span>
						</div>

						<DndContext
							sensors={sensors}
							collisionDetection={closestCenter}
							modifiers={modifiers}
							onDragStart={handleDragStart}
							onDragEnd={handleDragEnd}
							onDragCancel={handleDragCancel}
						>
							<SortableContext
								items={accounts.map((a) => a.id)}
								strategy={verticalListSortingStrategy}
							>
								<div
									className={cn(
										"mt-2 flex flex-col divide-y divide-border/30 overflow-hidden rounded-lg border border-border/40 bg-zinc-950/60",
										activeId && "select-none",
									)}
								>
									{accounts.length === 0 ? (
										<div className="p-4 text-center text-muted-foreground text-xs">
											No accounts configured yet. Click "Add Account" to get started with Ely.by.
										</div>
									) : (
										accounts.map((acc) => (
											<SortableAccountItem
												key={acc.id}
												account={acc}
												onPreviewSkin={(a) =>
													navigate({
														search: (prev) => ({ ...prev, preview: a.id }),
													})
												}
												onSetActive={handleSetActive}
												onRemove={handleRemove}
											/>
										))
									)}
								</div>
							</SortableContext>
							{typeof document !== "undefined"
								? createPortal(
										<DragOverlay dropAnimation={dropAnimationConfig} modifiers={modifiers}>
											{activeAccount ? (
												<AccountCardContent
													key={activeAccount.id}
													id={activeAccount.id}
													account={activeAccount}
													onPreviewSkin={() => {}}
													onSetActive={() => {}}
													onRemove={() => {}}
													isOverlay
												/>
											) : null}
										</DragOverlay>,
										document.body,
									)
								: null}
						</DndContext>
					</div>

					{/* Memory Allocation */}
					<MemoryAllocation />

					{/* Window & Launch Behavior */}
					<div className="flex flex-col gap-3 rounded-xl border border-border/40 bg-zinc-900/40 p-4 sm:p-5">
						<div className="flex flex-col gap-1">
							<div className="flex items-center gap-2">
								<Layers className="size-4 text-emerald-400" />
								<h3 className="font-semibold text-foreground text-sm">Launcher Behavior</h3>
							</div>
							<p className="text-muted-foreground text-xs">
								Choose what happens when an instance is launched.
							</p>
						</div>
						<div className="grid grid-cols-1 gap-2.5 pt-1 sm:grid-cols-3">
							{LAUNCHER_BEHAVIOR_OPTIONS.map((opt) => {
								const isSelected = behavior === opt.id
								const Icon = opt.icon
								return (
									<button
										key={opt.id}
										type="button"
										onClick={() => setLauncherBehavior(opt.id)}
										className={cn(
											"flex flex-col items-start gap-2 rounded-lg border p-3.5 text-left transition-all",
											isSelected
												? "border-primary/60 bg-primary/10 shadow-sm"
												: "border-border/30 bg-zinc-950/60 hover:border-border/60 hover:bg-zinc-900/60",
										)}
									>
										<div className="flex w-full items-center justify-between">
											<div
												className={cn(
													"flex size-7 items-center justify-center rounded-md transition-colors",
													isSelected
														? "bg-primary text-primary-foreground"
														: "bg-zinc-900 text-muted-foreground",
												)}
											>
												<Icon className="size-3.5" />
											</div>
											{isSelected && (
												<span className="rounded-full bg-primary/20 px-2 py-0.5 font-medium text-[10px] text-primary">
													Active
												</span>
											)}
										</div>
										<div>
											<div
												className={cn(
													"font-medium text-xs sm:text-sm",
													isSelected ? "font-semibold text-foreground" : "text-zinc-300",
												)}
											>
												{opt.label}
											</div>
											<div className="mt-0.5 text-[11px] text-muted-foreground leading-tight">
												{opt.description}
											</div>
										</div>
									</button>
								)
							})}
						</div>
					</div>

					{/* Window & Display */}
					<WindowSettings />

					{/* Game Data Synchronization */}
					<SyncSettings />
				</div>

				<AddAccountDialog
					open={isAddOpen}
					onOpenChange={(open) =>
						navigate({
							search: (prev) => ({
								...prev,
								dialog: open ? "add-account" : undefined,
							}),
						})
					}
				/>
				<SkinPreviewDialog
					account={previewAccount}
					open={Boolean(previewAccount)}
					onOpenChange={(open) =>
						navigate({
							search: (prev) => ({
								...prev,
								preview: open ? prev.preview : undefined,
							}),
						})
					}
				/>
			</div>
		</ScrollArea>
	)
}

SettingsPage.displayName = "SettingsPage"

const MemoizedSettingsPage = memo(SettingsPage)

export const Route = createFileRoute("/settings")({
	validateSearch: settingsSearchSchema,
	component: MemoizedSettingsPage,
})
