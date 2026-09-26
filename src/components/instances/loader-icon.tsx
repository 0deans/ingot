import { cn } from "cn"
import type React from "react"
import fabricIcon from "@/assets/loaders/fabric.svg"
import foliaIcon from "@/assets/loaders/folia.svg"
import forgeIcon from "@/assets/loaders/forge.png"
import neoforgeIcon from "@/assets/loaders/neoforge.svg"
import paperIcon from "@/assets/loaders/paper.svg"
import pumpkinIcon from "@/assets/loaders/pumpkin.png"
import purpurIcon from "@/assets/loaders/purpur.svg"
import quiltIcon from "@/assets/loaders/quilt.svg"
import vanillaIcon from "@/assets/loaders/vanilla.svg"
import type { ModLoaderType, ServerCoreType } from "@/bindings"

interface LoaderIconProps extends React.ImgHTMLAttributes<HTMLImageElement> {
	loader: ModLoaderType | ServerCoreType | string
	size?: number | string
}

export const LoaderIcon: React.FC<LoaderIconProps> = ({
	loader,
	size = 20,
	className,
	alt,
	style,
	...props
}) => {
	const normalized = String(loader).toLowerCase()

	let src = vanillaIcon
	let defaultAlt = "Minecraft Vanilla"

	if (normalized === "fabric") {
		src = fabricIcon
		defaultAlt = "Fabric"
	} else if (normalized === "forge") {
		src = forgeIcon
		defaultAlt = "Minecraft Forge"
	} else if (normalized === "neoforge") {
		src = neoforgeIcon
		defaultAlt = "NeoForge"
	} else if (normalized === "quilt") {
		src = quiltIcon
		defaultAlt = "Quilt"
	} else if (normalized === "paper") {
		src = paperIcon
		defaultAlt = "Paper"
	} else if (normalized === "purpur") {
		src = purpurIcon
		defaultAlt = "Purpur"
	} else if (normalized === "folia") {
		src = foliaIcon
		defaultAlt = "Folia"
	} else if (normalized === "pumpkin") {
		src = pumpkinIcon
		defaultAlt = "PumpkinMC"
	}

	return (
		<img
			src={src}
			alt={alt || defaultAlt}
			className={cn("shrink-0 select-none object-contain", className)}
			style={{
				width: typeof size === "number" ? `${size}px` : size,
				height: typeof size === "number" ? `${size}px` : size,
				...style,
			}}
			draggable={false}
			{...props}
		/>
	)
}

export default LoaderIcon
