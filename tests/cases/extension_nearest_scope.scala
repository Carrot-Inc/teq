// The extension method a selection names is the nearest binding of the name among extension
// methods (dotty's `findRef` for `tryExtension`), whatever its receiver: where it does not take the
// receiver, the enclosing scopes' are hidden and the receiver's implicit scope is searched (a
// companion's extension, a given's), not the next scope out. Imports of one scope that each bring
// one are alternatives tried alike, a named import's beating the wildcards'; a definition of the
// file's package in the file beats the file's imports. The std's stand-ins for members and for
// `Predef`'s implicit classes stay reachable. An import that brings the very extension an enclosing
// object defines is no rival to it, nor is a named import further out than one that brings the
// same extension.
class C
object C:
  extension (c: C) def tag: String = "companion"

object Outer:
  extension (c: C) def tag: String = "outer"
  extension (n: Int) def only: String = "outer only"
object Inner:
  extension (s: String) def tag: String = "inner"
  extension (s: String) def show: String = "inner show"

object A:
  extension (n: Int) def tag: String = "int"
  extension (n: Int) def wow: String = "a wow"
object B:
  extension (s: String) def tag: String = "string"
  extension (n: Int) def wow: String = "b wow"

trait Show[T]:
  extension (t: T) def show: String
given Show[Int] with
  extension (n: Int) def show: String = "given show"

extension (b: Boolean) def flag: String = "top flag"
object Flags:
  extension (n: Int) def flag: String = "imported flag"

import Outer.*
import Flags.*
import scala.util.chaining.*

@main def run(): Unit =
  import Inner.*
  println((new C).tag)
  println(1.only)
  println(1.show)
  println("s".show)
  println(true.flag)
  println(1.pipe(_ + 1))
  println("abc".length)
  locally {
    import A.*
    import B.*
    println(1.tag)
    println("s".tag)
  }
  locally {
    import A.*
    import B.wow
    println(1.wow)
  }
  println(Same.use())
  println(Barrier.run())

object Same:
  extension (x: Int) def same: String = "same"
  def use(): String =
    import Same.*
    1.same

object First:
  extension (x: Int) def barrier: String = "first"
object Second:
  extension (x: Int) def barrier: String = "second"
import First.barrier
object Barrier:
  import Second.barrier
  def run(): String =
    import Second.*
    1.barrier
