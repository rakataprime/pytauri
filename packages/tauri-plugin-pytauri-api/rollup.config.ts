import { defineConfig } from "rollup";
import { readFileSync } from "fs";
import { join } from "path";
import { cwd } from "process";
import typescript from "@rollup/plugin-typescript";

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
]);
