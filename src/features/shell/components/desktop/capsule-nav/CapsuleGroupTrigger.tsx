import { ChevronDown } from "lucide-react"
import { motion } from "motion/react"
import { useTranslation } from "react-i18next"
import {
	GROUP_ICONS,
	type NavGroupSection,
} from "@/features/shell/components/desktop/capsule-nav/capsuleNavModel"
import { NAV_GROUP_LABEL_KEYS, type NavGroupId } from "@/features/shell/navigation"
import { cn } from "@/shared/lib/utils"

interface CapsuleGroupTriggerProps {
	group: NavGroupSection
	isMenuOpen: boolean
	hasPill: boolean
	onRegisterTrigger: (groupId: NavGroupId, node: HTMLButtonElement | null) => void
	onOpen: (groupId: NavGroupId) => void
	onToggle: (groupId: NavGroupId) => void
}

const getTriggerToneClass = (isActive: boolean, hasPill: boolean): string => {
	if (hasPill) {
		return "text-foreground"
	}
	if (isActive) {
		return "text-foreground/75"
	}
	return "text-muted-foreground hover:text-foreground"
}

const CapsuleGroupTrigger = ({
	group,
	isMenuOpen,
	hasPill,
	onRegisterTrigger,
	onOpen,
	onToggle,
}: CapsuleGroupTriggerProps) => {
	const { t } = useTranslation()

	const GroupIcon = GROUP_ICONS[group.id]
	const groupLabel = t(NAV_GROUP_LABEL_KEYS[group.id])
	const triggerToneClass = getTriggerToneClass(group.isActive, hasPill)

	return (
		<button
			ref={(el) => {
				onRegisterTrigger(group.id, el)
			}}
			type="button"
			aria-expanded={isMenuOpen}
			aria-haspopup="menu"
			aria-label={groupLabel}
			onMouseEnter={() => {
				onOpen(group.id)
			}}
			onClick={() => {
				onToggle(group.id)
			}}
			onKeyDown={(event) => {
				if (event.key === "ArrowDown") {
					event.preventDefault()
					onOpen(group.id)
				}
			}}
			className={cn(
				"relative flex shrink-0 cursor-pointer items-center justify-center gap-1.5 rounded-full px-2.5 py-1 text-[11.5px] font-medium whitespace-nowrap transition-colors outline-none",
				triggerToneClass,
			)}
		>
			{hasPill ? (
				<motion.span
					layoutId="capsule-active-pill"
					transition={{
						type: "spring",
						stiffness: 440,
						damping: 34,
						mass: 0.85,
					}}
					className="absolute inset-0 rounded-full bg-foreground/12 shadow-xs ring-1 ring-border"
				/>
			) : null}

			<span className="relative z-10 flex shrink-0 items-center justify-center">
				<GroupIcon className={cn("size-3.5", group.isActive && "text-primary")} />
				{group.hasBadge ? (
					<span
						aria-hidden="true"
						className="absolute -top-0.5 -right-0.5 size-1.5 rounded-full bg-primary ring-1 ring-sidebar/90 @lg:hidden"
					/>
				) : null}
			</span>

			<span className="relative z-10 hidden @lg:inline">{groupLabel}</span>

			{group.hasBadge ? (
				<span
					aria-hidden="true"
					className="relative z-10 hidden size-1.5 shrink-0 rounded-full bg-primary @lg:inline-block"
				/>
			) : null}

			<ChevronDown
				className={cn(
					"relative z-10 hidden size-3 opacity-70 transition-transform duration-200 @lg:inline-block",
					isMenuOpen && "rotate-180",
				)}
			/>
		</button>
	)
}

export default CapsuleGroupTrigger
