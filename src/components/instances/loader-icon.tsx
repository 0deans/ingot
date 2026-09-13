import { cn } from "cn"
import type React from "react"
import fabricIcon from "@/assets/loaders/fabric.svg"
import forgeIcon from "@/assets/loaders/forge.png"
import neoforgeIcon from "@/assets/loaders/neoforge.svg"
import quiltIcon from "@/assets/loaders/quilt.svg"
import vanillaIcon from "@/assets/loaders/vanilla.svg"
import type { ModLoaderType } from "@/bindings"

interface LoaderIconProps extends React.ImgHTMLAttributes<HTMLImageElement> {
	loader: ModLoaderType | string
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
