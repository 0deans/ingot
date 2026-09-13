import { Popover as PopoverPrimitive } from "@base-ui/react/popover"
import { cn } from "cn"
import { Check, ChevronDown, Search, X } from "lucide-react"
import { memo, useEffect, useMemo, useRef, useState } from "react"

export interface SearchableSelectOption {
	value: string
	label: string
	badge?: string
}

interface SearchableSelectProps {
	id?: string
	value: string
	onValueChange: (value: string) => void
	options: SearchableSelectOption[]
	placeholder?: string
	searchPlaceholder?: string
	disabled?: boolean
	className?: string
}

export const SearchableSelect = memo(
	({
		id,
		value,
		onValueChange,
		options,
		placeholder = "Select an option",
		searchPlaceholder = "Search...",
		disabled = false,
		className,
	}: SearchableSelectProps) => {
		const [isOpen, setIsOpen] = useState(false)
		const [searchQuery, setSearchQuery] = useState("")
		const inputRef = useRef<HTMLInputElement>(null)

		const selectedOption = useMemo(() => {
			return options.find((opt) => opt.value === value)
		}, [options, value])

		const filteredOptions = useMemo(() => {
			if (!searchQuery.trim()) return options
			const q = searchQuery.toLowerCase().trim()
			return options.filter(
				(opt) =>
					opt.label.toLowerCase().includes(q) ||
					opt.value.toLowerCase().includes(q) ||
					Boolean(opt.badge?.toLowerCase().includes(q)),
			)
		}, [options, searchQuery])

		// Auto-focus search input when opened
		useEffect(() => {
			if (isOpen) {
				const timer = setTimeout(() => {
					inputRef.current?.focus()
				}, 60)
				return () => clearTimeout(timer)
			}
			setSearchQuery("")
		}, [isOpen])

		const handleSelect = (val: string) => {
			onValueChange(val)
			setIsOpen(false)
		}

		return (
			<PopoverPrimitive.Root open={isOpen} onOpenChange={setIsOpen}>
				<PopoverPrimitive.Trigger
					id={id}
					disabled={disabled}
					className={cn(
						"flex h-9 w-full select-none items-center justify-between gap-2 rounded-md border border-input bg-zinc-900/90 px-3 py-1.5 font-medium text-foreground text-xs shadow-xs outline-none transition-[color,box-shadow] hover:bg-zinc-800/60 focus-visible:border-ring focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50",
						isOpen && "border-ring ring-1 ring-ring",
						className,
					)}
				>
					<span className="truncate">
						{selectedOption ? (
							<span className="flex items-center gap-1.5">
								<span>{selectedOption.label}</span>
								{selectedOption.badge && (
									<span className="rounded bg-zinc-800 px-1.5 py-0.5 font-normal text-[10px] text-zinc-400">
										{selectedOption.badge}
									</span>
								)}
							</span>
						) : (
							<span className="text-muted-foreground">{placeholder}</span>
						)}
					</span>
					<ChevronDown
						className={cn(
							"size-4 shrink-0 text-muted-foreground opacity-60 transition-transform duration-150",
							isOpen && "rotate-180",
						)}
					/>
				</PopoverPrimitive.Trigger>

				<PopoverPrimitive.Portal>
					<PopoverPrimitive.Positioner
						className="isolate z-50 outline-none"
						side="bottom"
						sideOffset={4}
						align="start"
					>
						<PopoverPrimitive.Popup className="data-open:fade-in-0 data-open:zoom-in-95 data-closed:fade-out-0 data-closed:zoom-out-95 z-50 flex w-(--anchor-width) min-w-56 max-w-sm flex-col rounded-lg border border-border/70 bg-zinc-950 p-1.5 text-popover-foreground shadow-2xl outline-none ring-1 ring-white/10 duration-100 data-closed:animate-out data-open:animate-in">
							{/* Fixed Search Bar at top */}
							<div className="relative flex items-center border-border/50 border-b px-2 pt-0.5 pb-1.5">
								<Search className="size-3.5 shrink-0 text-muted-foreground" />
								<input
									ref={inputRef}
									type="text"
									value={searchQuery}
									onChange={(e) => setSearchQuery(e.target.value)}
									placeholder={searchPlaceholder}
									className="h-7 w-full bg-transparent px-2 text-foreground text-xs outline-none placeholder:text-muted-foreground/60"
									onKeyDown={(e) => {
										if (e.key === "Enter" && filteredOptions.length > 0) {
											e.preventDefault()
											handleSelect(filteredOptions[0].value)
										}
									}}
								/>
								{searchQuery && (
									<button
										type="button"
										onClick={() => setSearchQuery("")}
										className="rounded p-0.5 text-muted-foreground hover:text-foreground"
									>
										<X className="size-3" />
									</button>
								)}
							</div>

							{/* Compact Scrollable List (max-h-48, around 5-6 items, with styled scrollbar) */}
							<div className="flex max-h-48 flex-col gap-0.5 overflow-y-auto pt-1 [scrollbar-color:rgba(255,255,255,0.2)_transparent] [scrollbar-width:thin] [&::-webkit-scrollbar-thumb]:rounded-full [&::-webkit-scrollbar-thumb]:bg-white/20 hover:[&::-webkit-scrollbar-thumb]:bg-white/35 [&::-webkit-scrollbar-track]:bg-transparent [&::-webkit-scrollbar]:w-1.5">
								{filteredOptions.length === 0 ? (
									<div className="px-3 py-4 text-center text-muted-foreground text-xs">
										No matching versions
									</div>
								) : (
									filteredOptions.map((opt) => {
										const isSelected = opt.value === value
										return (
											<button
												key={opt.value}
												type="button"
												onClick={() => handleSelect(opt.value)}
												className={cn(
													"relative flex w-full cursor-pointer items-center justify-between rounded-md px-2.5 py-1.5 text-left text-xs transition-colors",
													isSelected
														? "bg-primary/15 font-semibold text-primary"
														: "text-zinc-300 hover:bg-zinc-800/70 hover:text-foreground",
												)}
											>
												<div className="flex items-center gap-2 truncate">
													<span className="truncate">{opt.label}</span>
													{opt.badge && (
														<span className="rounded bg-zinc-800/80 px-1.5 py-0.2 font-mono font-normal text-[10px] text-zinc-400">
															{opt.badge}
														</span>
													)}
												</div>
												{isSelected && <Check className="size-3.5 shrink-0 text-primary" />}
											</button>
										)
									})
								)}
							</div>
						</PopoverPrimitive.Popup>
					</PopoverPrimitive.Positioner>
				</PopoverPrimitive.Portal>
			</PopoverPrimitive.Root>
		)
	},
)

SearchableSelect.displayName = "SearchableSelect"

export default SearchableSelect
