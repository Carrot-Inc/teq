//> using platform jvm
//> using file ../../tools/script
// os: unix
// The scripts' library (tools/script) against scalac's run of the same source: JSON read and written
// as Python's `json.dumps` writes it; arguments taken apart; a command run with each redirect, its
// failure thrown, its deadline killing it with its child; a pool of twelve children four at a time,
// one past its deadline; frames whose bytes arrive in two reads; `Script.run`'s handlers. The files
// are under target/ of the working directory, removed at the end.
import java.nio.file.{Files, Paths}

object Main:
  def main(argv: Array[String]): Unit = Script.run {
    val dir = Paths.get("target/script_library")
    Sh.remove(dir)
    Files.createDirectories(dir)
    Script.atExit(println("the second handler runs first"))
    Script.atExit(Sh.remove(dir))
    // JSON
    val text = """{"name": "teq", "list": [1, -2.5, 3e2, true, null, "é\t\u0001"], "nested": {"empty": {}, "none": []}}"""
    val j = Json.parse(text)
    println(Json.write(j))
    println(Json.write(j, indent = 2))
    println(Json.write(j, ascii = false, sortKeys = true))
    println(Json.write(j("list")(5)) + " " + j("name").str + " " + j("list")(1).double + " " + j("nested")("empty").members.length + " " + j("absent").isNull)
    println(List(0.1, 1.0, 1e-5, 0.0001, 123456789012345678.0, 1e16, 9999999999999998.0, -0.0, 1.5e300, 3.141592653589793).map(d => Json.write(Json.num(d))).mkString(" "))
    println(Json.write(Json.obj("a" -> Json.num(1), "b" -> Json.arr(Json.str("x"), Json.Null)).updated("a", Json.bool(false)).updated("c", Json.str("ü"))))
    for bad <- List("{", "[1,]", "\"open", "{\"a\" 1}", "01x", "[1] 2") do
      println("bad " + bad + ": " + (try Json.write(Json.parse(bad)) catch case e: Json.ParseError => e.getMessage))
    // Arguments
    val args = new Args(Seq("--jobs", "4", "first", "--update", "--teq=/x/teq", "second"), "t [--jobs n] [--update] a b")
    println(List(args.option("jobs"), args.flag("update"), args.flag("record"), args.teq("default"), args.positionals).mkString(" "))
    // Commands
    val out = dir.resolve("out.txt")
    Sh("sh", "-c", "printf 'one\\n'; printf 'to err\\n' >&2").stdout(out).run()
    Sh("sh", "-c", "printf 'two\\n'").stdout(out, append = true).run()
    println("appended: " + Files.readString(out).replace("\n", "|"))
    val merged = Sh("sh", "-c", "printf 'a\\n'; printf 'b\\n' >&2; printf 'c\\n'").mergeErr.run()
    println("merged: " + merged.out.replace("\n", "|"))
    println("stdin text: " + Sh("cat").stdin("given\ntext").run().out)
    println("stdin file: " + Sh("cat").stdinFrom(out).run().lines.mkString("|"))
    println("stdin closed: " + Sh("sh", "-c", "cat; echo end").run().out.trim)
    println("before the inheriting command")
    Sh("sh", "-c", "echo from the command").inherit.run()
    println("after it")
    println("dir: " + Sh("sh", "-c", "pwd").in(dir).run().out.trim.endsWith("target/script_library"))
    println("env: " + Sh("sh", "-c", "printf '%s|%s' \"$TEQ_LIB_X\" \"${HOME-unset}\"").withEnv("TEQ_LIB_X" -> "x é").without("HOME").run().out)
    val failed = try Sh("sh", "-c", "echo out; echo the reason >&2; exit 3").run().status catch case e: Sh.Failed => e.getMessage + " / " + e.result.status
    println("failed: " + failed)
    println("unchecked: " + Sh("sh", "-c", "exit 4").check(false).run().status)
    val slow = Sh("sh", "-c", "sleep 30 & echo $!; wait").timeout(0.3).check(false).run()
    val grandchild = slow.out.trim.toLong
    // Killed, the grandchild is gone once the system has reaped it.
    val gone = (1 to 500).exists { _ =>
      val alive = ProcessHandle.of(grandchild).map(h => h.isAlive()).orElse(false)
      if alive then Thread.sleep(10)
      !alive
    }
    println("deadline: " + slow.status + " " + slow.timedOut + " grandchild ended " + gone)
    println("missing: " + (try Sh("no-such-program-of-teq").run() catch case e: java.io.IOException => e.getMessage))
    // A pool
    val ends = scala.collection.mutable.ArrayBuffer.empty[String]
    var most = 0
    var running = 0
    Sh.pool(4, 1 to 12) { i =>
      running += 1
      most = most.max(running)
      Sh.Then(Sh("sh", "-c", s"sleep 0.0$i; echo $i").timeout(if i == 7 then 0.02 else 20), r => {
        running -= 1
        ends += s"$i:${r.status}:${r.out.trim}"
        Sh.End
      })
    }
    println("pool: " + ends.sortBy(_.takeWhile(_ != ':').toInt).mkString(" ") + " at most " + most)
    // A job of two commands, the second chosen by the first's output.
    val chain = scala.collection.mutable.ArrayBuffer.empty[String]
    Sh.pool(2, List("x", "y")) { name =>
      Sh.Then(Sh("echo", name), r => Sh.Then(Sh("echo", r.out.trim + "!"), r2 => { chain += r2.out.trim; Sh.End }))
    }
    println("chain: " + chain.sorted.mkString(" "))
    // Frames split across reads, counted in bytes.
    val p = Sh("sh", "-c", "printf 'Content-Length: 8\\r\\n\\r\\n{\"\\303'; sleep 0.3; printf '\\251\":1}Content-Length: 2\\r\\n\\r\\n[]'").start()
    val frames = new Sh.Frames(p)
    println("frame: " + frames.next(10) + " " + frames.next(10) + " " + frames.next(1))
    println("the body ends")
  }
