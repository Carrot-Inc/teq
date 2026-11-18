// The twin of console_err_bound_at_making: Console's first use is by its `err`, read before the System.setErr, so Console.err is
// the stream of that use with the named rule (src/typer/check.rs) or without it. The expected lines are scalac's on the JDK.
import java.io.{ByteArrayOutputStream, PrintStream}
object ErrBoundAtFirstRead:
  def main(args: Array[String]): Unit =
    val first = System.err
    val moved = new PrintStream(new ByteArrayOutputStream())
    scala.Console.err.flush()
    System.setErr(moved)
    println("Console.err is the first stream: " + (scala.Console.err eq first) + ", the moved one: " + (scala.Console.err eq moved))
    System.setErr(first)
    println("after the restore: " + (scala.Console.err eq first))
