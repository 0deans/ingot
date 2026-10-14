import type {
	DetectedLauncher,
	ImportableInstance,
	ImportInstanceOptions,
	ImportReport,
} from "@/bindings"
import { rpc, unwrapRpc } from "@/lib/rpc"

export const importerService = {
	async getDetectedLaunchers(): Promise<DetectedLauncher[]> {
		try {
			return await unwrapRpc(rpc.instances.imports.get_detected_launchers())
		} catch (error) {
			console.error("Failed to get detected launchers:", error)
			return []
		}
	},

	async getLauncherInstances(
		launcherId: string,
		customPath: string | null = null,
	): Promise<ImportableInstance[]> {
		try {
			return await unwrapRpc(rpc.instances.imports.get_launcher_instances(launcherId, customPath))
		} catch (error) {
			console.error(`Failed to get instances for launcher ${launcherId}:`, error)
			return []
		}
	},

	async detectCustomInstance(path: string): Promise<ImportableInstance | null> {
		try {
			return await unwrapRpc(rpc.instances.imports.detect_custom_instance(path))
		} catch (error) {
			console.error("Failed to detect custom instance:", error)
			return null
		}
	},

	async importInstance(options: ImportInstanceOptions): Promise<ImportReport> {
		try {
			return await unwrapRpc(rpc.instances.imports.import_instance(options))
		} catch (error) {
			console.error("Failed to import instance:", error)
			throw error
		}
	},
}
