import { createFileRoute, redirect } from "@tanstack/react-router"
import {
	Activity,
	AlertCircle,
	AlertTriangle,
	Bell,
	Bold,
	CheckCircle2,
	ChevronDown,
	Copy,
	Gamepad2,
	Globe,
	HardDrive,
	Hash,
	Info,
	Italic,
	Layers,
	Maximize2,
	Moon,
	Package,
	Palette,
	Play,
	Plus,
	RefreshCw,
	Search,
	Server,
	Settings,
	Sliders,
	SlidersHorizontal,
	Sun,
	Trash2,
	Underline,
	X,
	Zap,
} from "lucide-react"
import { AnimatePresence, motion } from "motion/react"
import { memo, useId, useState } from "react"
import { toast } from "sonner"
import { alertTone } from "@/components/common/alert-tones"
import {
	SearchableSelect,
	type SearchableSelectOption,
} from "@/components/common/searchable-select"
import { SectionCardHeader } from "@/components/common/section-card"
import SkinAvatar from "@/components/common/skin-avatar"
import LoaderIcon from "@/components/instances/loader-icon"
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert"
import { Avatar, AvatarBadge, AvatarFallback, AvatarImage } from "@/components/ui/avatar"
import { Badge } from "@/components/ui/badge"
import { Button } from "@/components/ui/button"
import { Card, CardContent } from "@/components/ui/card"
import { Checkbox } from "@/components/ui/checkbox"
import { Collapsible, CollapsibleContent, CollapsibleTrigger } from "@/components/ui/collapsible"
import {
	Combobox,
	ComboboxContent,
	ComboboxEmpty,
	ComboboxInput,
	ComboboxItem,
	ComboboxList,
} from "@/components/ui/combobox"
import {
	Dialog,
	DialogClose,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
	DialogTrigger,
} from "@/components/ui/dialog"
import {
	DropdownMenu,
	DropdownMenuCheckboxItem,
	DropdownMenuContent,
	DropdownMenuGroup,
	DropdownMenuItem,
	DropdownMenuLabel,
	DropdownMenuRadioGroup,
	DropdownMenuRadioItem,
	DropdownMenuSeparator,
	DropdownMenuShortcut,
	DropdownMenuSub,
	DropdownMenuSubContent,
	DropdownMenuSubTrigger,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu"
import {
	Empty,
	EmptyContent,
	EmptyDescription,
	EmptyHeader,
	EmptyMedia,
	EmptyTitle,
} from "@/components/ui/empty"
import {
	Field,
	FieldContent,
	FieldDescription,
	FieldGroup,
	FieldLabel,
	FieldLegend,
	FieldSet,
	FieldTitle,
} from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import {
	InputGroup,
	InputGroupAddon,
	InputGroupButton,
	InputGroupInput,
	InputGroupText,
} from "@/components/ui/input-group"
import {
	Item,
	ItemActions,
	ItemContent,
	ItemDescription,
	ItemGroup,
	ItemMedia,
	ItemSeparator,
	ItemTitle,
} from "@/components/ui/item"
import { Kbd, KbdGroup } from "@/components/ui/kbd"
import { Label } from "@/components/ui/label"
import { Progress, ProgressLabel, ProgressValue } from "@/components/ui/progress"
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group"
import { ScrollArea } from "@/components/ui/scroll-area"
import {
	Select,
	SelectContent,
	SelectGroup,
	SelectItem,
	SelectLabel,
	SelectSeparator,
	SelectTrigger,
	SelectValue,
} from "@/components/ui/select"
import { Separator } from "@/components/ui/separator"
import {
	Sheet,
	SheetClose,
	SheetContent,
	SheetDescription,
	SheetFooter,
	SheetHeader,
	SheetTitle,
	SheetTrigger,
} from "@/components/ui/sheet"
import { Skeleton } from "@/components/ui/skeleton"
import { Slider } from "@/components/ui/slider"
import { Spinner } from "@/components/ui/spinner"
import { Switch } from "@/components/ui/switch"
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { Textarea } from "@/components/ui/textarea"
import { Toggle } from "@/components/ui/toggle"
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group"
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip"
import { springSnappy } from "@/lib/motion"
import { useTheme } from "@/lib/theme"

const COMBOBOX_ITEMS = [
	{ label: "Fabric 1.21.4", value: "fabric-1.21.4" },
	{ label: "Paper 1.21.4", value: "paper-1.21.4" },
	{ label: "NeoForge 1.21.1", value: "neoforge-1.21.1" },
	{ label: "Vanilla 1.20.4", value: "vanilla-1.20.4" },
	{ label: "Forge 1.20.1", value: "forge-1.20.1" },
	{ label: "Quilt 1.21", value: "quilt-1.21" },
	{ label: "Bedrock Dedicated Server", value: "bds-latest" },
]

const SEARCHABLE_OPTIONS: SearchableSelectOption[] = [
	{ value: "us-east", label: "US East (N. Virginia)", badge: "12ms" },
	{ value: "eu-central", label: "Europe (Frankfurt)", badge: "28ms" },
	{ value: "ap-southeast", label: "Asia Pacific (Singapore)", badge: "95ms" },
	{ value: "sa-east", label: "South America (São Paulo)", badge: "120ms" },
]

function UiTestPage() {
	const { themeMode, resolvedTheme, setThemeMode } = useTheme()

	// Component states
	const [sliderVal, setSliderVal] = useState(65)
	const [switchVal, setSwitchVal] = useState(true)
	const [checkboxVal, setCheckboxVal] = useState(true)
	const [radioVal, setRadioVal] = useState("option-1")
	const [selectVal, setSelectVal] = useState("paper")
	const [comboboxVal, setComboboxVal] = useState(COMBOBOX_ITEMS[0])
	const [searchableVal, setSearchableVal] = useState("eu-central")
	const [toggleState, setToggleState] = useState(true)
	const [toggleGroupSingle, setToggleGroupSingle] = useState<string[]>(["medium"])
	const [toggleGroupMulti, setToggleGroupMulti] = useState<string[]>(["bold", "underline"])
	const [collapsibleOpen, setCollapsibleOpen] = useState(false)
	const [dialogOpen, setDialogOpen] = useState(false)
	const [sheetOpen, setSheetOpen] = useState(false)
	const [showSkeleton, setShowSkeleton] = useState(false)
	const [dropdownChecked, setDropdownChecked] = useState(true)
	const [dropdownRadio, setDropdownRadio] = useState("high")
	const [filterTab, setFilterTab] = useState("all")
	const [motionTab, setMotionTab] = useState("springs")
	const [motionBox, setMotionBox] = useState(true)

	const fieldInputId = useId()

	return (
		<ScrollArea className="scroll-fade-y size-full flex-1">
			<div className="mx-auto flex max-w-7xl flex-col gap-8 p-4 pb-24 sm:p-6 lg:p-8">
				{/* Top Hero Header */}
				<div className="flex flex-col justify-between gap-4 rounded-2xl border border-border/60 bg-linear-to-r from-card/80 via-card/50 to-primary/5 p-6 shadow-sm backdrop-blur-md sm:flex-row sm:items-center">
					<div className="flex items-center gap-4">
						<div className="flex size-12 shrink-0 items-center justify-center rounded-2xl border border-primary/30 bg-primary/10 text-primary shadow-inner">
							<Palette className="size-6" />
						</div>
						<div>
							<div className="flex items-center gap-2">
								<h1 className="font-bold text-foreground text-xl sm:text-2xl">
									Design System & UI Components Showcase
								</h1>
								<Badge variant="outline" className="font-mono text-2xs">
									React 19 + Base UI
								</Badge>
							</div>
							<p className="mt-1 text-muted-foreground text-xs sm:text-sm">
								Interactive preview of all buttons, inputs, dialogs, drawers, dropdowns, and custom
								Minecraft widgets.
							</p>
						</div>
					</div>

					{/* Theme & Fast Action Bar */}
					<div className="flex flex-wrap items-center gap-2">
						<Button
							variant="outline"
							size="sm"
							onClick={() => setThemeMode(resolvedTheme === "dark" ? "light" : "dark")}
							className="gap-2 text-xs"
						>
							{resolvedTheme === "dark" ? (
								<Sun className="size-3.5" />
							) : (
								<Moon className="size-3.5" />
							)}
							<span>
								Theme: {themeMode} ({resolvedTheme})
							</span>
						</Button>

						<Button
							variant="outline"
							size="sm"
							onClick={() => {
								toast.success("Design System Toast", {
									description: "Sonner toast notification working perfectly in Ingot!",
								})
							}}
							className="gap-2 text-xs"
						>
							<Bell className="size-3.5 text-primary" />
							<span>Test Toast</span>
						</Button>
					</div>
				</div>

				{/* Category Filter Tabs */}
				<div className="flex items-center justify-between border-border/40 border-b pb-3">
					<Tabs value={filterTab} onValueChange={setFilterTab}>
						<TabsList variant="line">
							<TabsTrigger value="all">All Components</TabsTrigger>
							<TabsTrigger value="buttons">Buttons & Toggles</TabsTrigger>
							<TabsTrigger value="forms">Forms & Inputs</TabsTrigger>
							<TabsTrigger value="display">Data & Feedback</TabsTrigger>
							<TabsTrigger value="overlays">Modals & Overlays</TabsTrigger>
							<TabsTrigger value="minecraft">Ingot & Minecraft</TabsTrigger>
							<TabsTrigger value="motion">Motion & Physics</TabsTrigger>
						</TabsList>
					</Tabs>
				</div>

				{/* SECTION 1: BUTTONS & TOGGLES */}
				{(filterTab === "all" || filterTab === "buttons") && (
					<section className="flex flex-col gap-4">
						<div className="flex items-center gap-2">
							<div className="size-2 rounded-full bg-primary" />
							<h2 className="font-semibold text-base text-foreground">Buttons & Toggles</h2>
						</div>

						<Card>
							<SectionCardHeader
								icon={Activity}
								title="Button Variants & Sizes"
								description="Standard interactive buttons across all variants and sizes with SVG icon support"
							/>
							<CardContent className="flex flex-col gap-6">
								{/* Variants */}
								<div className="flex flex-col gap-2">
									<span className="font-medium text-muted-foreground text-xs uppercase tracking-wider">
										Variants
									</span>
									<div className="flex flex-wrap items-center gap-2.5">
										<Button variant="default">
											<Play className="size-3.5" /> Default
										</Button>
										<Button variant="secondary">Secondary</Button>
										<Button variant="outline">Outline</Button>
										<Button variant="ghost">Ghost</Button>
										<Button variant="destructive">
											<Trash2 className="size-3.5" /> Destructive
										</Button>
										<Button variant="link">Link Button</Button>
										<Button disabled>Disabled</Button>
										<Button variant="default">
											<Spinner className="size-3.5" /> Loading
										</Button>
									</div>
								</div>

								<Separator />

								{/* Sizes */}
								<div className="flex flex-col gap-2">
									<span className="font-medium text-muted-foreground text-xs uppercase tracking-wider">
										Sizes & Icon Buttons
									</span>
									<div className="flex flex-wrap items-center gap-3">
										<Button size="xs">Size XS</Button>
										<Button size="sm">Size SM</Button>
										<Button size="default">Size Default</Button>
										<Button size="lg">Size LG</Button>
										<Button size="icon-xs" variant="outline" aria-label="Icon XS">
											<Plus className="size-3" />
										</Button>
										<Button size="icon-sm" variant="outline" aria-label="Icon SM">
											<Plus className="size-3.5" />
										</Button>
										<Button size="icon" variant="outline" aria-label="Icon Default">
											<Plus className="size-4" />
										</Button>
										<Button size="icon-lg" variant="outline" aria-label="Icon LG">
											<Plus className="size-4.5" />
										</Button>
									</div>
								</div>

								<Separator />

								{/* Toggles & Toggle Groups */}
								<div className="grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-3">
									<div className="flex flex-col gap-2">
										<span className="font-medium text-muted-foreground text-xs uppercase tracking-wider">
											Single Toggle
										</span>
										<div className="flex items-center gap-3">
											<Toggle
												pressed={toggleState}
												onPressedChange={setToggleState}
												aria-label="Toggle Bold"
												variant="outline"
											>
												<Bold className="size-4" />
											</Toggle>
											<span className="text-muted-foreground text-xs">
												State: {toggleState ? "Active (On)" : "Inactive (Off)"}
											</span>
										</div>
									</div>

									<div className="flex flex-col gap-2">
										<span className="font-medium text-muted-foreground text-xs uppercase tracking-wider">
											Toggle Group (Single Choice)
										</span>
										<ToggleGroup
											value={toggleGroupSingle}
											onValueChange={setToggleGroupSingle}
											variant="outline"
										>
											<ToggleGroupItem value="low" aria-label="Low">
												Low
											</ToggleGroupItem>
											<ToggleGroupItem value="medium" aria-label="Medium">
												Medium
											</ToggleGroupItem>
											<ToggleGroupItem value="high" aria-label="High">
												High
											</ToggleGroupItem>
										</ToggleGroup>
									</div>

									<div className="flex flex-col gap-2">
										<span className="font-medium text-muted-foreground text-xs uppercase tracking-wider">
											Toggle Group (Multi-select)
										</span>
										<ToggleGroup
											value={toggleGroupMulti}
											onValueChange={setToggleGroupMulti}
											variant="outline"
										>
											<ToggleGroupItem value="bold" aria-label="Bold">
												<Bold className="size-4" />
											</ToggleGroupItem>
											<ToggleGroupItem value="italic" aria-label="Italic">
												<Italic className="size-4" />
											</ToggleGroupItem>
											<ToggleGroupItem value="underline" aria-label="Underline">
												<Underline className="size-4" />
											</ToggleGroupItem>
										</ToggleGroup>
									</div>
								</div>

								<Separator />

								{/* Keyboard Shortcuts */}
								<div className="flex flex-col gap-2">
									<span className="font-medium text-muted-foreground text-xs uppercase tracking-wider">
										Keyboard Badges (`Kbd` & `KbdGroup`)
									</span>
									<div className="flex flex-wrap items-center gap-4">
										<div className="flex items-center gap-2">
											<span className="text-muted-foreground text-xs">Command Palette:</span>
											<KbdGroup>
												<Kbd>Ctrl</Kbd>
												<Kbd>K</Kbd>
											</KbdGroup>
										</div>
										<div className="flex items-center gap-2">
											<span className="text-muted-foreground text-xs">Quick Save:</span>
											<KbdGroup>
												<Kbd>Ctrl</Kbd>
												<Kbd>S</Kbd>
											</KbdGroup>
										</div>
										<div className="flex items-center gap-2">
											<span className="text-muted-foreground text-xs">Execute:</span>
											<Kbd>Enter ↵</Kbd>
										</div>
										<div className="flex items-center gap-2">
											<span className="text-muted-foreground text-xs">Close:</span>
											<Kbd>Esc</Kbd>
										</div>
									</div>
								</div>
							</CardContent>
						</Card>
					</section>
				)}

				{/* SECTION 2: FORMS & INPUTS */}
				{(filterTab === "all" || filterTab === "forms") && (
					<section className="flex flex-col gap-4">
						<div className="flex items-center gap-2">
							<div className="size-2 rounded-full bg-primary" />
							<h2 className="font-semibold text-base text-foreground">Forms, Fields & Inputs</h2>
						</div>

						<Card>
							<SectionCardHeader
								icon={Sliders}
								title="Form Inputs & InputGroup"
								description="Text fields, addons, search inputs, textareas, and complex input groups"
							/>
							<CardContent className="flex flex-col gap-6">
								{/* Basic Inputs */}
								<div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
									<div className="space-y-1.5">
										<Label htmlFor="standard-input">Standard Text Input</Label>
										<Input id="standard-input" placeholder="e.g. My Survival Server" />
									</div>

									<div className="space-y-1.5">
										<Label htmlFor="disabled-input">Disabled Input</Label>
										<Input id="disabled-input" value="Locked value" disabled />
									</div>

									<div className="space-y-1.5">
										<Label htmlFor="invalid-input">Invalid State Input</Label>
										<Input id="invalid-input" value="invalid@@port" aria-invalid="true" />
									</div>
								</div>

								{/* InputGroup Examples */}
								<div className="flex flex-col gap-2">
									<span className="font-medium text-muted-foreground text-xs uppercase tracking-wider">
										Composite Input Groups
									</span>
									<div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
										{/* Search with Addons */}
										<div className="space-y-1.5">
											<Label>Search with Clear Button</Label>
											<InputGroup>
												<InputGroupAddon align="inline-start">
													<Search className="size-4" />
												</InputGroupAddon>
												<InputGroupInput placeholder="Search mods or packs..." />
												<InputGroupAddon align="inline-end">
													<InputGroupButton size="icon-xs" aria-label="Clear">
														<X className="size-3.5" />
													</InputGroupButton>
												</InputGroupAddon>
											</InputGroup>
										</div>

										{/* URL with Protocol */}
										<div className="space-y-1.5">
											<Label>URL with Prefix & Copy Button</Label>
											<InputGroup>
												<InputGroupAddon align="inline-start">
													<InputGroupText>https://</InputGroupText>
												</InputGroupAddon>
												<InputGroupInput defaultValue="api.modrinth.com" />
												<InputGroupAddon align="inline-end">
													<InputGroupButton
														size="icon-xs"
														onClick={() => toast.info("Copied URL to clipboard")}
														aria-label="Copy"
													>
														<Copy className="size-3.5" />
													</InputGroupButton>
												</InputGroupAddon>
											</InputGroup>
										</div>

										{/* Port with Suffix */}
										<div className="space-y-1.5">
											<Label>Server Port with Icon</Label>
											<InputGroup>
												<InputGroupAddon align="inline-start">
													<Hash className="size-4 text-muted-foreground" />
												</InputGroupAddon>
												<InputGroupInput defaultValue="25565" />
												<InputGroupAddon align="inline-end">
													<Badge variant="secondary" className="h-5 text-3xs">
														TCP / UDP
													</Badge>
												</InputGroupAddon>
											</InputGroup>
										</div>
									</div>
								</div>

								{/* Textarea */}
								<div className="space-y-1.5">
									<Label htmlFor="motd-textarea">Message of the Day (Textarea)</Label>
									<Textarea
										id="motd-textarea"
										rows={3}
										defaultValue="§aWelcome to §bIngot Survival§r! Enjoy your stay."
										placeholder="Enter server MOTD description..."
									/>
								</div>

								<Separator />

								{/* The Field Component System */}
								<div className="flex flex-col gap-3">
									<span className="font-medium text-muted-foreground text-xs uppercase tracking-wider">
										Field & FieldSet Abstractions
									</span>
									<FieldSet className="mt-1 rounded-xl border border-border/60 bg-muted/20 p-4 pt-2">
										<FieldLegend className="rounded bg-card px-2 font-semibold text-foreground text-sm">
											Server Process Configuration
										</FieldLegend>
										<FieldGroup className="gap-4">
											<Field>
												<FieldLabel htmlFor={fieldInputId}>Java Executable Path</FieldLabel>
												<Input
													id={fieldInputId}
													defaultValue="C:\Program Files\Eclipse Adoptium\jdk-21\bin\javaw.exe"
												/>
												<FieldDescription>
													Point to custom OpenJDK installation or leave empty for auto-detection.
												</FieldDescription>
											</Field>

											<Separator className="bg-border/40" />

											<Field orientation="horizontal">
												<FieldContent>
													<FieldTitle>Auto-restart on Unexpected Crash</FieldTitle>
													<FieldDescription>
														Automatically attempts to relaunch server process if exit code is
														non-zero.
													</FieldDescription>
												</FieldContent>
												<Switch defaultChecked />
											</Field>
										</FieldGroup>
									</FieldSet>
								</div>

								<Separator />

								{/* Selection & Toggle Controls */}
								<div className="grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-4">
									{/* Switch */}
									<div className="flex flex-col gap-3">
										<span className="font-medium text-muted-foreground text-xs uppercase tracking-wider">
											Switch
										</span>
										<div className="flex items-center gap-3">
											<Switch checked={switchVal} onCheckedChange={setSwitchVal} id="switch-demo" />
											<Label htmlFor="switch-demo">
												{switchVal ? "Enabled (Active)" : "Disabled (Off)"}
											</Label>
										</div>
										<div className="flex items-center gap-3">
											<Switch size="sm" defaultChecked id="switch-demo-sm" />
											<Label htmlFor="switch-demo-sm" className="text-xs">
												Small Size Switch
											</Label>
										</div>
									</div>

									{/* Checkbox */}
									<div className="flex flex-col gap-3">
										<span className="font-medium text-muted-foreground text-xs uppercase tracking-wider">
											Checkbox
										</span>
										<div className="flex items-center gap-2">
											<Checkbox
												checked={checkboxVal}
												onCheckedChange={(checked) => setCheckboxVal(Boolean(checked))}
												id="checkbox-demo"
											/>
											<Label htmlFor="checkbox-demo">Sync servers.dat file</Label>
										</div>
										<div className="flex items-center gap-2">
											<Checkbox defaultChecked disabled id="checkbox-disabled" />
											<Label htmlFor="checkbox-disabled" className="opacity-50">
												Protected setting (Disabled)
											</Label>
										</div>
									</div>

									{/* Radio Group */}
									<div className="flex flex-col gap-3">
										<span className="font-medium text-muted-foreground text-xs uppercase tracking-wider">
											Radio Group
										</span>
										<RadioGroup value={radioVal} onValueChange={setRadioVal}>
											<div className="flex items-center gap-2">
												<RadioGroupItem value="option-1" id="radio-1" />
												<Label htmlFor="radio-1">Keep launcher open</Label>
											</div>
											<div className="flex items-center gap-2">
												<RadioGroupItem value="option-2" id="radio-2" />
												<Label htmlFor="radio-2">Hide to tray</Label>
											</div>
											<div className="flex items-center gap-2">
												<RadioGroupItem value="option-3" id="radio-3" />
												<Label htmlFor="radio-3">Close launcher</Label>
											</div>
										</RadioGroup>
									</div>

									{/* Slider */}
									<div className="flex flex-col gap-3">
										<div className="flex items-center justify-between">
											<span className="font-medium text-muted-foreground text-xs uppercase tracking-wider">
												RAM Slider
											</span>
											<span className="font-mono font-semibold text-primary text-xs">
												{sliderVal * 128} MB
											</span>
										</div>
										<Slider
											value={sliderVal}
											onValueChange={(val) => typeof val === "number" && setSliderVal(val)}
											min={8}
											max={128}
										/>
										<span className="text-3xs text-muted-foreground">
											Min: 1024 MB · Max: 16384 MB
										</span>
									</div>
								</div>

								<Separator />

								{/* Select & Combobox */}
								<div className="grid grid-cols-1 gap-6 sm:grid-cols-3">
									{/* Standard Select */}
									<div className="space-y-1.5">
										<Label>Standard Select Dropdown</Label>
										<Select value={selectVal} onValueChange={(val) => val && setSelectVal(val)}>
											<SelectTrigger className="w-full">
												<SelectValue placeholder="Select server core..." />
											</SelectTrigger>
											<SelectContent>
												<SelectGroup>
													<SelectLabel>Plugin Cores</SelectLabel>
													<SelectItem value="paper">Paper (Recommended)</SelectItem>
													<SelectItem value="purpur">Purpur</SelectItem>
													<SelectItem value="folia">Folia (Multi-threaded)</SelectItem>
												</SelectGroup>
												<SelectSeparator />
												<SelectGroup>
													<SelectLabel>Modded Cores</SelectLabel>
													<SelectItem value="fabric">Fabric</SelectItem>
													<SelectItem value="neoforge">NeoForge</SelectItem>
													<SelectItem value="forge">Forge</SelectItem>
												</SelectGroup>
											</SelectContent>
										</Select>
									</div>

									{/* Base UI Combobox */}
									<div className="space-y-1.5">
										<Label>Combobox Autocomplete</Label>
										<Combobox
											items={COMBOBOX_ITEMS}
											value={comboboxVal}
											onValueChange={(val) => val && setComboboxVal(val)}
											itemToStringLabel={(item) => item.label}
											isItemEqualToValue={(a, b) => a.value === b.value}
										>
											<ComboboxInput placeholder="Type version..." />
											<ComboboxContent>
												<ComboboxList>
													<ComboboxEmpty>No match found</ComboboxEmpty>
													{COMBOBOX_ITEMS.map((item) => (
														<ComboboxItem key={item.value} value={item}>
															{item.label}
														</ComboboxItem>
													))}
												</ComboboxList>
											</ComboboxContent>
										</Combobox>
									</div>

									{/* Ingot SearchableSelect */}
									<div className="space-y-1.5">
										<Label>Ingot SearchableSelect</Label>
										<SearchableSelect
											value={searchableVal}
											onValueChange={setSearchableVal}
											options={SEARCHABLE_OPTIONS}
											placeholder="Pick server region..."
										/>
									</div>
								</div>
							</CardContent>
						</Card>
					</section>
				)}

				{/* SECTION 3: DATA DISPLAY & FEEDBACK */}
				{(filterTab === "all" || filterTab === "display") && (
					<section className="flex flex-col gap-4">
						<div className="flex items-center gap-2">
							<div className="size-2 rounded-full bg-primary" />
							<h2 className="font-semibold text-base text-foreground">Data Display & Feedback</h2>
						</div>

						<Card>
							<SectionCardHeader
								icon={Layers}
								title="Badges, Alerts, Cards & Item Rows"
								description="Component visual elements for status, notifications, and metadata presentation"
							/>
							<CardContent className="flex flex-col gap-6">
								{/* Badges */}
								<div className="flex flex-col gap-2">
									<span className="font-medium text-muted-foreground text-xs uppercase tracking-wider">
										Badges
									</span>
									<div className="flex flex-wrap items-center gap-2">
										<Badge variant="default">Default Badge</Badge>
										<Badge variant="secondary">Secondary</Badge>
										<Badge variant="outline">Outline</Badge>
										<Badge variant="destructive">Destructive</Badge>
										<Badge variant="ghost">Ghost Badge</Badge>
										<Badge variant="default" className="gap-1.5">
											<span className="size-1.5 animate-pulse rounded-full bg-white" />
											Running
										</Badge>
									</div>
								</div>

								<Separator />

								{/* Alerts */}
								<div className="flex flex-col gap-2">
									<span className="font-medium text-muted-foreground text-xs uppercase tracking-wider">
										Alert Notifications
									</span>
									<div className="grid grid-cols-1 gap-3 sm:grid-cols-2">
										<Alert className={alertTone.info}>
											<Info className="size-4" />
											<AlertTitle>Java 21 Required</AlertTitle>
											<AlertDescription>
												Minecraft 1.20.5 and newer automatically runs on Eclipse Adoptium OpenJDK
												21.
											</AlertDescription>
										</Alert>

										<Alert className={alertTone.success}>
											<CheckCircle2 className="size-4" />
											<AlertTitle>Backup Created</AlertTitle>
											<AlertDescription>
												World and mod state saved before version migration. Rollback is available.
											</AlertDescription>
										</Alert>

										<Alert className={alertTone.warning}>
											<AlertTriangle className="size-4" />
											<AlertTitle>Server Needs Restart</AlertTitle>
											<AlertDescription>
												New plugin installed. Changes will take effect once the server reboots.
											</AlertDescription>
										</Alert>

										<Alert variant="destructive">
											<AlertCircle className="size-4" />
											<AlertTitle>Sync Conflict Detected</AlertTitle>
											<AlertDescription>
												Local options.txt differs from Sync Master pool. Choose which copy to keep.
											</AlertDescription>
										</Alert>
									</div>
								</div>

								<Separator />

								{/* Item Rows Abstraction */}
								<div className="flex flex-col gap-2">
									<span className="font-medium text-muted-foreground text-xs uppercase tracking-wider">
										Item & ItemGroup Component (Server / Mod Lists)
									</span>
									<ItemGroup className="rounded-xl border border-border/50 bg-background/50 p-2">
										<Item variant="outline">
											<ItemMedia
												variant="icon"
												className="rounded-lg bg-primary/10 p-2 text-primary"
											>
												<Gamepad2 className="size-5" />
											</ItemMedia>
											<ItemContent>
												<ItemTitle>Fabric 1.21.4 Survival</ItemTitle>
												<ItemDescription>
													Sodium, Iris Shaders, Lithium · Last played 2 hours ago
												</ItemDescription>
											</ItemContent>
											<ItemActions>
												<Badge variant="secondary">Ready</Badge>
												<Button size="sm" variant="default" className="gap-1.5">
													<Play className="size-3" /> Play
												</Button>
											</ItemActions>
										</Item>

										<ItemSeparator />

										<Item variant="outline">
											<ItemMedia variant="icon" className="rounded-lg bg-info/10 p-2 text-info">
												<Server className="size-5" />
											</ItemMedia>
											<ItemContent>
												<ItemTitle>Paper Dedicated Host</ItemTitle>
												<ItemDescription>
													Port: 25565 · 3/20 Players Online · Uptime: 4h 12m
												</ItemDescription>
											</ItemContent>
											<ItemActions>
												<Badge variant="outline" className="border-primary/40 text-primary">
													Online
												</Badge>
												<Button size="sm" variant="outline">
													Console
												</Button>
											</ItemActions>
										</Item>
									</ItemGroup>
								</div>

								<Separator />

								{/* Avatars & Progress */}
								<div className="grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-3">
									{/* Avatars */}
									<div className="flex flex-col gap-3">
										<span className="font-medium text-muted-foreground text-xs uppercase tracking-wider">
											Avatars & Badges
										</span>
										<div className="flex items-center gap-4">
											<Avatar size="sm">
												<AvatarImage src="https://github.com/shadcn.png" alt="User" />
												<AvatarFallback>CN</AvatarFallback>
											</Avatar>

											<Avatar size="default">
												<AvatarImage src="https://github.com/shadcn.png" alt="User" />
												<AvatarFallback>JD</AvatarFallback>
												<AvatarBadge />
											</Avatar>

											<Avatar size="lg">
												<AvatarFallback className="bg-primary/20 font-bold text-primary">
													IN
												</AvatarFallback>
												<AvatarBadge className="bg-emerald-500" />
											</Avatar>
										</div>
									</div>

									{/* Progress Bar */}
									<div className="flex flex-col gap-3">
										<span className="font-medium text-muted-foreground text-xs uppercase tracking-wider">
											Progress Tracker
										</span>
										<Progress value={72}>
											<div className="flex w-full items-center justify-between text-xs">
												<ProgressLabel>Downloading Libraries</ProgressLabel>
												<ProgressValue />
											</div>
										</Progress>
									</div>

									{/* Spinners */}
									<div className="flex flex-col gap-3">
										<span className="font-medium text-muted-foreground text-xs uppercase tracking-wider">
											Spinners
										</span>
										<div className="flex items-center gap-4">
											<Spinner className="size-4 text-muted-foreground" />
											<Spinner className="size-6 text-primary" />
											<Spinner className="size-8 text-info" />
										</div>
									</div>
								</div>

								<Separator />

								{/* Skeleton Preview Toggle */}
								<div className="flex flex-col gap-3">
									<div className="flex items-center justify-between">
										<span className="font-medium text-muted-foreground text-xs uppercase tracking-wider">
											Skeleton Loading Placeholder
										</span>
										<Button
											size="xs"
											variant="outline"
											onClick={() => setShowSkeleton(!showSkeleton)}
										>
											{showSkeleton ? "Show Loaded View" : "Toggle Skeleton State"}
										</Button>
									</div>

									{showSkeleton ? (
										<div className="flex items-center gap-4 rounded-xl border border-border/40 p-4">
											<Skeleton className="size-12 rounded-xl" />
											<div className="flex-1 space-y-2">
												<Skeleton className="h-4 w-48" />
												<Skeleton className="h-3 w-72" />
											</div>
											<Skeleton className="h-8 w-20 rounded-lg" />
										</div>
									) : (
										<div className="flex items-center gap-4 rounded-xl border border-border/40 bg-card/40 p-4">
											<div className="flex size-12 items-center justify-center rounded-xl bg-primary/10 text-primary">
												<Package className="size-6" />
											</div>
											<div className="flex-1">
												<h4 className="font-medium text-foreground text-sm">Iris Shaders</h4>
												<p className="text-muted-foreground text-xs">
													A modern shader mod for Minecraft compatible with Sodium.
												</p>
											</div>
											<Badge variant="outline">Installed</Badge>
										</div>
									)}
								</div>

								<Separator />

								{/* Empty State Component */}
								<div className="flex flex-col gap-2">
									<span className="font-medium text-muted-foreground text-xs uppercase tracking-wider">
										Empty State Component (`Empty`)
									</span>
									<Empty className="border border-border/60 bg-background/20 py-8">
										<EmptyHeader>
											<EmptyMedia variant="icon">
												<Package className="size-5" />
											</EmptyMedia>
											<EmptyTitle>No Modpacks Found</EmptyTitle>
											<EmptyDescription>
												Try adjusting your search filters or browse popular categories from
												Modrinth.
											</EmptyDescription>
										</EmptyHeader>
										<EmptyContent>
											<Button size="sm" variant="outline" className="gap-2">
												<RefreshCw className="size-3.5" /> Reset Filters
											</Button>
										</EmptyContent>
									</Empty>
								</div>
							</CardContent>
						</Card>
					</section>
				)}

				{/* SECTION 4: MODALS & OVERLAYS */}
				{(filterTab === "all" || filterTab === "overlays") && (
					<section className="flex flex-col gap-4">
						<div className="flex items-center gap-2">
							<div className="size-2 rounded-full bg-primary" />
							<h2 className="font-semibold text-base text-foreground">
								Modals, Drawers & Overlays
							</h2>
						</div>

						<Card>
							<SectionCardHeader
								icon={Maximize2}
								title="Dialogs, Sheets, Dropdowns & Tooltips"
								description="Floating UI layer primitives for user confirmation, side panels, and contextual menus"
							/>
							<CardContent className="flex flex-col gap-6">
								<div className="grid grid-cols-1 gap-6 sm:grid-cols-2 lg:grid-cols-4">
									{/* Modal Dialog */}
									<div className="flex flex-col gap-2">
										<span className="font-medium text-muted-foreground text-xs uppercase tracking-wider">
											Dialog Modal
										</span>
										<Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
											<DialogTrigger
												render={
													<Button variant="outline" className="w-full gap-2">
														<Maximize2 className="size-3.5" /> Open Dialog
													</Button>
												}
											/>
											<DialogContent>
												<DialogHeader>
													<DialogTitle>Instance Configuration</DialogTitle>
													<DialogDescription>
														Modify the maximum JVM heap memory allocated to this client.
													</DialogDescription>
												</DialogHeader>
												<div className="space-y-3 py-2">
													<Field>
														<FieldLabel>Allocated RAM (MB)</FieldLabel>
														<Input defaultValue="4096" />
													</Field>
												</div>
												<DialogFooter>
													<DialogClose render={<Button variant="ghost">Cancel</Button>} />
													<Button
														onClick={() => {
															setDialogOpen(false)
															toast.success("Settings Saved")
														}}
													>
														Save Changes
													</Button>
												</DialogFooter>
											</DialogContent>
										</Dialog>
									</div>

									{/* Sliding Sheet / Drawer */}
									<div className="flex flex-col gap-2">
										<span className="font-medium text-muted-foreground text-xs uppercase tracking-wider">
											Sheet / Drawer (Side Panel)
										</span>
										<Sheet open={sheetOpen} onOpenChange={setSheetOpen}>
											<SheetTrigger
												render={
													<Button variant="outline" className="w-full gap-2">
														<SlidersHorizontal className="size-3.5" /> Open Sheet
													</Button>
												}
											/>
											<SheetContent side="right">
												<SheetHeader>
													<SheetTitle>Server Logs Inspection</SheetTitle>
													<SheetDescription>
														Live terminal output and server telemetry parameters.
													</SheetDescription>
												</SheetHeader>
												<div className="flex-1 space-y-4 py-4 text-xs">
													<div className="rounded-lg border border-border bg-muted/40 p-3 font-mono">
														[14:20:00 INFO]: Preparing spawn area: 84%
														<br />
														[14:20:02 INFO]: Done (12.4s)!
														<br />
														[14:20:10 INFO]: Player steve connected.
													</div>
													<div className="space-y-2">
														<Label>Command Injection</Label>
														<Input placeholder="Enter command..." />
													</div>
												</div>
												<SheetFooter>
													<SheetClose render={<Button variant="outline">Close Panel</Button>} />
												</SheetFooter>
											</SheetContent>
										</Sheet>
									</div>

									{/* Rich Dropdown Menu */}
									<div className="flex flex-col gap-2">
										<span className="font-medium text-muted-foreground text-xs uppercase tracking-wider">
											Dropdown Menu
										</span>
										<DropdownMenu>
											<DropdownMenuTrigger
												render={
													<Button variant="outline" className="w-full justify-between">
														<span>Actions Menu</span>
														<ChevronDown className="size-3.5" />
													</Button>
												}
											/>
											<DropdownMenuContent align="start" className="w-56">
												<DropdownMenuGroup>
													<DropdownMenuLabel>Instance Actions</DropdownMenuLabel>
													<DropdownMenuItem onClick={() => toast.info("Launching...")}>
														<Play className="size-4 text-primary" />
														<span>Launch Game</span>
														<DropdownMenuShortcut>F5</DropdownMenuShortcut>
													</DropdownMenuItem>
													<DropdownMenuItem onClick={() => toast.info("Opened folder")}>
														<HardDrive className="size-4" />
														<span>Open Directory</span>
													</DropdownMenuItem>

													<DropdownMenuSub>
														<DropdownMenuSubTrigger>
															<Globe className="size-4" />
															<span>Quick Play Server</span>
														</DropdownMenuSubTrigger>
														<DropdownMenuSubContent>
															<DropdownMenuItem>Hypixel Network</DropdownMenuItem>
															<DropdownMenuItem>Local Host (25565)</DropdownMenuItem>
														</DropdownMenuSubContent>
													</DropdownMenuSub>
												</DropdownMenuGroup>

												<DropdownMenuSeparator />

												<DropdownMenuGroup>
													<DropdownMenuLabel>Toggles</DropdownMenuLabel>
													<DropdownMenuCheckboxItem
														checked={dropdownChecked}
														onCheckedChange={setDropdownChecked}
													>
														Show Snapshots
													</DropdownMenuCheckboxItem>
												</DropdownMenuGroup>

												<DropdownMenuSeparator />

												<DropdownMenuRadioGroup
													value={dropdownRadio}
													onValueChange={setDropdownRadio}
												>
													<DropdownMenuLabel>Memory Priority</DropdownMenuLabel>
													<DropdownMenuRadioItem value="low">Low Priority</DropdownMenuRadioItem>
													<DropdownMenuRadioItem value="high">High Priority</DropdownMenuRadioItem>
												</DropdownMenuRadioGroup>

												<DropdownMenuSeparator />

												<DropdownMenuItem
													className="text-destructive focus:bg-destructive/10"
													onClick={() => toast.error("Instance deleted")}
												>
													<Trash2 className="size-4" />
													<span>Delete Instance</span>
												</DropdownMenuItem>
											</DropdownMenuContent>
										</DropdownMenu>
									</div>

									{/* Tooltips */}
									<div className="flex flex-col gap-2">
										<span className="font-medium text-muted-foreground text-xs uppercase tracking-wider">
											Tooltips
										</span>
										<div className="flex items-center gap-2">
											<Tooltip>
												<TooltipTrigger
													render={
														<Button variant="outline" size="sm" className="flex-1">
															Hover Me
														</Button>
													}
												/>
												<TooltipContent side="top">Tooltip info text on top</TooltipContent>
											</Tooltip>

											<Tooltip>
												<TooltipTrigger
													render={
														<Button variant="outline" size="icon-sm" aria-label="Info">
															<Info className="size-3.5" />
														</Button>
													}
												/>
												<TooltipContent side="bottom">
													Bottom tooltip with shortcut <Kbd>⌘S</Kbd>
												</TooltipContent>
											</Tooltip>
										</div>
									</div>
								</div>

								<Separator />

								{/* Collapsible Accordion */}
								<div className="flex flex-col gap-2">
									<span className="font-medium text-muted-foreground text-xs uppercase tracking-wider">
										Collapsible Section
									</span>
									<Collapsible
										open={collapsibleOpen}
										onOpenChange={setCollapsibleOpen}
										className="rounded-xl border border-border/50 bg-background/40 p-4"
									>
										<div className="flex items-center justify-between">
											<div className="flex items-center gap-2">
												<Settings className="size-4 text-primary" />
												<h4 className="font-medium text-foreground text-sm">
													Advanced JVM Parameters
												</h4>
											</div>
											<CollapsibleTrigger
												render={
													<Button variant="ghost" size="icon-xs" aria-label="Toggle">
														<ChevronDown
															className={`size-4 transition-transform duration-200 ${
																collapsibleOpen ? "rotate-180" : ""
															}`}
														/>
													</Button>
												}
											/>
										</div>
										<CollapsibleContent className="mt-3 space-y-2 border-border/40 border-t pt-3 text-muted-foreground text-xs">
											<p>
												Configured flags:{" "}
												<code>
													-XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=200
												</code>
											</p>
											<p>Active Garbage Collector: G1GC with modern low-latency profiling.</p>
										</CollapsibleContent>
									</Collapsible>
								</div>
							</CardContent>
						</Card>
					</section>
				)}

				{/* SECTION 5: MINECRAFT & INGOT CUSTOM */}
				{(filterTab === "all" || filterTab === "minecraft") && (
					<section className="flex flex-col gap-4">
						<div className="flex items-center gap-2">
							<div className="size-2 rounded-full bg-primary" />
							<h2 className="font-semibold text-base text-foreground">
								Minecraft & Ingot Custom Widgets
							</h2>
						</div>

						<Card>
							<SectionCardHeader
								icon={Gamepad2}
								title="Loader Icons & Ingot Brand Primitives"
								description="Custom Minecraft assets, mod loader SVG icons, and skin renderers"
							/>
							<CardContent className="flex flex-col gap-6">
								{/* Loader Icons */}
								<div className="flex flex-col gap-2">
									<span className="font-medium text-muted-foreground text-xs uppercase tracking-wider">
										Mod Loader & Server Core Icons (`LoaderIcon`)
									</span>
									<div className="grid grid-cols-2 gap-3 sm:grid-cols-5">
										{[
											{ id: "vanilla", name: "Vanilla" },
											{ id: "fabric", name: "Fabric" },
											{ id: "quilt", name: "Quilt" },
											{ id: "neoforge", name: "NeoForge" },
											{ id: "forge", name: "Forge" },
											{ id: "paper", name: "Paper" },
											{ id: "purpur", name: "Purpur" },
											{ id: "folia", name: "Folia" },
											{ id: "pumpkin", name: "PumpkinMC" },
											{ id: "bedrock", name: "Bedrock" },
										].map((loader) => (
											<div
												key={loader.id}
												className="flex items-center gap-2.5 rounded-xl border border-border/50 bg-background/40 p-2.5"
											>
												<LoaderIcon loader={loader.id} size={24} />
												<span className="font-medium text-foreground text-xs">{loader.name}</span>
											</div>
										))}
									</div>
								</div>

								<Separator />

								{/* Skin Avatars */}
								<div className="flex flex-col gap-2">
									<span className="font-medium text-muted-foreground text-xs uppercase tracking-wider">
										2D Skin Avatar Rendering (`SkinAvatar`)
									</span>
									<div className="flex items-center gap-4">
										<SkinAvatar username="Steve" size={32} />
										<SkinAvatar username="Alex" size={40} />
										<SkinAvatar username="Custom" size={48} />
										<div className="flex flex-col">
											<span className="font-medium text-foreground text-xs">
												Pixelated Skin Head Canvas
											</span>
											<span className="text-3xs text-muted-foreground">
												Double-layered Minecraft skin texture renderer with 3D accessory layer
											</span>
										</div>
									</div>
								</div>
							</CardContent>
						</Card>
					</section>
				)}

				{/* SECTION 7: MOTION & PHYSICS ANIMATIONS */}
				{(filterTab === "all" || filterTab === "motion") && (
					<section className="flex flex-col gap-4">
						<div className="flex items-center gap-2">
							<div className="size-2 rounded-full bg-primary" />
							<h2 className="font-semibold text-base text-foreground">
								Motion & Physics (motion/react)
							</h2>
						</div>

						<Card>
							<SectionCardHeader
								icon={Zap}
								title="Motion & Physics Animations (motion/react)"
								description="Declarative gesture feedback, shared layout animations (layoutId), and AnimatePresence unmount lifecycles"
							/>
							<CardContent className="flex flex-col gap-6">
								{/* Shared Layout Pill Selector */}
								<div className="flex flex-col gap-2">
									<span className="font-medium text-muted-foreground text-xs uppercase tracking-wider">
										Shared Layout Pill (`layoutId`)
									</span>
									<div className="inline-flex w-fit items-center gap-1 rounded-xl border border-border/50 bg-background/60 p-1">
										{[
											{ id: "springs", label: "Spring Physics" },
											{ id: "layout", label: "Shared Layout" },
											{ id: "presence", label: "AnimatePresence" },
										].map((tab) => (
											<button
												key={tab.id}
												type="button"
												onClick={() => setMotionTab(tab.id)}
												className={`relative rounded-lg px-3 py-1.5 font-medium text-xs transition-colors ${
													motionTab === tab.id
														? "text-primary"
														: "text-muted-foreground hover:text-foreground"
												}`}
											>
												{motionTab === tab.id && (
													<motion.div
														layoutId="uiTestMotionPill"
														className="absolute inset-0 rounded-lg bg-primary/15"
														transition={springSnappy}
													/>
												)}
												<span className="relative z-10">{tab.label}</span>
											</button>
										))}
									</div>
								</div>

								<Separator />

								{/* Interactive Spring Controls */}
								<div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
									{/* Tap scaling button */}
									<div className="flex flex-col gap-2 rounded-xl border border-border/40 bg-card/40 p-4">
										<span className="font-medium text-foreground text-xs">
											Tactile Press (`whileTap`)
										</span>
										<p className="text-3xs text-muted-foreground">
											Subtle scale-down response on click for natural feel.
										</p>
										<motion.button
											type="button"
											whileTap={{ scale: 0.92 }}
											whileHover={{ scale: 1.02 }}
											className="mt-2 inline-flex h-9 items-center justify-center rounded-lg bg-primary font-medium text-primary-foreground text-xs shadow-sm transition-colors"
										>
											Click or Press Me
										</motion.button>
									</div>

									{/* Hover Lift Card */}
									<motion.div
										whileHover={{ y: -4, transition: { duration: 0.15 } }}
										className="flex flex-col gap-2 rounded-xl border border-border/40 bg-card/40 p-4 shadow-sm"
									>
										<span className="font-medium text-foreground text-xs">
											Hover Lift (`whileHover`)
										</span>
										<p className="text-3xs text-muted-foreground">
											Card elevates by 4px on mouse hover with spring ease.
										</p>
										<div className="mt-2 rounded-lg border border-primary/20 bg-primary/10 p-2 text-center font-mono text-primary text-xs">
											Hover this Card
										</div>
									</motion.div>

									{/* AnimatePresence Toggle */}
									<div className="flex flex-col gap-2 rounded-xl border border-border/40 bg-card/40 p-4">
										<div className="flex items-center justify-between">
											<span className="font-medium text-foreground text-xs">
												Exit Animation (`AnimatePresence`)
											</span>
											<Button size="xs" variant="outline" onClick={() => setMotionBox(!motionBox)}>
												{motionBox ? "Hide" : "Show"}
											</Button>
										</div>
										<p className="text-3xs text-muted-foreground">
											Smoothly animates in AND out upon mount/unmount.
										</p>
										<div className="mt-2 flex h-10 items-center justify-center">
											<AnimatePresence mode="wait">
												{motionBox && (
													<motion.div
														key="motion-box"
														initial={{ opacity: 0, scale: 0.85 }}
														animate={{ opacity: 1, scale: 1 }}
														exit={{ opacity: 0, scale: 0.85 }}
														transition={{ duration: 0.15 }}
														className="w-full rounded-lg bg-emerald-500/20 p-2 text-center font-medium text-emerald-400 text-xs"
													>
														Visible Component
													</motion.div>
												)}
											</AnimatePresence>
										</div>
									</div>
								</div>
							</CardContent>
						</Card>
					</section>
				)}
			</div>
		</ScrollArea>
	)
}

function UiTestRouteComponent() {
	if (!import.meta.env.DEV) {
		return null
	}
	return <UiTestPage />
}

export const Route = createFileRoute("/ui-test")({
	beforeLoad: () => {
		if (!import.meta.env.DEV) {
			throw redirect({ to: "/" })
		}
	},
	component: memo(UiTestRouteComponent),
})
