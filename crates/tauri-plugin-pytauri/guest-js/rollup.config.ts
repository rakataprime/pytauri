import { join } from "path";
import { readFileSync } from "fs";
import { cwd } from "process";

import { defineConfig } from "rollup";
import typescript from "@rollup/plugin-typescript";
import terser from "@rollup/plugin-terser";
import replace from "@rollup/plugin-replace";
import { nodeResolve } from "@rollup/plugin-node-resolve";

const pkg = JSON.parse(readFileSync(join(cwd(), "package.json"), "utf8"));

const inputDir = "./src";
const outputDir = "./dist";

const indexInputName = "index.ts";

const iifeInputBaseName = "api-iife";
const iifeInputName = `${iifeInputBaseName}.ts`;
const iifeProdOutputName = `${iifeInputBaseName}.prod.js`;
const iifeDevOutputName = `${iifeInputBaseName}.dev.js`;

const replacedNodeEnvVarName = "process.env.NODE_ENV";

const preventAssignment = true;

function getIifeBasePlugins() {
    return [typescript(), nodeResolve()];
}

export default defineConfig([
    {
        input: join(inputDir, indexInputName),
        output: [
            {
                file: pkg.exports.import,
                format: "es",
                sourcemap: true,
            },
            {
                file: pkg.exports.require,
                format: "cjs",
                sourcemap: true,
            },
        ],
        plugins: [
            typescript({
                declaration: true,
                declarationDir: outputDir,
                declarationMap: true,
                sourceMap: true,
                inlineSources: true,
            }),
        ],
        external: [
            /^@tauri-apps\/api/,
            ...Object.keys(pkg.dependencies || {}),
            ...Object.keys(pkg.peerDependencies || {}),
        ],
    },
    {
        input: join(inputDir, iifeInputName),
        output: {
            file: join(outputDir, iifeProdOutputName),
            format: "iife",
        },
        plugins: [
            ...getIifeBasePlugins(),
            replace({
                preventAssignment,
                [replacedNodeEnvVarName]: JSON.stringify("production"),
            }),
            terser(),
        ],
    },
    {
        input: join(inputDir, iifeInputName),
        output: {
            file: join(outputDir, iifeDevOutputName),
            format: "iife",
        },
        plugins: [
            ...getIifeBasePlugins(),
            replace({
                preventAssignment,
                [replacedNodeEnvVarName]: JSON.stringify("development"),
            }),
        ],
    },
]);
