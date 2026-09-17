const sectionLocations = new Map<string, string>()

export function getTopLevelSection(pathname: string): string {
	const clean = pathname.split("?")[0]
	const segment = clean.split("/").filter(Boolean)[0]
	return segment ? `/${segment}` : "/"
}

export function recordLocation(href: string, pathname: string): void {
	const section = getTopLevelSection(pathname)
	sectionLocations.set(section, href)
}

/**
 * Returns the remembered URL when switching tabs, or resets to the base route if already active.
 */
export function getTabDestination(target: string, currentPathname: string): string {
	const currentSection = getTopLevelSection(currentPathname)
	if (currentSection === target) {
		return target
	}
	return sectionLocations.get(target) ?? target
}
