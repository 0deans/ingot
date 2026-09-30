import { Copy } from "lucide-react"
import { useEffect, useState } from "react"
import { useTranslation } from "react-i18next"
import type { InstanceConfig } from "@/bindings"
import { alertTone } from "@/components/common/alert-tones"
import LoaderIcon from "@/components/instances/loader-icon"
import { Alert, AlertDescription } from "@/components/ui/alert"
import { Button } from "@/components/ui/button"
import { Checkbox } from "@/components/ui/checkbox"
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog"
import {
	Field,
	FieldContent,
	FieldDescription,
	FieldLabel,
	FieldTitle,
} from "@/components/ui/field"
import { Input } from "@/components/ui/input"
import { Label } from "@/components/ui/label"
import { Spinner } from "@/components/ui/spinner"
import { instanceService, rpc } from "@/services/instance-service"

interface DuplicateInstanceDialogProps {
	instance: InstanceConfig | null
	open: boolean
	isRunning: boolean
	onOpenChange: (open: boolean) => void
}

/** Copies an instance into a new, independent one, e.g. to try a new version or modpack */
export const DuplicateInstanceDialog = ({
	instance,
	open,
	isRunning,
	onOpenChange,
}: DuplicateInstanceDialogProps) => {
	const { t } = useTranslation()
	const [name, setName] = useState("")
	const [includeWorlds, setIncludeWorlds] = useState(true)
	const [worldCount, setWorldCount] = useState<number | null>(null)
	const [busy, setBusy] = useState(false)
	const [error, setError] = useState<string | null>(null)

	const instanceId = instance?.id
	const instanceName = instance?.name
	useEffect(() => {
		if (!open || !instanceId) return
		setName(t("duplicateInstance.defaultName", { name: instanceName }))
		setIncludeWorlds(true)
		setError(null)
		setWorldCount(null)
		rpc
			.get_instance_worlds(instanceId)
			.then((worlds) => setWorldCount(worlds.length))
			.catch(() => setWorldCount(null))
	}, [open, instanceId, instanceName, t])

	if (!instance) return null

	const copyingWorlds = includeWorlds && worldCount !== 0
	const blocked = copyingWorlds && isRunning

	const handleDuplicate = async () => {
		setBusy(true)
		setError(null)
		try {
			await instanceService.duplicateInstance(instance.id, name, includeWorlds)
			onOpenChange(false)
		} catch (e) {
			setError(String(e))
		} finally {
			setBusy(false)
		}
	}

	return (
		<Dialog open={open} onOpenChange={(next) => !busy && onOpenChange(next)}>
			<DialogContent className="sm:max-w-md">
				<DialogHeader className="gap-2">
					<div className="flex size-11 items-center justify-center rounded-2xl border border-primary/30 bg-primary/10 text-primary">
						<Copy className="size-5" />
					</div>
					<DialogTitle className="font-semibold text-foreground text-lg">
						{t("duplicateInstance.title")}
					</DialogTitle>
					<DialogDescription className="text-muted-foreground text-xs">
						{t("duplicateInstance.description")}
					</DialogDescription>
				</DialogHeader>

				<div className="flex items-center gap-3 rounded-xl border border-border bg-card/50 p-3.5">
					<div className="flex size-10 shrink-0 items-center justify-center rounded-lg border border-border bg-card">
						<LoaderIcon loader={instance.loader} size={22} />
					</div>
					<div className="flex min-w-0 flex-col">
						<span className="truncate font-semibold text-foreground text-sm">{instance.name}</span>
						<span className="text-2xs text-muted-foreground">
							{String(instance.loader).toUpperCase()} • {instance.gameVersion}
						</span>
					</div>
				</div>

				<div className="flex flex-col gap-1.5">
					<Label htmlFor="duplicate-name" className="font-medium text-foreground/80 text-xs">
						{t("duplicateInstance.nameLabel")}
					</Label>
					<Input
						id="duplicate-name"
						value={name}
						onChange={(e) => setName(e.target.value)}
						onKeyDown={(e) => {
							if (e.key === "Enter" && name.trim() && !busy && !blocked) handleDuplicate()
						}}
						disabled={busy}
						autoFocus
					/>
				</div>

				<FieldLabel htmlFor="duplicate-include-worlds">
					<Field orientation="horizontal">
						<Checkbox
							id="duplicate-include-worlds"
							checked={includeWorlds}
							onCheckedChange={setIncludeWorlds}
							disabled={busy}
						/>
						<FieldContent>
							<FieldTitle>
								{t("duplicateInstance.copyWorlds")}
								{worldCount !== null && (
									<span className="font-normal text-muted-foreground">
										{" "}
										({worldCount === 0 ? t("duplicateInstance.noWorldsYet") : worldCount})
									</span>
								)}
							</FieldTitle>
							<FieldDescription>{t("duplicateInstance.copyWorldsDesc")}</FieldDescription>
						</FieldContent>
					</Field>
				</FieldLabel>

				<p className="text-2xs text-muted-foreground">{t("duplicateInstance.note")}</p>

				{blocked && (
					<Alert className={alertTone.warning}>
						<AlertDescription>{t("duplicateInstance.running")}</AlertDescription>
					</Alert>
				)}
				{error && (
					<Alert variant="destructive">
						<AlertDescription>{error}</AlertDescription>
					</Alert>
				)}

				<DialogFooter>
					<Button
						type="button"
						onClick={handleDuplicate}
						disabled={busy || blocked || !name.trim()}
						className="w-full gap-1.5 font-semibold"
					>
						{busy ? <Spinner className="size-4" /> : <Copy className="size-4" />}
						{busy ? t("duplicateInstance.copying") : t("duplicateInstance.action")}
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	)
}

export default DuplicateInstanceDialog
