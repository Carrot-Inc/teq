// Console binds its `err` as it is made, whichever of its streams the program uses first (scala.Console's `errVar`, Console.scala
// 128-129): a System.setErr after Console's first use by its `out` leaves Console.err the stream of that use. Under
// --std scala-library this holds by a named rule (src/typer/check.rs: scala-library's Console's `outVar` and `errVar` initialise in
// its constructor), which goes when a jar object's vals initialise in its constructor; without it the moved stream answers. The
// expected lines are scalac's on the JDK.
import java.io.{ByteArrayOutputStream, PrintStream}
object ErrBoundAtMaking:
  def main(args: Array[String]): Unit =
    val first = System.err
    val moved = new PrintStream(new ByteArrayOutputStream())
    scala.Console.flush()
    System.setErr(moved)
    println("Console.err is the first stream: " + (scala.Console.err eq first) + ", the moved one: " + (scala.Console.err eq moved))
    System.setErr(first)
    println("after the restore: " + (scala.Console.err eq first))
