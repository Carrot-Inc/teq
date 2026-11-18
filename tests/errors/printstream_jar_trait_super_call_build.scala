// jars: pstraits-lib
// command: build
// expect: 9:7: error: Member method printf of mixin trait Shouting is missing a concrete super implementation in class Loud.
// expect: 10:7: error: Member method format of mixin trait Asking is missing a concrete super implementation in class Curious.
// A scalac-built library's traits reach java.io.PrintStream's printf and format through super (tests/support/
// printstream_traits). scalac refuses a class that mixes one in (ResolveSuper: the JDK's members take an array, the
// traits' super accessors a Seq); the JavaScript build reads the traits' bodies and says scalac's lines.
import java.io.{ByteArrayOutputStream, OutputStream, PrintStream}
class Loud(o: OutputStream) extends PrintStream(o) with pstraits.Shouting
class Curious(o: OutputStream) extends PrintStream(o) with pstraits.Asking
object Probe:
  def main(args: Array[String]): Unit =
    val b = new ByteArrayOutputStream
    new Loud(b).shout("a")
    new Curious(b).format("%d", 1)
    println(b.toString("UTF-8"))
