import rust from "@wasm-tool/rollup-plugin-rust";
import { terser } from "rollup-plugin-terser";

export default {
    input: {
        popup: "./Cargo.toml",
    },
    output: {
        dir: "dist/js",
        format: "es",
        sourcemap: true,
    },
    plugins: [
        rust({
            serverPath: "js/",
        }),

        terser(),
    ],
};
