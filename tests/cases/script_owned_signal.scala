//> using platform jvm
// os: unix
//> using file ../../tools/script
// The child of a blocking `Sh.run` dies with the script at a SIGTERM: tests/run_interp.sh runs this
// with a directory, waits for the file naming the child, sends SIGTERM to the script, and expects
// the JDK's status 143, the handlers' lines, the last registered first, and the child gone. Run
// without one it says so.
import java.nio.file.{Files, Paths}

object Main:
  def main(args: Array[String]): Unit = Script.run {
    if args.isEmpty then println("run by tests/run_interp.sh with a directory")
    else
      val dir = Paths.get(args(0))
      Script.atExit(println("cleanup-first"))
      Script.atExit(println("cleanup-second"))
      Files.writeString(dir.resolve("pid"), ProcessHandle.current().pid().toString)
      Sh("sh", "-c", "echo $$ > \"$0/child\"; exec sleep 30", dir.toString).inherit.run()
      println("not reached")
  }
