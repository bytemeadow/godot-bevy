import argparse
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path


class ExportHandler(SimpleHTTPRequestHandler):
    extensions_map = {
        **SimpleHTTPRequestHandler.extensions_map,
        ".wasm": "application/wasm",
        ".js": "text/javascript",
        ".pck": "application/octet-stream",
    }

    def __init__(self, *args, variant, **kwargs):
        self.variant = variant
        super().__init__(*args, **kwargs)

    def end_headers(self):
        if self.variant == "web":
            self.send_header("Cross-Origin-Opener-Policy", "same-origin")
            self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
        self.send_header("Cache-Control", "no-store")
        super().end_headers()


def make_server(directory, variant, *, bind="127.0.0.1", port=8060):
    if variant not in ("web", "web-nothreads"):
        raise ValueError("variant must be web or web-nothreads")
    directory = Path(directory).resolve(strict=True)
    if not directory.is_dir():
        raise ValueError("export directory must be a directory")
    handler = partial(ExportHandler, directory=str(directory), variant=variant)
    return ThreadingHTTPServer((bind, port), handler)


def main():
    parser = argparse.ArgumentParser(description="Serve a prepared Godot web export.")
    parser.add_argument("directory", type=Path)
    parser.add_argument("--variant", choices=("web", "web-nothreads"), required=True)
    parser.add_argument("--bind", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=8060)
    args = parser.parse_args()
    try:
        server = make_server(args.directory, args.variant, bind=args.bind, port=args.port)
    except (OSError, ValueError) as error:
        parser.error(str(error))
    with server:
        print(f"Serving {args.variant} at http://{args.bind}:{server.server_port}", flush=True)
        try:
            server.serve_forever()
        except KeyboardInterrupt:
            pass


if __name__ == "__main__":
    main()
