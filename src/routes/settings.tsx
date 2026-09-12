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
import { Plus, ShieldCheck } from "lucide-react"
import { memo, useMemo, useState } from "react"
import { createPortal } from "react-dom"
import AddAccountDialog from "@/components/accounts/add-account-dialog"
import SkinPreviewDialog from "@/components/accounts/skin-preview-dialog"
import {
	AccountCardContent,
	SortableAccountItem,
} from "@/components/accounts/sortable-account-item"
import MemoryAllocation from "@/components/settings/memory-allocation"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { cn } from "@/lib/utils"
import { useAccounts } from "@/services/account-service"
import type { AccountProfile } from "@/types/account"

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

const SettingsPage = () => {
	const { accounts, setActiveAccount, removeAccount, reorderAccounts } = useAccounts()
	const [isAddOpen, setIsAddOpen] = useState(false)
	const [previewAccount, setPreviewAccount] = useState<AccountProfile | null>(null)
	const [activeId, setActiveId] = useState<string | null>(null)

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
		setActiveId(event.active.id as string)
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
		<div className="mx-auto flex w-full max-w-5xl flex-1 flex-col gap-8 pb-8">
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
							<Button size="sm" onClick={() => setIsAddOpen(true)} className="gap-1.5 text-xs">
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
											onPreviewSkin={setPreviewAccount}
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

				{/* Java Runtime */}
				<div className="flex flex-col gap-2 rounded-xl border border-border/40 bg-zinc-900/40 p-4 sm:p-5">
					<h3 className="font-semibold text-foreground text-sm">Default Java Runtime</h3>
					<p className="text-muted-foreground text-xs">
						Java 21 is required for Minecraft 1.20.5 and newer.
					</p>
					<div className="mt-2 flex flex-col gap-2 sm:flex-row sm:gap-3">
						<Input
							defaultValue="C:\Program Files\Eclipse Adoptium\jdk-21.0.3.9-hotspot\bin\javaw.exe"
							className="flex-1 font-mono text-xs"
						/>
						<Button variant="outline" className="shrink-0">
							Browse
						</Button>
					</div>
				</div>

				{/* Memory Allocation */}
				<MemoryAllocation />

				{/* Window & Launch Behavior */}
				<div className="flex flex-col gap-2 rounded-xl border border-border/40 bg-zinc-900/40 p-4 sm:p-5">
					<h3 className="font-semibold text-foreground text-sm">Launcher Behavior</h3>
					<p className="text-muted-foreground text-xs">
						Choose what happens when an instance is launched.
					</p>
					<div className="mt-3 flex flex-wrap gap-2">
						<Button size="sm" variant="outline" className="flex-1 sm:flex-none">
							Keep Launcher Open
						</Button>
						<Button size="sm" variant="outline" className="flex-1 sm:flex-none">
							Hide Launcher to System Tray
						</Button>
					</div>
				</div>
			</div>

			<AddAccountDialog open={isAddOpen} onOpenChange={setIsAddOpen} />
			<SkinPreviewDialog
				account={previewAccount}
				open={Boolean(previewAccount)}
				onOpenChange={(open) => !open && setPreviewAccount(null)}
			/>
		</div>
	)
}

SettingsPage.displayName = "SettingsPage"

const MemoizedSettingsPage = memo(SettingsPage)

export const Route = createFileRoute("/settings")({
	component: MemoizedSettingsPage,
})
