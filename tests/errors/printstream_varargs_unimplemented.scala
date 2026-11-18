// expect: 18:7: error: class Direct needs to be abstract, since def printf(f: String, a: Any*): PrintStream in trait F is not defined
// expect: 19:7: error: class Generic needs to be abstract, since def format(f: String, a: Any*): PrintStream in trait G is not defined
// expect: 20:7: error: class Wider needs to be abstract, since def printf(f: String, a: Any*): Any in trait H is not defined
// expect: 22:7: error: class Below needs to be abstract, since def printf(f: String, a: Any*): PrintStream in trait F is not defined
// expect: 23:8: error: object creation impossible, since def printf(f: String, a: Any*): PrintStream in trait F is not defined
// expect: 5 errors found
// The JDK's PrintStream.printf and format take a Java varargs array, which implements no Scala member of a
// repeated parameter (after ElimRepeated one is an array, the other a Seq): a deferred `printf(f: String, a: Any*)`
// of a trait stays unimplemented beside the stream's, as scalac's RefChecks says. In a class, under an abstract
// class, with a wider result, from a generic trait, in an object. Implemented by the class itself it is accepted.
import java.io.*
trait F:
  def printf(f: String, a: Any*): PrintStream
trait G[R]:
  def format(f: String, a: Any*): R
trait H:
  def printf(f: String, a: Any*): Any
class Direct(o: OutputStream) extends PrintStream(o) with F
class Generic(o: OutputStream) extends PrintStream(o) with G[PrintStream]
class Wider(o: OutputStream) extends PrintStream(o) with H
abstract class Middle(o: OutputStream) extends PrintStream(o) with F
class Below(o: OutputStream) extends Middle(o)
object Single extends PrintStream(new ByteArrayOutputStream) with F
