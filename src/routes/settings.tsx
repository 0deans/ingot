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
import {
	AppWindow,
	Layers,
	Minimize2,
	Palette,
	Plus,
	Power,
	ShieldCheck,
	Users,
} from "lucide-react"
import { memo, useMemo, useState } from "react"
import { createPortal } from "react-dom"
import { useTranslation } from "react-i18next"
import * as v from "valibot"
import AddAccountDialog from "@/components/accounts/add-account-dialog"
import SkinPreviewDialog from "@/components/accounts/skin-preview-dialog"
import {
	AccountCardContent,
	SortableAccountItem,
} from "@/components/accounts/sortable-account-item"
import { alertTone } from "@/components/common/alert-tones"
import { ScrollArea } from "@/components/common/scroll-area"
import { SectionCardHeader } from "@/components/common/section-card"
import LanguageSettings from "@/components/settings/language-settings"
import { LicensesSettings } from "@/components/settings/licenses"
import MemoryAllocation from "@/components/settings/memory-allocation"
import SyncSettings from "@/components/settings/sync-settings"
import ThemeSettings from "@/components/settings/theme-settings"
import UpdateSettings from "@/components/settings/update-settings"
import WindowSettings from "@/components/settings/window-settings"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import { Card, CardContent } from "@/components/ui/card"
import {
	Field,
	FieldContent,
	FieldDescription,
	FieldLabel,
	FieldTitle,
} from "@/components/ui/field"
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group"
import { cn } from "@/lib/utils"
import { accountService, useAccounts } from "@/services/account-service"
import {
	type LauncherBehavior,
	settingsService,
	useLauncherBehavior,
} from "@/services/settings-service"

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
	icon: typeof AppWindow
}[] = [
	{
		id: "keepOpen",
		icon: AppWindow,
	},
	{
		id: "hideToTray",
		icon: Minimize2,
	},
	{
		id: "close",
		icon: Power,
	},
]

const SettingsPage = () => {
	const { t } = useTranslation()
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
					<h1 className="font-bold text-2xl text-foreground tracking-tight">
						{t("settings.title")}
					</h1>
					<p className="text-muted-foreground text-sm">{t("settings.subtitle")}</p>
				</div>

				<div className="flex flex-col gap-6">
					{/* Language Settings */}
					<LanguageSettings />

					{/* Appearance & Theme Settings */}
					<ThemeSettings />

					{/* Accounts & Security */}
					<Card>
						<SectionCardHeader
							icon={Users}
							title={t("settings.accounts.title")}
							description={t("settings.accounts.description")}
							action={
								<Button
									size="sm"
									onClick={() =>
										navigate({ search: (prev) => ({ ...prev, dialog: "add-account" }) })
									}
								>
									<Plus />
									{t("accounts.addAccount")}
								</Button>
							}
						/>
						<CardContent className="flex flex-col gap-3">
							<Alert className={alertTone.success}>
								<ShieldCheck />
								<AlertDescription>{t("settings.accounts.keyringActive")}</AlertDescription>
							</Alert>

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
											"mt-2 flex flex-col divide-y divide-border/30 overflow-hidden rounded-lg border border-border/40 bg-background/60",
											activeId && "select-none",
										)}
									>
										{accounts.length === 0 ? (
											<div className="p-4 text-center text-muted-foreground text-xs">
												{t("settings.accounts.empty")}
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
						</CardContent>
					</Card>

					{/* Memory Allocation */}
					<MemoryAllocation />

					{/* Window & Launch Behavior */}
					<Card>
						<SectionCardHeader
							icon={Layers}
							title={t("settings.behavior.title")}
							description={t("settings.behavior.description")}
						/>
						<CardContent className="flex flex-col gap-3">
							<RadioGroup
								value={behavior}
								onValueChange={(value) => {
									const option = LAUNCHER_BEHAVIOR_OPTIONS.find((o) => o.id === value)
									if (option) setLauncherBehavior(option.id)
								}}
								className="grid-cols-1 sm:grid-cols-3"
							>
								{LAUNCHER_BEHAVIOR_OPTIONS.map(({ id, icon: Icon }) => (
									<FieldLabel key={id} htmlFor={`behavior-${id}`}>
										<Field orientation="horizontal">
											<Icon className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
											<FieldContent>
												<FieldTitle>{t(`settings.behavior.${id}.label`)}</FieldTitle>
												<FieldDescription>
													{t(`settings.behavior.${id}.description`)}
												</FieldDescription>
											</FieldContent>
											<RadioGroupItem value={id} id={`behavior-${id}`} />
										</Field>
									</FieldLabel>
								))}
							</RadioGroup>
						</CardContent>
					</Card>

					{/* Window & Display */}
					<WindowSettings />

					{/* Game Data Synchronization */}
					<SyncSettings />

					{/* Application Updates */}
					<UpdateSettings />

					<LicensesSettings />

					{/* Design System & UI Components Test Page (Dev mode only) */}
					{import.meta.env.DEV && (
						<Card>
							<SectionCardHeader
								icon={Palette}
								title="UI Components & Design System"
								description="Interactive test page previewing all Base UI components, forms, dialogs, drawers, and Minecraft widgets"
								action={
									<Button size="sm" variant="outline" onClick={() => navigate({ to: "/ui-test" })}>
										Open UI Showcase
									</Button>
								}
							/>
						</Card>
					)}
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
	loader: async () => {
		await Promise.all([accountService.refreshAccounts(), settingsService.preloadSettings()])
	},
	component: MemoizedSettingsPage,
})
