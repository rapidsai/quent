# Simulator browser demo

This crate runs the simulator analyzer inside a browser Web Worker. The UI uses
the same typed `@quent/client` operations as a normal deployment. The normal
client implements them with HTTP; the demo dispatches them directly to this
WASM API facade without constructing URLs, HTTP requests, or `Response` objects.

`pnpm demo:data` runs the native simulator and writes its events directly to the
ignored `ui/generated/simulator-demo.postcard` build artifact. Both `demo:dev`
and `demo:build` run this step automatically, so recordings are never checked
into the repository.

## Run locally

Enter the Pixi environment and start the demo:

```sh
pixi shell
cd ui
pnpm install --frozen-lockfile
pnpm demo:dev
```

Open <http://localhost:5173/>. No simulator or analyzer server process is
needed. To produce the static artifact instead, run `pnpm demo:build` and serve
`ui/dist/`.
