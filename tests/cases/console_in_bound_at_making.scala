// Console binds its `in` as it is made (scala.Console's `inVar`, a reader of System.in as it stands then): a System.setIn after
// Console's first use leaves StdIn reading the stream of that use. Not yet on JavaScript nor in the interpreter: the std has no
// System.setIn, and under --std scala-library Console's `inVar` is bound at its first read (scala-library's `outVar` and `errVar`
// are bound as Console is made by a named rule, src/typer/check.rs, `inVar` not). The expected lines are scalac's on the JDK.
import java.io.ByteArrayInputStream
object InBoundAtMaking:
  def main(args: Array[String]): Unit =
    val first = System.in
    System.setIn(new ByteArrayInputStream("zero\n".getBytes("UTF-8")))
    println("made")
    System.setIn(new ByteArrayInputStream("one\n".getBytes("UTF-8")))
    println(scala.io.StdIn.readLine())
    System.setIn(first)
