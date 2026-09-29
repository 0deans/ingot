/**
 * Extra tones for shadcn's Alert, which only ships `default` and `destructive`. Like
 * `destructive`, a tone colors the text and icon and keeps the regular alert background.
 *
 *   <Alert className={alertTone.warning}>…</Alert>
 */
export const alertTone = {
	success: "text-primary *:data-[slot=alert-description]:text-primary/90",
	warning: "text-warning *:data-[slot=alert-description]:text-warning/90",
	info: "text-info *:data-[slot=alert-description]:text-info/90",
} as const
