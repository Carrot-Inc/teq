//> using platform jvm
//> using file ../../tools/script
// The scripts' library speaking JSON-RPC with a resident child: `teq lsp` (TEQ names it; the
// suites set it) started, initialized, shut down and ended, its replies read as frames counted in
// bytes, within deadlines. What is printed holds for any version of the server.
object Main:
  def main(argv: Array[String]): Unit = Script.run {
    val teq = new Args(argv.toSeq, "script_lsp [--teq teq]").teq("target/release/teq")
    val lsp = Sh(teq, "lsp").start()
    val frames = new Sh.Frames(lsp)
    frames.send("""{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"processId": null, "rootUri": null, "capabilities": {}}}""")
    val init = frames.next(30).map(Json.parse).getOrElse(Json.Null)
    println("initialize: " + init("id").int + " " + init("result")("serverInfo")("name").str + " " + !init("result")("capabilities").isNull)
    frames.send("""{"jsonrpc": "2.0", "id": 2, "method": "shutdown", "params": null}""")
    println("shutdown: " + frames.next(30).map(b => Json.write(Json.parse(b)("result"))))
    frames.send("""{"jsonrpc": "2.0", "method": "exit", "params": null}""")
    println("exit: " + lsp.waitFor(30))
    println("no more frames: " + frames.next(1))
  }
