import { defineConfig } from "rollup";
import { readFileSync } from "fs";
import { join } from "path";
import { cwd } from "process";
import typescript from "@rollup/plugin-typescript";
import terser from "@rollup/plugin-terser";
import replace from "@rollup/plugin-replace";
import { nodeResolve } from "@rollup/plugin-node-resolve";

const pkg = JSON.parse(readFileSync(join(cwd(), "package.json"), "utf8"));

const outputDir = "./dist";

export default defineConfig([
    {
        input: "src/index.ts",
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
        input: "src/iife.ts",
        output: {
            file: join(outputDir, "iife.js"),
            format: "iife",
        },

        plugins: [
            typescript(),
            nodeResolve(),
            replace({
                preventAssignment: true,
                "process.env.NODE_ENV": JSON.stringify("production"),
            }),
            terser(),
        ],
    },
]);
