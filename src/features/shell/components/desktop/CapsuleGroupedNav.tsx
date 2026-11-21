import { useMemo } from "react"
import { useTranslation } from "react-i18next"
import CapsuleGroupMenu from "@/features/shell/components/desktop/capsule-nav/CapsuleGroupMenu"
import CapsuleGroupTrigger from "@/features/shell/components/desktop/capsule-nav/CapsuleGroupTrigger"
import { buildNavGroupSections } from "@/features/shell/components/desktop/capsule-nav/capsuleNavModel"
import { useCapsuleNavMenu } from "@/features/shell/components/desktop/capsule-nav/useCapsuleNavMenu"

interface CapsuleGroupedNavProps {
	currentPath: string
	showDeveloperPages: boolean
}

const CapsuleGroupedNav = ({ currentPath, showDeveloperPages }: CapsuleGroupedNavProps) => {
	const { t } = useTranslation()

	const groups = useMemo(() => {
		return buildNavGroupSections(currentPath, showDeveloperPages)
	}, [currentPath, showDeveloperPages])

	const { state, actions, registry } = useCapsuleNavMenu(groups)

	const activeRouteGroup = groups.find((group) => {
		return group.isActive
	})
	const highlightedGroupId = state.openGroup ?? activeRouteGroup?.id ?? null

	return (
		<>
			<nav
				ref={(node) => {
					registry.nav(node)
				}}
				aria-label={t("shell.primaryNavigationAria")}
				onMouseEnter={actions.cancelCloseTimer}
				onMouseLeave={actions.scheduleClose}
				className="no-scrollbar flex max-w-full min-w-0 items-center overflow-x-auto rounded-full border border-border bg-foreground/5 p-0.5 backdrop-blur-md"
			>
				{groups.map((group) => {
					return (
						<CapsuleGroupTrigger
							key={group.id}
							group={group}
							isMenuOpen={state.openGroup === group.id}
							hasPill={highlightedGroupId === group.id}
							onRegisterTrigger={registry.trigger}
							onOpen={actions.openGroupMenu}
							onToggle={actions.toggleGroupMenu}
						/>
					)
				})}
			</nav>

			<CapsuleGroupMenu
				activeGroupSection={state.activeGroupSection}
				anchor={state.anchor}
				panelHeight={state.panelHeight}
				slideDirection={state.slideDirection}
				currentPath={currentPath}
				onRegisterPopup={registry.popup}
				onRegisterContent={registry.content}
				onMouseEnter={actions.cancelCloseTimer}
				onMouseLeave={actions.scheduleClose}
				onSelectItem={actions.closeImmediately}
			/>
		</>
	)
}

export default CapsuleGroupedNav
