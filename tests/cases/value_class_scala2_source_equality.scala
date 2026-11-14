// In a source under `import scala.language.`2.13`` a value class's `equals` and `hashCode` are the rule's whatever
// its traits define: scalac 3.8.4 synthesizes both there as Scala 2 does (`existingDef`'s first branch).
import scala.language.`2.13`

trait Odd extends Any {
  override def equals(o: Any): Boolean = false
  override def hashCode: Int = 7
}
class Wrapped(val x: Int) extends AnyVal with Odd

trait Hashed extends Any { override def hashCode: Int = 7 }
class OnlyHash(val x: Int) extends AnyVal with Hashed

object Main {
  def main(args: Array[String]): Unit = {
    val a: Any = new Wrapped(1)
    val b: Any = new Wrapped(1)
    val c: Any = new Wrapped(2)
    println(a == b)
    println(a == c)
    println(a.hashCode)
    println(Set(new Wrapped(1), new Wrapped(1)).size)
    val h: Any = new OnlyHash(3)
    println(h == new OnlyHash(3))
    println(h.hashCode)
  }
}
