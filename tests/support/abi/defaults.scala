// Item 2 of the JVM ABI alignment: a default's getter takes the
// parameters of the clauses before the default's (none for the first clause's), a by-name parameter's returns the
// value; a constructor's getters are its companion's instance methods, with static forwarders in a top-level
// class, the companion written by the backend for a plain class without one (`Plain$`).
// A by-name constructor parameter's getter returns the value.
// abi: classes $lessinit$greater$default$1 $lessinit$greater$default$2 scaled$default$2 curried$default$2 curried$default$3 f$default$1 f$default$2 ext$default$2
package defaults
class Plain(val a: Int = 1)(val b: String = a.toString)
case class CC(a: Int = 1)(b: String = a.toString)
class WithObject(x: Int, y: Int = 2)
object WithObject:
  def make = new WithObject(1)
class Sec(val a: Int, val b: Int):
  def this(a: Int, s: String = "7") = this(a, s.toInt)
enum Level(val n: Int = 3):
  case Low(x: Int) extends Level(x)
  case High(y: Int) extends Level()
object O:
  def scaled(x: Int, y: Int = 2): Int = x * y
  def curried[A](a: A)(b: String = a.toString)(c: Int = b.length): Int = c
  def f(x: Int = 1, y: => Int = 2): Int = x + y + y
  extension (s: String) def ext(n: Int = 3): String = s * n
class D:
  def scaled(x: Int, y: Int = 2): Int = x * y
trait T:
  def scaled(x: Int, y: Int = 2): Int = x * y
class ByName(x: => Int = 7)
case class ByNameCase(y: Int)(z: => Int = y + 1)
