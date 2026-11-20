import type { TFunction } from "i18next"
import { Bell } from "lucide-react"
import { useTranslation } from "react-i18next"
import { useShallow } from "zustand/react/shallow"
import { useBreakpoint } from "@/features/shell/hooks/useBreakpoint"
import ToastPositionPicker from "@/features/ui-test/components/ToastPositionPicker"
import { useLauncherPrefsStore, type ToastPosition } from "@/shared/stores/useLauncherPrefsStore"
import { Button } from "@/shared/ui/button"
import { appleToast } from "@/shared/ui/sonner"

type ToastPositionLabelKey =
	| "toastPicker.positions.topLeft"
	| "toastPicker.positions.topCenter"
	| "toastPicker.positions.topRight"
	| "toastPicker.positions.bottomLeft"
	| "toastPicker.positions.bottomCenter"
	| "toastPicker.positions.bottomRight"

const POSITION_LABEL_KEYS: Record<ToastPosition, ToastPositionLabelKey> = {
	"top-left": "toastPicker.positions.topLeft",
	"top-center": "toastPicker.positions.topCenter",
	"top-right": "toastPicker.positions.topRight",
	"bottom-left": "toastPicker.positions.bottomLeft",
	"bottom-center": "toastPicker.positions.bottomCenter",
	"bottom-right": "toastPicker.positions.bottomRight",
}

const formatToastPositionLabel = (
	position: ToastPosition,
	isMobile: boolean,
	t: TFunction,
): string => {
	if (!isMobile) {
		return t(POSITION_LABEL_KEYS[position])
	}
	return position.startsWith("top-")
		? t("toastPicker.positions.top")
		: t("toastPicker.positions.bottom")
}

const NotificationSettingsCard = () => {
	const { t } = useTranslation()
	const breakpoint = useBreakpoint()
	const { toastPosition, setToastPosition } = useLauncherPrefsStore(
		useShallow((s) => {
			return {
				toastPosition: s.toastPosition,
				setToastPosition: s.setToastPosition,
			}
		}),
	)

	const isMobile = breakpoint === "mobile"

	const handleChangePosition = (nextPosition: ToastPosition) => {
		setToastPosition(nextPosition)
		appleToast.info(
			t("settings.notifications.updatedToast", {
				position: formatToastPositionLabel(nextPosition, isMobile, t),
			}),
			{
				description: t("settings.notifications.updatedToastDescription"),
				position: nextPosition,
			},
		)
	}

	const handleTriggerPreview = () => {
		appleToast.success(t("settings.notifications.previewToast"), {
			description: t("settings.notifications.previewToastDescription", {
				position: formatToastPositionLabel(toastPosition, isMobile, t),
			}),
			position: toastPosition,
		})
	}

	return (
		<section className="@container rounded-2xl border border-border bg-card/65 p-4 backdrop-blur-md">
			<div className="mb-3 flex flex-wrap items-center justify-between gap-2">
				<div className="flex items-center gap-2">
					<Bell className="size-4 text-primary" />
					<h2 className="text-[13.5px] font-semibold text-foreground">
						{t("settings.notifications.title")}
					</h2>
				</div>

				<Button
					type="button"
					variant="outline"
					size="sm"
					onClick={handleTriggerPreview}
					className="cursor-pointer bg-foreground/5 text-[11.5px] hover:bg-foreground/10"
				>
					<Bell className="size-3.5 text-primary" />
					<span>{t("settings.notifications.testToast")}</span>
				</Button>
			</div>

			<div className="grid grid-cols-1 gap-4 rounded-xl border border-border/60 bg-foreground/2 p-3.5 @xl:grid-cols-[auto_1fr] @xl:items-center">
				<ToastPositionPicker
					value={toastPosition}
					layoutId="settings-toast-position-card"
					onChange={handleChangePosition}
				/>

				<div className="flex min-w-0 flex-col gap-2 @xl:border-l @xl:border-border/50 @xl:pl-5">
					<p className="text-[12.5px] font-medium text-foreground">
						{t("settings.notifications.preferredPlacement")}
					</p>
					<p className="text-[11.5px] leading-relaxed text-muted-foreground">
						{isMobile
							? t("settings.notifications.mobilePlacementDescription")
							: t("settings.notifications.desktopPlacementDescription")}
					</p>
				</div>
			</div>
		</section>
	)
}

export default NotificationSettingsCard
