import { readFileSync } from "node:fs"
import { fileURLToPath } from "node:url"

const isRecord = (value) => {
	return typeof value === "object" && value !== null && !Array.isArray(value)
}

export const checkCoverage = (report, baseline) => {
	if (!isRecord(baseline) || !Array.isArray(baseline.files)) {
		throw new Error("Invalid coverage baseline")
	}
	const minimum = baseline.minimumLinePercent
	if (typeof minimum !== "number" || minimum < 0 || minimum > 100) {
		throw new Error("Invalid coverage threshold")
	}
	if (!isRecord(report) || !Array.isArray(report.data)) {
		throw new Error("Invalid LLVM coverage report")
	}
	const summaries = new Map()
	for (const data of report.data) {
		if (!isRecord(data) || !Array.isArray(data.files)) {
			throw new Error("Coverage report has no file summaries")
		}
		for (const file of data.files) {
			if (!isRecord(file) || typeof file.filename !== "string" || !isRecord(file.summary)) {
				throw new Error("Invalid coverage file summary")
			}
			const lines = file.summary.lines
			if (!isRecord(lines) || !Number.isInteger(lines.count) || !Number.isInteger(lines.covered)) {
				throw new Error("Invalid coverage line counts")
			}
			if (lines.count < 0 || lines.covered < 0 || lines.covered > lines.count) {
				throw new Error("Inconsistent coverage line counts")
			}
			summaries.set(file.filename.replaceAll("\\", "/"), lines)
		}
	}
	const scope = baseline.sourceRoot
	if (scope !== undefined && (typeof scope !== "string" || !scope.endsWith("/"))) {
		throw new Error("Invalid coverage source root")
	}
	const expectedFiles = new Set(baseline.files)
	if (expectedFiles.size !== baseline.files.length) {
		throw new Error("Duplicate coverage baseline file")
	}
	if (scope !== undefined) {
		for (const filename of summaries.keys()) {
			const offset = filename.lastIndexOf(`/${scope}`)
			if (offset < 0) continue
			const relative = filename.slice(offset + 1)
			const isTestFile = /(?:^|\/)(?:tests|[^/]+_tests)\.rs$|\/tests\/|\/ipc_contract\.rs$/.test(
				relative,
			)
			if (!isTestFile && !expectedFiles.has(relative)) {
				throw new Error(`Untracked coverage source: ${relative}; update the measured baseline`)
			}
		}
	}
	let count = 0
	let covered = 0
	for (const path of baseline.files) {
		if (typeof path !== "string" || path.length === 0) {
			throw new Error("Invalid coverage baseline path")
		}
		const matches = [...summaries].filter(([filename]) => {
			return filename.endsWith(`/${path}`)
		})
		if (matches.length !== 1) {
			throw new Error(`Expected one coverage summary for ${path}`)
		}
		const lines = matches[0][1]
		count += lines.count
		covered += lines.covered
	}
	if (count === 0) {
		throw new Error("Coverage scope is empty")
	}
	const percent = (covered / count) * 100
	if (percent + 0.000001 < minimum) {
		throw new Error(`Coverage regressed: ${percent.toFixed(2)}% < ${minimum.toFixed(2)}%`)
	}
	return { covered, count, percent }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
	const reportPath = process.argv[2]
	const baselinePath = process.argv[3]
	if (!reportPath || !baselinePath) {
		throw new Error("Usage: coverage-ratchet.mjs <llvm-json> <baseline-json>")
	}
	const report = JSON.parse(readFileSync(reportPath, "utf8"))
	const baseline = JSON.parse(readFileSync(baselinePath, "utf8"))
	const result = checkCoverage(report, baseline)
	process.stdout.write(
		`Coverage: ${result.covered}/${result.count} lines (${result.percent.toFixed(2)}%)\n`,
	)
}
