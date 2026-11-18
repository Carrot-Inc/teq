// expect: 14:16: error: not supported yet: an override of PrintStream.printf on the JVM
// expect: 16:16: error: not supported yet: an override of PrintStream.format on the JVM
// expect: 18:16: error: not supported yet: an override of PrintStream.format on the JVM
// expect: 20:81: error: not supported yet: an override of PrintStream.printf on the JVM
// expect: 4 errors found
// teq: --target jvm
// On the JVM the std's java.io.PrintStream stands for the JDK's class, whose printf and format take an array where
// the std's take a sequence: an override would be a method the JDK's class never calls, so it is refused: in a
// direct subclass, in a class below one, in an anonymous class. An overload of the name and the other members
// are overridden freely.
import java.io.*
class Direct(o: OutputStream) extends PrintStream(o):
  override def print(s: String): Unit = super.print("[" + s + "]")
  override def printf(f: String, a: Any*): PrintStream = this
class Middle(o: OutputStream) extends PrintStream(o):
  override def format(f: String, a: Any*): PrintStream = this
class Below(o: OutputStream) extends Direct(o):
  override def format(f: String, a: Any*): PrintStream = this
  def printf(n: Int): PrintStream = this
def anonymous(o: OutputStream): PrintStream = new PrintStream(o) { override def printf(f: String, a: Any*): PrintStream = this }
