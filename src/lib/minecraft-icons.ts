import {
	Apple,
	ArrowUp,
	Ban,
	BowArrow,
	Bug,
	Clover,
	Droplets,
	Dumbbell,
	Eye,
	EyeOff,
	Feather,
	Fish,
	Flame,
	FlaskConical,
	Footprints,
	Hammer,
	Heart,
	HeartCrack,
	HeartPulse,
	type LucideIcon,
	Moon,
	Network,
	Pickaxe,
	Shield,
	Shirt,
	Skull,
	Snail,
	Snowflake,
	Sparkles,
	Sun,
	Sword,
	Target,
	Waves,
	Wind,
	Zap,
} from "lucide-react"
import { prettyId, stripNamespace } from "@/lib/minecraft"

const effectIcons: Record<string, LucideIcon> = {
	absorption: Shield,
	bad_omen: Skull,
	blindness: EyeOff,
	breath_of_the_nautilus: Waves,
	conduit_power: Waves,
	darkness: Moon,
	dolphins_grace: Fish,
	fire_resistance: Flame,
	glowing: Sun,
	haste: Pickaxe,
	health_boost: Heart,
	hero_of_the_village: Shield,
	hunger: Apple,
	infested: Bug,
	instant_damage: HeartCrack,
	instant_health: HeartPulse,
	invisibility: EyeOff,
	jump_boost: ArrowUp,
	levitation: Feather,
	luck: Clover,
	mining_fatigue: Pickaxe,
	nausea: Eye,
	night_vision: Eye,
	oozing: Droplets,
	poison: FlaskConical,
	raid_omen: Skull,
	regeneration: HeartPulse,
	resistance: Shield,
	saturation: Apple,
	slow_falling: Feather,
	slowness: Snail,
	speed: Zap,
	strength: Dumbbell,
	trial_omen: Skull,
	unluck: Ban,
	water_breathing: Waves,
	weakness: HeartCrack,
	weaving: Network,
	wind_charged: Wind,
	wither: Skull,
}

const enchantmentIcons: Record<string, LucideIcon> = {
	aqua_affinity: Waves,
	bane_of_arthropods: Bug,
	binding_curse: Ban,
	blast_protection: Shield,
	breach: Hammer,
	channeling: Zap,
	density: Hammer,
	depth_strider: Footprints,
	efficiency: Pickaxe,
	feather_falling: Feather,
	fire_aspect: Flame,
	fire_protection: Flame,
	flame: Flame,
	fortune: Clover,
	frost_walker: Snowflake,
	impaling: Sword,
	infinity: BowArrow,
	knockback: Sword,
	looting: Clover,
	loyalty: Target,
	luck_of_the_sea: Fish,
	lunge: Sword,
	lure: Fish,
	mending: HeartPulse,
	multishot: BowArrow,
	piercing: BowArrow,
	power: BowArrow,
	projectile_protection: Shield,
	protection: Shield,
	punch: BowArrow,
	quick_charge: BowArrow,
	respiration: Waves,
	riptide: Waves,
	sharpness: Sword,
	silk_touch: Pickaxe,
	smite: Sword,
	soul_speed: Footprints,
	sweeping: Sword,
	sweeping_edge: Sword,
	swift_sneak: Footprints,
	thorns: Shirt,
	unbreaking: Shield,
	vanishing_curse: Ban,
	wind_burst: Wind,
}

function vanillaId(id: string): string | null {
	if (id.includes(":") && !id.startsWith("minecraft:")) return null
	const name = stripNamespace(id)
	return /^[a-z0-9_]+$/.test(name) ? name : null
}

export function effectIcon(id: string): LucideIcon {
	const name = vanillaId(id)
	return (name && effectIcons[name]) || Sparkles
}

export function enchantmentIcon(id: string): LucideIcon {
	const name = vanillaId(id)
	return (name && enchantmentIcons[name]) || Sparkles
}

export function enchantmentName(id: string): string {
	const name = vanillaId(id)
	if (name === "binding_curse") return "Curse of Binding"
	if (name === "vanishing_curse") return "Curse of Vanishing"
	return prettyId(id)
}
