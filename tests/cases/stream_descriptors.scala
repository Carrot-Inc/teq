//> using platform jvm
// os: unix
// A stream holds no descriptor once closed, and a child's pipes are closed when it ends (the JDK's
// reaper's `processExited`) whether or not the program touches them again: 400 files opened and
// closed, 400 children dropped, 400 children with their streams closed, each leaves the program's
// descriptors as they were. tests/run_interp.sh runs this again under `ulimit -n 256`.
import java.nio.file.{Files, Paths}

object Main:
  def count(): Long =
    val s = Files.list(Paths.get("/dev/fd"))
    try s.count() finally s.close()
  def main(args: Array[String]): Unit =
    for mode <- List("file", "dropped", "closed") do
      val before = count()
      var n = 0
      try
        while n < 400 do
          if mode == "file" then
            val in = Files.newInputStream(Paths.get("/dev/null"))
            in.close()
          else
            val p = new ProcessBuilder("true").start()
            p.waitFor()
            if mode == "closed" then
              p.getInputStream.close()
              p.getErrorStream.close()
              p.getOutputStream.close()
          n += 1
        // The reapers close a child's pipes once it has ended: a moment for the last ones.
        Thread.sleep(200)
        println(s"$mode: $n done, descriptors kept: ${count() - before <= 4}")
      catch case e: Exception => println(s"$mode: failed after $n: $e")
