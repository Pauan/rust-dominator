import rust from "@wasm-tool/rollup-plugin-rust";
import serve from "rollup-plugin-serve";
import livereload from "rollup-plugin-livereload";

const is_watch = !!process.env.ROLLUP_WATCH;

export default {
    input: {
        index: "./Cargo.toml",
    },
    output: {
        dir: "dist/js",
        format: "iife",
        sourcemap: true,
    },
    plugins: [
        rust({
            serverPath: "js/",
            debug: false,
        }),

        is_watch && serve({
            contentBase: "dist",
            open: true,
        }),

        is_watch && livereload("dist"),
    ],
};
