import { memo, type ReactNode } from "react"
import Titlebar from "@/components/layout/titlebar"

interface WindowFrameProps {
	title?: string
	children: ReactNode
}

const WindowFrame = ({ title = "Ingot", children }: WindowFrameProps) => {
	return (
		<div className="relative flex h-screen w-screen flex-col overflow-hidden bg-radial-[at_top_center] from-zinc-900/60 via-zinc-950 to-black text-foreground antialiased selection:bg-primary/20">
			<Titlebar title={title} />
			<div className="flex flex-1 overflow-hidden">{children}</div>
		</div>
	)
}

WindowFrame.displayName = "WindowFrame"

export default memo(WindowFrame)
