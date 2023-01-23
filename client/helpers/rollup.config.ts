import { nodeResolve } from "@rollup/plugin-node-resolve";
import typescript from "@rollup/plugin-typescript";
import type { RollupOptions } from "rollup";

const config: RollupOptions = {
    external: /node_modules/u,
    input: "src/index.ts",
    onwarn: (warning, warn) => {
        if (
            warning.code === "UNUSED_EXTERNAL_IMPORT" &&
            warning.exporter === "typia"
        )
            return;
        warn(warning);
    },
    output: {
        dir: "build",
        sourcemap: true,
    },
    plugins: [nodeResolve(), typescript()],
    treeshake: {
        moduleSideEffects: (id) => id !== "typia",
    },
};

export default config;
