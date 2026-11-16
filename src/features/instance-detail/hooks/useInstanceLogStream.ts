import { useEffect, useState } from "react"
import { subscribeTauriEvent } from "@/shared/lib/tauriEvents"
import { selectProcessByName, useProcessStore } from "@/shared/stores/useProcessStore"

interface InstanceLogPayload {
	instanceName: string
	line: string
}

const DEFAULT_IDLE_LOGS: string[] = [
	"[18:00:00] [ingot/INFO]: Instance workspace indexed cleanly",
	"[18:00:01] [ingot/INFO]: Ready for launch via Java / Horizon supervisor",
]

export const useInstanceLogStream = (
	instanceName: string,
): {
	logs: string[]
	isRunning: boolean
	clearLogs: () => void
} => {
	const activeProcess = useProcessStore(selectProcessByName(instanceName))

	const [streamedLines, setStreamedLines] = useState<string[]>([])

	useEffect(() => {
		let cancelled = false
		let unlisten: (() => void) | undefined

		void subscribeTauriEvent<InstanceLogPayload>("instance-log", (payload) => {
			if (cancelled || payload.instanceName !== instanceName) {
				return
			}
			setStreamedLines((prev) => {
				return [...prev.slice(-199), payload.line]
			})
		})
			.then((dispose) => {
				if (cancelled) {
					dispose()
				} else {
					unlisten = dispose
				}
			})
			.catch((error: unknown) => {
				console.error("[Ingot] Failed to subscribe to instance-log:", error)
			})

		return () => {
			cancelled = true
			if (unlisten) {
				unlisten()
			}
		}
	}, [instanceName])

	const baseLogs = activeProcess ? activeProcess.logs : DEFAULT_IDLE_LOGS
	const logs = [...baseLogs, ...streamedLines]

	const clearLogs = () => {
		setStreamedLines([])
	}

	return {
		logs,
		isRunning: Boolean(activeProcess),
		clearLogs,
	}
}
