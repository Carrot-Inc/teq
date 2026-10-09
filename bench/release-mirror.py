#!/usr/bin/env python3
"""bench/release-mirror.py <port file> <releases dir> <maven dir>: a local stand-in for a release that is not public
yet (docs/TARGETS.md, "Releases"), for bench/release-smoke.sh --mirror: on 127.0.0.1 at a free port, which it writes
to <port file> once it listens,

  /releases/download/v<version>/<name>   the release's files from <releases dir>/v<version>/, as GitHub serves a
                                         release's: a binary through a redirect to another host name,
                                         http://localhost:<port>/objects/... (which serves every file of the
                                         release), the manifest and SHA256SUMS directly
  /maven/<path>                          a Maven tree from <maven dir>, as Central serves the plugin

answering GET and HEAD with each file's length, 404 for anything else. It serves until it is killed.
"""
import functools
import http.server
import os
import sys


class Mirror(http.server.BaseHTTPRequestHandler):
    def __init__(self, *args, releases, maven, **kwargs):
        self.releases, self.maven = releases, maven
        super().__init__(*args, **kwargs)

    def log_message(self, *args):
        pass

    def answer(self, body):
        path = self.path.split("?", 1)[0]
        name = path.rsplit("/", 1)[-1]
        # A binary's asset, whatever its name's suffix (.exe for Windows alone): any file but the sums and the manifest.
        if path.startswith("/releases/download/") and name != "SHA256SUMS" and not name.endswith("-binaries.txt"):
            port = self.server.server_address[1]
            self.send_response(302)
            self.send_header("Location", f"http://localhost:{port}/objects{path}")
            self.send_header("Content-Length", "0")
            self.end_headers()
            return
        for prefix, root in (("/objects/releases/download/", self.releases), ("/releases/download/", self.releases), ("/maven/", self.maven)):
            if path.startswith(prefix):
                rel = os.path.normpath(path[len(prefix):])
                file = os.path.join(root, rel)
                if not rel.startswith("..") and os.path.isfile(file):
                    with open(file, "rb") as f:
                        data = f.read()
                    self.send_response(200)
                    self.send_header("Content-Length", str(len(data)))
                    self.end_headers()
                    if body:
                        self.wfile.write(data)
                    return
                break
        self.send_response(404)
        self.send_header("Content-Length", "0")
        self.end_headers()

    def do_GET(self):
        self.answer(True)

    def do_HEAD(self):
        self.answer(False)


def main(port_file, releases, maven):
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), functools.partial(Mirror, releases=os.path.abspath(releases), maven=os.path.abspath(maven)))
    with open(port_file + ".part", "w") as f:
        f.write(str(server.server_address[1]))
    os.replace(port_file + ".part", port_file)
    server.serve_forever()


if __name__ == "__main__":
    if len(sys.argv) != 4:
        sys.exit(__doc__)
    main(*sys.argv[1:])
