//> using platform jvm
// os: unix
// A reader over a child's pipe (the interpreter's own stream, read and decoded natively) decides a
// UTF-8 sequence the input ends inside as the JDK's decoder does: a lone c0 is replaced as soon as it
// comes, while the child still runs, and a prefix that more bytes complete waits for them.
import java.io.InputStreamReader

object Main:
  def main(args: Array[String]): Unit =
    val child = new ProcessBuilder("sh", "-c", "printf '\\300'; sleep 3; printf '\\342\\202'; sleep 1; printf '\\254'").start()
    val reader = new InputStreamReader(child.getInputStream())
    val start = System.nanoTime()
    val first = reader.read()
    val waited = (System.nanoTime() - start) / 1000000
    println(s"$first ${if waited < 2000 then "at once" else s"after $waited ms"}")
    println(s"${reader.read()} ${reader.read()}")
    child.waitFor()
