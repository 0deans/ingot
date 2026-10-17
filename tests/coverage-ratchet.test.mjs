import assert from "node:assert/strict"
import { test } from "node:test"
import { checkCoverage } from "../scripts/coverage-ratchet.mjs"

test("coverage gate rejects regressions, missing files and malformed counters", () => {
	const baseline = { files: ["src/rules.rs"], minimumLinePercent: 90 }
	const report = (covered) => {
		return {
			data: [
				{ files: [{ filename: "/repo/src/rules.rs", summary: { lines: { count: 10, covered } } }] },
			],
		}
	}
	assert.equal(checkCoverage(report(9), baseline).percent, 90)
	assert.throws(() => {
		return checkCoverage(report(8), baseline)
	}, /regressed/)
	assert.throws(() => {
		return checkCoverage({ data: [{ files: [] }] }, baseline)
	}, /Expected one/)
	assert.throws(() => {
		return checkCoverage(report(11), baseline)
	}, /Inconsistent/)
	assert.throws(() => {
		return checkCoverage(report(9), { ...baseline, files: ["src/rules.rs", "src/rules.rs"] })
	}, /Duplicate/)
	assert.throws(() => {
		return checkCoverage(report(9), { ...baseline, files: [], sourceRoot: "src/" })
	}, /Untracked/)
})
