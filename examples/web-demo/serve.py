#!/usr/bin/env python3
"""
Simple HTTP server with Cross-Origin Isolation headers for Godot web exports.

These headers are required for SharedArrayBuffer support (threaded builds):
- Cross-Origin-Opener-Policy: same-origin
- Cross-Origin-Embedder-Policy: require-corp

Usage:
    python serve.py [port] [directory]

Examples:
    python serve.py                    # Serve current dir on port 8000
    python serve.py 8080               # Serve current dir on port 8080
    python serve.py 8080 ./export      # Serve ./export on port 8080
"""

import http.server
import socketserver
import sys
import os

class CORSRequestHandler(http.server.SimpleHTTPRequestHandler):
    """HTTP request handler with Cross-Origin Isolation headers."""

    def end_headers(self):
        # Required for SharedArrayBuffer (threaded WebAssembly)
        self.send_header('Cross-Origin-Opener-Policy', 'same-origin')
        self.send_header('Cross-Origin-Embedder-Policy', 'require-corp')
        # Cache control for development
        self.send_header('Cache-Control', 'no-cache, no-store, must-revalidate')
        super().end_headers()

    def guess_type(self, path):
        """Add WASM MIME type."""
        if path.endswith('.wasm'):
            return 'application/wasm'
        return super().guess_type(path)

def main():
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8000
    directory = sys.argv[2] if len(sys.argv) > 2 else '.'

    os.chdir(directory)

    with socketserver.TCPServer(("", port), CORSRequestHandler) as httpd:
        print(f"Serving at http://localhost:{port}")
        print(f"Directory: {os.getcwd()}")
        print()
        print("Cross-Origin Isolation headers enabled for SharedArrayBuffer support.")
        print("Press Ctrl+C to stop.")
        print()
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            print("\nServer stopped.")

if __name__ == '__main__':
    main()
