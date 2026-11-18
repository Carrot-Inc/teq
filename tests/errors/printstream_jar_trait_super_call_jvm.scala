// jars: scala-library pstraits-lib
// teq: --target jvm --std=scala-library
// command: build
// expect: 11:7: error: not supported on the JVM yet: a trait's super call of PrintStream.printf
// expect: 12:7: error: not supported on the JVM yet: a trait's super call of PrintStream.format
// The same program built for the JVM: the back end's link mode implements a class-path trait's super accessors by
// the member under the accessor's descriptor, and the std declares the JDK's varargs printf and format with a Scala
// repeated parameter: the class it would write calls a method the JDK's class does not have, so the build is
// refused (scalac refuses the classes themselves).
import java.io.{ByteArrayOutputStream, OutputStream, PrintStream}
class Loud(o: OutputStream) extends PrintStream(o) with pstraits.Shouting
class Curious(o: OutputStream) extends PrintStream(o) with pstraits.Asking
object Probe:
  def main(args: Array[String]): Unit =
    val b = new ByteArrayOutputStream
    new Loud(b).shout("a")
    new Curious(b).format("%d", 1)
    println(b.toString("UTF-8"))
