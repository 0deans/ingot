import { create } from "zustand"
import { COLOR_TAGS, type GameInstance } from "@/shared/stores/useInstancesStore"

export type ProcessKind = "instance" | "server"

export interface RunningProcess {
	id: string
	name: string
	kind: ProcessKind
	engine: string
	account: string
	pid: number
	ram: string
	cpu: string
	uptime: string
	colorDot: string
	logs: string[]
}

const INITIAL_RUNNING_PROCESSES: RunningProcess[] = [
	{
		id: "proc-1",
		name: "Horizon 0.4.2-nightly",
		kind: "instance",
		engine: "Horizon (Rust)",
		account: "Horizon_RustDev",
		pid: 18420,
		ram: "0.8 GB",
		cpu: "4.2%",
		uptime: "01:14:22",
		colorDot: "bg-rose-400",
		logs: [
			"[16:20:01] [horizon-rt/INFO]: Initialized WGPU Vulkan backend (NVIDIA RTX 4080)",
			"[16:20:02] [horizon-net/INFO]: Connected to smp.ingot.gg:25565 (14ms)",
			"[16:45:19] [horizon-render/INFO]: 940 FPS • Chunk meshers: 8 threads idle",
		],
	},
	{
		id: "proc-2",
		name: "1.21.4 Fabric (Sodium)",
		kind: "instance",
		engine: "Java 21",
		account: "Alex_Craft",
		pid: 24912,
		ram: "4.6 GB",
		cpu: "18.5%",
		uptime: "00:42:10",
		colorDot: "bg-emerald-400",
		logs: [
			"[16:31:10] [main/INFO]: Loading Minecraft 1.21.4 with Fabric Loader 0.16.9",
			"[16:31:14] [Render thread/INFO]: Loaded 48 mods (Sodium 0.6.0, Iris 1.8.1)",
			'[16:32:00] [Render thread/INFO]: Joined world "Survival Highlands Season 4"',
		],
	},
	{
		id: "proc-3",
		name: "Ingot Local SMP Paper Server",
		kind: "server",
		engine: "PaperMC 1.21.4",
		account: "Port :25565 (4/20 Players)",
		pid: 9914,
		ram: "2.4 GB",
		cpu: "6.1%",
		uptime: "04:19:05",
		colorDot: "bg-cyan-400",
		logs: [
			"[12:30:01] [ServerMain/INFO]: Starting Minecraft server on *:25565",
			'[12:30:08] [Server thread/INFO]: Done (6.812s)! For help, type "help"',
			"[16:40:11] [User Authenticator #2/INFO]: UUID of player Alex_Craft is 8667ba71...",
		],
	},
]

interface LaunchRequest {
	instance: GameInstance
	username: string
	ramGb: number
}

export interface ProcessState {
	runningProcesses: RunningProcess[]
	isLaunching: boolean
	launchInstance: (req: LaunchRequest) => void
	stopProcess: (id: string) => void
	stopAllProcesses: () => void
	spawnMockProcess: (
		customName?: string,
		kind?: ProcessKind,
		engine?: string,
		username?: string,
	) => void
}

export const findProcessByName = (
	processes: RunningProcess[],
	name: string,
): RunningProcess | undefined => {
	return processes.find((proc) => {
		return proc.name === name
	})
}

export const selectProcessByName = (name: string) => {
	return (state: ProcessState): RunningProcess | undefined => {
		return findProcessByName(state.runningProcesses, name)
	}
}

let launchTimerId: number | null = null

export const useProcessStore = create<ProcessState>((set, get) => {
	return {
		runningProcesses: INITIAL_RUNNING_PROCESSES,
		isLaunching: false,
		launchInstance: ({ instance, username, ramGb }) => {
			if (get().isLaunching) {
				return
			}
			set({ isLaunching: true })
			if (launchTimerId !== null) {
				window.clearTimeout(launchTimerId)
			}
			launchTimerId = window.setTimeout(() => {
				launchTimerId = null
				set((state) => {
					const alreadyRunning = state.runningProcesses.some((proc) => {
						return proc.name === instance.name
					})
					if (alreadyRunning) {
						return { isLaunching: false }
					}

					const isHorizon = instance.engine === "Horizon (Rust)"
					const nextProc: RunningProcess = {
						id: `proc-${crypto.randomUUID()}`,
						name: instance.name,
						kind: "instance",
						engine: instance.engine,
						account: username,
						pid: Math.floor(10000 + Math.random() * 40000),
						ram: isHorizon ? "0.9 GB" : `${Math.min(ramGb, 6)}.2 GB`,
						cpu: isHorizon ? "4.8%" : "16.4%",
						uptime: "00:00:04",
						colorDot: COLOR_TAGS[instance.colorTag].dot,
						logs: [
							`[ingot-supervisor/INFO]: Spawning ${instance.name} as ${username}`,
							"[runtime/INFO]: Verified SHA-1 assets in 140ms (Streaming Tokio I/O)",
							"[client/INFO]: OpenGL / Vulkan swapchain ready.",
						],
					}
					return {
						isLaunching: false,
						runningProcesses: [nextProc, ...state.runningProcesses],
					}
				})
			}, 900)
		},
		stopProcess: (id) => {
			set((state) => {
				const nextProcesses = state.runningProcesses.filter((proc) => {
					return proc.id !== id
				})

				return { runningProcesses: nextProcesses }
			})
		},
		stopAllProcesses: () => {
			set({ runningProcesses: [] })
		},
		spawnMockProcess: (
			customName,
			kind = "instance",
			engine = "Horizon (Rust)",
			username = "Alex_Craft",
		) => {
			set((state) => {
				const count = state.runningProcesses.length
				const defaultName =
					kind === "server" ? `Local Test Server #${count + 1}` : `Instance #${count + 1}`
				const pid = Math.floor(10000 + Math.random() * 50000)
				const nextProc: RunningProcess = {
					id: `proc-${crypto.randomUUID()}`,
					name: customName ?? defaultName,
					kind,
					engine,
					account: kind === "server" ? `Port :${25565 + count} (1/20)` : username,
					pid,
					ram: kind === "server" ? "2.0 GB" : "1.4 GB",
					cpu: "5.2%",
					uptime: "00:00:01",
					colorDot: kind === "server" ? "bg-cyan-400" : "bg-emerald-400",
					logs: [
						`[ingot-supervisor/INFO]: Process started (PID ${pid})`,
						"[stdout]: Ready and listening for connections.",
					],
				}
				return {
					runningProcesses: [nextProc, ...state.runningProcesses],
				}
			})
		},
	}
})
