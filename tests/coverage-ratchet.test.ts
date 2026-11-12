import { expect, test } from "vitest"
import { checkCoverage } from "../scripts/coverage-ratchet.mjs"

interface CoverageBaselineFixture {
	files: string[]
	minimumLinePercent: number
	sourceRoot?: string
	sourceRoots?: string[]
}

interface CoverageReportFixture {
	data: Array<{
		files: Array<{
			filename: string
			summary: {
				lines: {
					count: number
					covered: number
				}
			}
		}>
	}>
}

test("coverage gate rejects regressions, missing files and malformed counters", () => {
	const baseline: CoverageBaselineFixture = {
		files: ["src/rules.rs"],
		minimumLinePercent: 90,
	}
	const report = (covered: number): CoverageReportFixture => {
		return {
			data: [
				{
					files: [
						{
							filename: "/repo/src/rules.rs",
							summary: { lines: { count: 10, covered } },
						},
					],
				},
			],
		}
	}

	expect(checkCoverage(report(9), baseline).percent).toBe(90)
	expect(() => {
		return checkCoverage(report(8), baseline)
	}).toThrow(/regressed/)
	expect(() => {
		return checkCoverage({ data: [{ files: [] }] }, baseline)
	}).toThrow(/Expected one/)
	expect(() => {
		return checkCoverage(report(11), baseline)
	}).toThrow(/Inconsistent/)
	expect(() => {
		return checkCoverage(report(9), { ...baseline, files: ["src/rules.rs", "src/rules.rs"] })
	}).toThrow(/Duplicate/)
	expect(() => {
		return checkCoverage(report(9), { ...baseline, files: [], sourceRoot: "src/" })
	}).toThrow(/Untracked/)
	expect(() => {
		return checkCoverage(report(9), { ...baseline, files: [], sourceRoots: ["src/"] })
	}).toThrow(/Untracked/)
})
