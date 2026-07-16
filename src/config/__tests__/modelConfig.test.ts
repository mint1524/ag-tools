/**
 * Standalone assertion script for categorizeModel and getModelProtectionKey.
 *
 * Imports from the standalone utils module (no @lobehub/icons dependency,
 * runs under plain Node.js via npx tsx).
 *
 * Run: npx tsx src/config/__tests__/modelConfig.test.ts
 */
import { categorizeModel, getModelProtectionKey, getModelDisplayName, type ModelCategory } from '../../utils/modelCategory';

let passed = 0;
let failed = 0;

function test(description: string, fn: () => void): void {
    try {
        fn();
        passed++;
    } catch (e: unknown) {
        failed++;
        const msg = e instanceof Error ? e.message : String(e);
        console.error(`  FAIL: ${description}  — ${msg}`);
    }
}

function assertEqual<T>(actual: T, expected: T): void {
    if (actual !== expected) {
        throw new Error(`expected "${expected}", got "${actual}"`);
    }
}

const categorizeCases: Array<[string, ModelCategory]> = [
    // canonical
    ['gemini-3.5-flash', 'gemini-flash'],
    ['gemini-3.1-pro', 'gemini-pro'],
    // physical / variants
    ['gemini-3-flash-agent', 'gemini-flash'],
    ['gemini-3.5-flash-low', 'gemini-flash'],
    ['gemini-3.5-flash-extra-low', 'gemini-flash'],
    ['gemini-pro-agent', 'gemini-pro'],
    ['gemini-3.1-pro-low', 'gemini-pro'],
    // legacy
    ['gemini-3-flash', 'gemini-flash'],
    ['gemini-3-pro-high', 'gemini-pro'],
    ['gemini-3.1-pro-high', 'gemini-pro'],
    ['gemini-3-pro-low', 'gemini-pro'],
    // image split
    ['gemini-3.1-flash-image', 'gemini-flash-image'],
    ['gemini-3-pro-image', 'gemini-pro-image'],
    ['imagen-3.0', 'gemini-pro-image'],
    // claude
    ['claude-sonnet-4-6', 'claude'],
    ['claude-opus-4-6-thinking', 'claude'],
    // edge / other providers
    ['gpt-4o', 'other'],
    ['gpt-oss-120b-medium', 'other'],
];

for (const [name, expected] of categorizeCases) {
    test(`categorizeModel("${name}")`, () => {
        assertEqual(categorizeModel(name), expected);
    });
}

const protectionCases: Array<[string, string | null]> = [
    ['gemini-3.5-flash', 'gemini-3-flash'],
    ['gemini-3.1-pro', 'gemini-3-pro-high'],
    ['gemini-3.1-flash-image', 'gemini-3.1-flash-image'],
    ['gemini-3-pro-image', 'gemini-3-pro-image'],
    ['claude-sonnet-4-6', 'claude'],
    ['gpt-4o', null],
];

for (const [name, expected] of protectionCases) {
    test(`getModelProtectionKey("${name}")`, () => {
        assertEqual(getModelProtectionKey(name), expected);
    });
}

// ── getModelDisplayName ──────────────────────────────────────────────────────

type ModelInput = { name: string; display_name?: string } | null | undefined;

const displayNameCases: Array<[ModelInput, string | undefined, string]> = [
    [{ name: 'gemini-3-pro-high', display_name: 'Gemini 3.1 Pro High' }, undefined, 'Gemini 3.1 Pro High'],
    [{ name: 'gemini-3-flash' }, undefined, 'gemini-3-flash'],
    [{ name: 'gemini-3.1-flash-image', display_name: undefined }, undefined, 'gemini-3.1-flash-image'],
    [undefined, 'Claude 系列', 'Claude 系列'],
    [null, undefined, ''],
    [{ name: 'claude-opus-4-6-thinking', display_name: 'Claude Opus 4.6 TK' }, undefined, 'Claude Opus 4.6 TK'],
];

for (const [model, fallback, expected] of displayNameCases) {
    const label = model === null
        ? 'getModelDisplayName(null)'
        : model === undefined
            ? `getModelDisplayName(undefined, '${fallback}')`
            : `getModelDisplayName({name:'${model.name}'${model.display_name ? `, display_name:'${model.display_name}'` : ''}})`;
    test(label, () => {
        assertEqual(getModelDisplayName(model, fallback), expected);
    });
}

if (failed > 0) {
    throw new Error(`${failed} test(s) failed`);
}
