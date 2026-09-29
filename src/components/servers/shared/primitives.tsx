import type { LucideIcon } from "lucide-react"
import { type ReactNode, useEffect, useRef, useState } from "react"
import type { ItemStack } from "@/bindings"
import { Alert, AlertDescription } from "@/components/ui/alert"
import {
	Card,
	CardAction,
	CardContent,
	CardDescription,
	CardTitle,
	CardHeader as ShadcnCardHeader,
} from "@/components/ui/card"
import {
	Empty,
	EmptyContent,
	EmptyDescription,
	EmptyHeader,
	EmptyMedia,
	EmptyTitle,
} from "@/components/ui/empty"
import { Tabs, TabsList, TabsTrigger } from "@/components/ui/tabs"
import { avatarUrl, itemIconUrls, prettyId } from "@/lib/minecraft"
import { cn } from "@/lib/utils"

/** Panel surfaces are shadcn Cards; CardHeader below adds the app's icon + title layout */
export { Card, CardContent }

export function CardHeader({
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
		<ShadcnCardHeader>
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
		</ShadcnCardHeader>
	)
}

export function EmptyState({
	icon: Icon,
	title,
	description,
	action,
	className,
}: {
	icon: LucideIcon
	title: string
	description?: ReactNode
	action?: ReactNode
	className?: string
}) {
	return (
		<Empty className={className}>
			<EmptyHeader>
				<EmptyMedia variant="icon">
					<Icon />
				</EmptyMedia>
				<EmptyTitle>{title}</EmptyTitle>
				{description && <EmptyDescription>{description}</EmptyDescription>}
			</EmptyHeader>
			{action && <EmptyContent>{action}</EmptyContent>}
		</Empty>
	)
}

/** Tab switcher (shadcn Tabs) for panels that swap their own content */
export function Segmented<T extends string>({
	value,
	onChange,
	options,
	className,
}: {
	value: T
	onChange: (value: T) => void
	options: { value: T; label: ReactNode; icon?: LucideIcon }[]
	className?: string
}) {
	return (
		<Tabs
			value={value}
			onValueChange={(v) => onChange(v as T)}
			className={cn("shrink-0", className)}
		>
			{/* Long translations wrap at spaces (down to their longest word) before the list scrolls */}
			<TabsList className="w-full overflow-x-auto group-data-horizontal/tabs:h-auto">
				{options.map(({ value: v, label, icon: Icon }) => (
					<TabsTrigger key={v} value={v} className="min-w-min whitespace-normal py-1 leading-tight">
						{Icon && <Icon />}
						{label}
					</TabsTrigger>
				))}
			</TabsList>
		</Tabs>
	)
}

/** Player head from their skin; falls back to initials when offline or unknown */
export function PlayerAvatar({
	name,
	size = 36,
	online,
	className,
}: {
	name: string
	size?: number
	online?: boolean
	className?: string
}) {
	const [failed, setFailed] = useState(false)
	return (
		<div className={cn("relative shrink-0", className)} style={{ width: size, height: size }}>
			{failed ? (
				<div className="flex size-full items-center justify-center rounded-lg bg-zinc-800 font-semibold text-xs text-zinc-400">
					{name.slice(0, 2).toUpperCase()}
				</div>
			) : (
				<img
					src={avatarUrl(name, size * 2)}
					alt=""
					loading="lazy"
					onError={() => setFailed(true)}
					className="size-full rounded-lg bg-zinc-800 [image-rendering:pixelated]"
				/>
			)}
			{online !== undefined && (
				<span
					className={cn(
						"absolute -right-0.5 -bottom-0.5 size-2.5 rounded-full ring-2 ring-zinc-950",
						online ? "bg-emerald-400" : "bg-zinc-600",
					)}
				/>
			)}
		</div>
	)
}

/** Item icon that walks through fallback sources */
export function ItemIcon({ id, className }: { id: string; className?: string }) {
	const urls = itemIconUrls(id)
	const [index, setIndex] = useState(0)
	if (index >= urls.length) {
		return (
			<span className={cn("font-bold text-[9px] text-zinc-400 uppercase", className)}>
				{prettyId(id).slice(0, 3)}
			</span>
		)
	}
	return (
		<img
			src={urls[index]}
			alt={prettyId(id)}
			loading="lazy"
			onError={() => setIndex((i) => i + 1)}
			className={cn("object-contain [image-rendering:pixelated]", className)}
		/>
	)
}

