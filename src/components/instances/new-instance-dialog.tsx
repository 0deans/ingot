import { memo, useState } from "react"
import { Button } from "@/components/ui/button"
import {
	Dialog,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogTitle,
} from "@/components/ui/dialog"
import { Input } from "@/components/ui/input"

interface NewInstanceDialogProps {
	open: boolean
	onOpenChange: (open: boolean) => void
	onCreateInstance: (name: string) => void
}

const NewInstanceDialog = ({ open, onOpenChange, onCreateInstance }: NewInstanceDialogProps) => {
	const [instanceName, setInstanceName] = useState("")

	const handleCreate = () => {
		if (!instanceName.trim()) return
		onCreateInstance(instanceName.trim())
		setInstanceName("")
		onOpenChange(false)
	}

	return (
		<Dialog open={open} onOpenChange={onOpenChange}>
			<DialogContent className="sm:max-w-md">
				<DialogHeader>
					<DialogTitle>Create New Instance</DialogTitle>
					<DialogDescription>
						Configure a new Minecraft installation with your preferred version and mod loader.
					</DialogDescription>
				</DialogHeader>
				<div className="grid gap-4 py-4">
					<div className="grid gap-2">
						<label htmlFor="instance-name" className="font-medium text-muted-foreground text-xs">
							Instance Name
						</label>
						<Input
							id="instance-name"
							value={instanceName}
							onChange={(e) => setInstanceName(e.target.value)}
							placeholder="e.g. Vanilla 1.21.4"
							onKeyDown={(e) => {
								if (e.key === "Enter") handleCreate()
							}}
						/>
					</div>
				</div>
				<DialogFooter>
					<Button variant="outline" onClick={() => onOpenChange(false)}>
						Cancel
					</Button>
					<Button onClick={handleCreate} disabled={!instanceName.trim()}>
						Create
					</Button>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	)
}

NewInstanceDialog.displayName = "NewInstanceDialog"

export default memo(NewInstanceDialog)
