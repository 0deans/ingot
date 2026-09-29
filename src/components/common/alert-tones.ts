/**
 * Extra tones for shadcn's Alert, which only ships `default` and `destructive`. Like
 * `destructive`, a tone colors the text and icon and keeps the regular alert background.
 *
 *   <Alert className={alertTone.warning}>…</Alert>
 */
export const alertTone = {
	success: "text-emerald-400 *:data-[slot=alert-description]:text-emerald-400/90",
	warning: "text-amber-400 *:data-[slot=alert-description]:text-amber-400/90",
	info: "text-sky-400 *:data-[slot=alert-description]:text-sky-400/90",
} as const
