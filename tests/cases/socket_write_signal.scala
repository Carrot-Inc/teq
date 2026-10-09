//> using platform jvm
// os: unix
// A write the peer does not take sees a signal: tests/run_interp.sh runs this with a directory,
// waits for the file naming the process, lets the send buffer fill, sends SIGTERM, and expects the
// JDK's status 143 and the shutdown hook's line. Run without one it says so.
import java.net.*
import java.nio.file.{Files, Paths}

object Main:
  def main(args: Array[String]): Unit =
    if args.isEmpty then println("run by tests/run_interp.sh with a directory")
    else
      Runtime.getRuntime.addShutdownHook(new Thread(() => println("hook")))
      val server = new ServerSocket(0)
      val writer = new Socket("127.0.0.1", server.getLocalPort())
      val reader = server.accept()
      val data = new Array[Byte](8192)
      Files.writeString(Paths.get(args(0)).resolve("pid"), ProcessHandle.current().pid().toString)
      val out = writer.getOutputStream
      while true do out.write(data)
