// jars: pstraits-lib
// expect: 9:7: error: Member method printf of mixin trait Shouting is missing a concrete super implementation in class Loud.
// expect: 10:7: error: Member method format of mixin trait Asking is missing a concrete super implementation in class Curious.
// expect: 2 errors found
// The same program checked, not built: scalac's lines. teq's check binds a trait's super calls from a body it read,
// and reads none of a class-path trait's before a build's reach: it accepts the program today, and so does the
// interpreter, which runs the plain meaning (`a![Asking]1`).
import java.io.{ByteArrayOutputStream, OutputStream, PrintStream}
class Loud(o: OutputStream) extends PrintStream(o) with pstraits.Shouting
class Curious(o: OutputStream) extends PrintStream(o) with pstraits.Asking
object Probe:
  def main(args: Array[String]): Unit =
    val b = new ByteArrayOutputStream
    new Loud(b).shout("a")
    new Curious(b).format("%d", 1)
    println(b.toString("UTF-8"))
