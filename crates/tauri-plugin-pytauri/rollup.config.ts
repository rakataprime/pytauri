import path from "path";

import { defineConfig } from "rollup";
import typescript from "@rollup/plugin-typescript";
import terser from "@rollup/plugin-terser";
import replace from "@rollup/plugin-replace";
import { nodeResolve } from "@rollup/plugin-node-resolve";

const inputBaseName = "api-iife";
const inputName = `${inputBaseName}.ts`;
const prodOutputName = `${inputBaseName}.prod.js`;
const devOutputName = `${inputBaseName}.dev.js`;

const format = "iife";
const replacedNodeEnvVarName = "process.env.NODE_ENV";

const outputDir = "./dist";

function getBasePlugins() {
    return [typescript(), nodeResolve()];
}

export default defineConfig([
    {
        input: inputName,
        output: {
            file: path.join(outputDir, prodOutputName),
            format: format,
        },

        plugins: [
            ...getBasePlugins(),
            replace({
                preventAssignment: true,
                [replacedNodeEnvVarName]: JSON.stringify("production"),
            }),
            terser(),
        ],
    },
    {
        input: inputName,
        output: {
            file: path.join(outputDir, devOutputName),
            format: format,
        },

        plugins: [
            ...getBasePlugins(),
            replace({
                preventAssignment: true,
                [replacedNodeEnvVarName]: JSON.stringify("development"),
            }),
        ],
    },
]);
