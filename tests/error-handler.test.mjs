import assert from "node:assert/strict"
import { test } from "node:test"
import {
	buildDiagnosticReport,
	buildGitHubIssueBody,
	buildGitHubIssueUrl,
	collectDeviceDetails,
	detectArchitecture,
	detectOperatingSystem,
	getErrorMessage,
	getErrorStack,
	isRecord,
} from "../src/lib/error-report.ts"

test("isRecord narrows object values and rejects null/primitives", () => {
	assert.equal(isRecord({ foo: "bar" }), true)
	assert.equal(isRecord(new Error("failure")), true)
	assert.equal(isRecord(null), false)
	assert.equal(isRecord(undefined), false)
	assert.equal(isRecord("error string"), false)
	assert.equal(isRecord(42), false)
})

test("getErrorMessage handles standard errors, custom messages, strings, and fallbacks", () => {
	const standardError = new Error("Cannot read properties of undefined (reading 'length')")
	assert.equal(
		getErrorMessage(standardError),
		"Cannot read properties of undefined (reading 'length')",
	)

	const emptyError = new Error("")
	assert.equal(getErrorMessage(emptyError), "Error")

	const stringError = "Failed to fetch remote manifest"
	assert.equal(getErrorMessage(stringError), "Failed to fetch remote manifest")

	const objectError = { message: "Network timed out after 5000ms" }
	assert.equal(getErrorMessage(objectError), "Network timed out after 5000ms")

	const unknownError = 12345
	assert.equal(getErrorMessage(unknownError), "An unexpected error occurred")
})

test("getErrorStack extracts stack traces from Errors and plain objects", () => {
	const errorWithStack = new Error("Something broke")
	assert.ok(typeof getErrorStack(errorWithStack) === "string")

	const customObjectWithStack = { stack: "Error: custom\n    at functionA (file.js:1:1)" }
	assert.equal(
		getErrorStack(customObjectWithStack),
		"Error: custom\n    at functionA (file.js:1:1)",
	)

	assert.equal(getErrorStack("no stack"), null)
	assert.equal(getErrorStack(null), null)
})

test("buildDiagnosticReport formats diagnostic output with version, error message, and stacks", () => {
	const error = new Error("Cannot read properties of undefined (reading 'length')")
	const report = buildDiagnosticReport(error, {
		componentStack: "\n    at InstanceCard\n    at InstancesPage",
	})

	assert.ok(report.includes("Ingot Crash Report"))
	assert.ok(report.includes("Cannot read properties of undefined (reading 'length')"))
	assert.ok(report.includes("Stack Trace:"))
	assert.ok(report.includes("Component Stack:"))
	assert.ok(report.includes("InstanceCard"))
})

test("buildGitHubIssueBody formats markdown sections with error, environment, and details", () => {
	const error = new Error("Cannot read properties of undefined (reading 'length')")
	const body = buildGitHubIssueBody(error, {
		componentStack: "\n    at InstanceCard\n    at InstancesPage",
	})

	assert.ok(body.includes("### What happened?"))
	assert.ok(body.includes("Cannot read properties of undefined (reading 'length')"))
	assert.ok(body.includes("### Device & Environment"))
	assert.ok(body.includes("- **Ingot Version**: v"))
	assert.ok(body.includes("### Stack Trace"))
	assert.ok(body.includes("<details>"))
	assert.ok(body.includes("### Component Stack"))
	assert.ok(body.includes("InstanceCard"))
})

test("buildGitHubIssueUrl creates prefilled GitHub URL with title and body query params", () => {
	const error = new Error("Cannot read properties of undefined (reading 'length')")
	const urlString = buildGitHubIssueUrl(error, { componentStack: "\n    at InstanceCard" })

	assert.ok(urlString.startsWith("https://github.com/0deans/ingot/issues/new?"))
	const url = new URL(urlString)
	const title = url.searchParams.get("title")
	const body = url.searchParams.get("body")

	assert.ok(title && title.includes("[Bug]: Cannot read properties of undefined"))
	assert.ok(body && body.includes("### What happened?"))
	assert.ok(body && body.includes("InstanceCard"))
})

test("detectOperatingSystem accurately identifies major platforms from user agent", () => {
	assert.equal(
		detectOperatingSystem("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"),
		"Windows 10/11",
	)
	assert.equal(
		detectOperatingSystem("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36"),
		"macOS",
	)
	assert.equal(detectOperatingSystem("Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36"), "Linux")
	assert.equal(
		detectOperatingSystem("Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36"),
		"Android",
	)
})

test("detectArchitecture identifies x64, arm64, and 32-bit platforms", () => {
	assert.equal(detectArchitecture("Win64; x64"), "x64")
	assert.equal(detectArchitecture("aarch64; Linux"), "arm64")
	assert.equal(detectArchitecture("arm64; macOS"), "arm64")
	assert.equal(detectArchitecture("i686; Linux"), "x86")
})

test("collectDeviceDetails returns complete device attributes and integrates into reports", () => {
	const details = collectDeviceDetails()
	assert.ok(typeof details.os === "string")
	assert.ok(typeof details.arch === "string")
	assert.ok(details.appMode === "mobile" || details.appMode === "desktop")

	const error = new Error("Test device failure")
	const report = buildDiagnosticReport(error)
	assert.ok(report.includes("Device & Environment:"))
	assert.ok(report.includes("- OS:"))
	assert.ok(report.includes("- Display:"))

	const issueBody = buildGitHubIssueBody(error)
	assert.ok(issueBody.includes("### Device & Environment"))
	assert.ok(issueBody.includes("- **Operating System**:"))
	assert.ok(issueBody.includes("- **CPU Cores**:"))
})
