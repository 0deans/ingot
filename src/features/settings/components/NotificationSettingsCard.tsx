import { Bell } from "lucide-react"
import { useShallow } from "zustand/react/shallow"
import ToastPositionPicker from "@/features/ui-test/components/ToastPositionPicker"
import { useLauncherPrefsStore, type ToastPosition } from "@/shared/stores/useLauncherPrefsStore"
import { appleToast } from "@/shared/ui/sonner"

const POSITION_LABELS: Record<ToastPosition, string> = {
	"top-left": "Top Left",
	"top-center": "Top Center",
	"top-right": "Top Right",
	"bottom-left": "Bottom Left",
	"bottom-center": "Bottom Center",
	"bottom-right": "Bottom Right",
}

const NotificationSettingsCard = () => {
	const { toastPosition, setToastPosition } = useLauncherPrefsStore(
		useShallow((s) => {
			return {
				toastPosition: s.toastPosition,
				setToastPosition: s.setToastPosition,
			}
		}),
	)

	const handleChangePosition = (nextPosition: ToastPosition) => {
		setToastPosition(nextPosition)
		appleToast.info(`Notifications set to ${POSITION_LABELS[nextPosition]}`, {
			description: "Launcher alerts will now appear here.",
			position: nextPosition,
		})
	}

	const handleTriggerPreview = () => {
		appleToast.success("Sample notification preview", {
			description: `Anchored at ${POSITION_LABELS[toastPosition]}.`,
			position: toastPosition,
		})
	}

	return (
		<section className="@container rounded-2xl border border-border bg-card/65 p-4 backdrop-blur-md">
			<div className="mb-3 flex flex-wrap items-center justify-between gap-2">
				<div className="flex items-center gap-2">
					<Bell className="size-4 text-primary" />
					<h2 className="text-[13.5px] font-semibold text-foreground">Notifications</h2>
				</div>

				<button
					type="button"
					onClick={handleTriggerPreview}
					className="inline-flex cursor-pointer items-center gap-1.5 rounded-lg border border-border bg-foreground/5 px-2.5 py-1 text-[11.5px] font-medium text-foreground transition-colors hover:bg-foreground/10"
				>
					<Bell className="size-3.5 text-primary" />
					<span>Test Toast</span>
				</button>
			</div>

			<div className="grid grid-cols-1 gap-4 rounded-xl border border-border/60 bg-foreground/2 p-3.5 @xl:grid-cols-[auto_1fr] @xl:items-center">
				<ToastPositionPicker
					value={toastPosition}
					layoutId="settings-toast-position-card"
					onChange={handleChangePosition}
				/>

				<div className="flex min-w-0 flex-col gap-2 @xl:border-l @xl:border-border/50 @xl:pl-5">
					<p className="text-[12.5px] font-medium text-foreground">Preferred Toast Placement</p>
					<p className="text-[11.5px] leading-relaxed text-muted-foreground">
						Choose which corner or edge of the launcher window displays frosted status banners,
						download progress cards, and stacked alerts.
					</p>
				</div>
			</div>
		</section>
	)
}

export default NotificationSettingsCard
