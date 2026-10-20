// An import's named selector may name a member every type has, `Any`'s and `AnyRef`'s
// (dotty's `Checking.checkImportSelectors`: `qualType.member(name)` finds them through the
// bases), of an object or of a class's value, renamed or not; the deprecated `Predef` members
// scala-library keeps may be unimported (`any2stringadd`, `$conforms`).
import Predef.{any2stringadd as _, $conforms as _, *}

object Foo:
  def bar = 1
  override def toString = "Foo!"

class Box(val n: Int):
  override def hashCode = n * 7

object Main:
  def main(args: Array[String]): Unit =
    import Foo.{bar as quux, toString as fooText}
    println(quux)
    println(fooText)
    val b = new Box(6)
    import b.{hashCode as boxHash, n as size}
    println(boxHash)
    println(size)