/** One inventory slot, styled like the game's */
export function ItemSlot({
	item,
	selected,
	placeholder,
	size = "md",
	onSelect,
}: {
	item?: ItemStack | null
	selected?: boolean
	placeholder?: ReactNode
	size?: "sm" | "md"
	onSelect?: (item: ItemStack) => void
}) {
	const durability =
		item?.maxDamage && item.damage > 0 ? Math.max(0, 1 - item.damage / item.maxDamage) : null
	return (
		<button
			type="button"
			disabled={!item}
			onClick={() => item && onSelect?.(item)}
			title={item ? (item.customName ?? prettyId(item.id)) : undefined}
			className={cn(
				"relative flex aspect-square items-center justify-center rounded-md border bg-zinc-900/80 transition-colors",
				size === "sm" ? "p-1" : "p-1.5",
				selected ? "border-emerald-400/70 bg-emerald-500/10" : "border-zinc-800",
				item && "hover:border-zinc-600",
			)}
		>
			{item ? (
				<>
					<ItemIcon
						key={item.id}
						id={item.id}
						className={cn("size-full", item.enchanted && "drop-shadow-[0_0_4px_#c084fc]")}
					/>
					{item.count > 1 && (
						<span className="absolute right-0.5 bottom-0 font-bold text-[10px] text-white [text-shadow:1px_1px_0_#000]">
							{item.count}
						</span>
					)}
					{durability !== null && (
						<span className="absolute inset-x-1 bottom-0.5 h-0.5 rounded-full bg-zinc-950">
							<span
								className="block h-full rounded-full"
								style={{
									width: `${durability * 100}%`,
									background: `hsl(${durability * 120} 80% 50%)`,
								}}
							/>
						</span>
					)}
				</>
			) : (
				placeholder
			)}
		</button>
	)
}

/** Hearts/hunger style bar: value out of max, drawn as segments */
export function StatBar({
	value,
	max,
	color,
	label,
	icon: Icon,
}: {
	value: number
	max: number
	color: string
	label: string
	icon: LucideIcon
}) {
	const pct = max > 0 ? Math.min(100, (value / max) * 100) : 0
	return (
		<div className="flex flex-col gap-1.5">
			<div className="flex items-center justify-between text-xs">
				<span className="flex items-center gap-1.5 text-zinc-400">
					<Icon className="size-3.5" style={{ color }} />
					{label}
				</span>
				<span className="font-medium font-mono text-zinc-200">
					{Math.round(value * 10) / 10}
					<span className="text-zinc-600">/{max}</span>
				</span>
			</div>
			<div className="h-1.5 overflow-hidden rounded-full bg-zinc-800">
				<div
					className="h-full rounded-full transition-[width] duration-500"
					style={{ width: `${pct}%`, background: color }}
				/>
			</div>
		</div>
	)
}

export function ErrorNote({ children }: { children: ReactNode }) {
	return (
		<Alert variant="destructive">
			<AlertDescription>{children}</AlertDescription>
		</Alert>
	)
}

/**
 * Last non-null value. Sheets are closed by clearing their subject (e.g. the selected
 * player); keeping the old value lets the close animation play with the content intact.
 */
export function useSticky<T>(value: T | null | undefined): T | null {
	const ref = useRef<T | null>(value ?? null)
	if (value != null) ref.current = value
	return ref.current
}

/** Scrollable div whose top/bottom edges fade only while there's more content that way */
export function FadeScroll({
	className,
	children,
	...props
}: React.HTMLAttributes<HTMLDivElement>) {
	const ref = useRef<HTMLDivElement>(null)
	useEffect(() => {
		const el = ref.current
		if (!el) return
		const update = () => {
			el.style.setProperty("--scroll-area-overflow-y-start", `${el.scrollTop}px`)
			el.style.setProperty(
				"--scroll-area-overflow-y-end",
				`${Math.max(0, el.scrollHeight - el.scrollTop - el.clientHeight)}px`,
			)
		}
		update()
		el.addEventListener("scroll", update, { passive: true })
		// Content can grow or shrink (lazy data, images, collapsing rows) without the
		// container resizing, so watch the children's sizes too
		const resize = new ResizeObserver(update)
		const observeChildren = () => {
			resize.disconnect()
			resize.observe(el)
			for (const child of el.children) resize.observe(child)
		}
		observeChildren()
		const mutations = new MutationObserver(() => {
			observeChildren()
			update()
		})
		mutations.observe(el, { childList: true, subtree: true, characterData: true })
		return () => {
			el.removeEventListener("scroll", update)
			resize.disconnect()
			mutations.disconnect()
		}
	}, [])
	return (
		<div ref={ref} className={cn("scroll-fade overflow-y-auto", className)} {...props}>
			{children}
		</div>
	)
}
