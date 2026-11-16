// A library compiled by scalac whose bodies create classes with hidden constructors from inside their boundary
// (tests/classpath/js/constructor_access_jar_bodies*): a body read from a jar is asked nothing of them.
package paper
import scala.annotation.publicInBinary
class Wax @publicInBinary private[paper] (val n: Int)
class Seal private[paper] (val n: Int):
  private def this(s: String) = this(s.length)
  def twice: Int = n * 2
object Seal:
  def of(s: String): Seal = new Seal(s)
class Guarded protected (val n: Int):
  protected def this() = this(7)
class Mill(val k: Int):
  def stamp(n: Int): Int = new Seal(n + k).twice
  def stampOf(s: String): Int = Seal.of(s).twice
  def kept(): Int = (new Guarded(3) {}).n
  def keptDefault(): Int = (new Guarded() {}).n
  inline def inlineWax(n: Int): Int = new Wax(n).n
  def lambda(): Int = List(1, 2).map(i => new Seal(i).n).sum
  class Inner:
    def stamp(n: Int): Int = new Seal(n).n
object Mill:
  def default: Mill = new Mill(0)
// Generic classes a program names bare: one with a public constructor beside a protected one, one whose one
// constructor is protected (scalac writes both public in the class file: no class of the JVM guards them).
class Tray[T]():
  protected def this(n: Int) = this()
  def v: Int = 21
class Rack[T] protected ():
  def v: Int = 22
