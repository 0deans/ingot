import { describe, expect, it } from "vitest"
import { resolveNextOverlayKind } from "@/features/shell/stores/useShellOverlayStore"
import { calculateTotalProcessRamMb, type RunningProcess } from "@/shared/stores/useProcessStore"

const MOCK_PROCESSES: RunningProcess[] = [
	{
		id: "p-1",
		name: "Fabric",
		kind: "instance",
		engine: "Java",
		account: "Alex",
		pid: 100,
		ram: "2.5 GB",
		cpu: "10%",
		uptime: "00:05:00",
		colorDot: "bg-emerald-400",
		logs: [],
	},
	{
		id: "p-2",
		name: "Utility",
		kind: "server",
		engine: "Paper",
		account: "25565",
		pid: 101,
		ram: "512 MB",
		cpu: "2%",
		uptime: "00:02:00",
		colorDot: "bg-sky-400",
		logs: [],
	},
	{
		id: "p-3",
		name: "Unknown",
		kind: "instance",
		engine: "Rust",
		account: "Alex",
		pid: 102,
		ram: "invalid",
		cpu: "0%",
		uptime: "00:00:00",
		colorDot: "bg-rose-400",
		logs: [],
	},
]

describe("Shell overlay state transitions & RAM aggregation", () => {
	it("switches exclusively between overlays and closes when toggling active layoutKey", () => {
		const initial = resolveNextOverlayKind({ kind: "none" }, "notifications", "sonoma:0:0")
		expect(initial).toEqual({ kind: "notifications", layoutKey: "sonoma:0:0" })

		const switched = resolveNextOverlayKind(initial, "processes", "sonoma:0:0")
		expect(switched).toEqual({ kind: "processes", layoutKey: "sonoma:0:0" })

		const closed = resolveNextOverlayKind(switched, "processes", "sonoma:0:0")
		expect(closed).toEqual({ kind: "none" })

		const reopenedAcrossLayout = resolveNextOverlayKind(switched, "processes", "capsule:0:0")
		expect(reopenedAcrossLayout).toEqual({ kind: "processes", layoutKey: "capsule:0:0" })
	})

	it("sums GB and MB process RAM allocations while skipping unparseable values", () => {
		expect(calculateTotalProcessRamMb(MOCK_PROCESSES)).toBe(2.5 * 1024 + 512)
	})
})
