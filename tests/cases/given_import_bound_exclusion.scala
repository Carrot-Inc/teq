// What a wildcard import brings to the search (`ImportInfo.importedImplicits`): through `given T` only the
// givens that conform to `T` (`givenBound`), none whose name another selector of the clause hides or renames
// (`excluded`; a renamed one comes under its new name), and through `*` a Scala 2 implicit on those terms too;
// an inline method's search at the call site sees the site's imports so, and the body's own imports likewise.
import scala.compiletime.summonFrom
object A:
  given Int = 1
  given String = "leaked"
  given intList: List[Int] = List(1)
  given strList: List[String] = List("s")
object G:
  implicit val hidden: Int = 7
  implicit val shown: String = "shown"
inline def str: String = summonFrom { case s: String => s; case _ => "none" }
inline def int: String = summonFrom { case i: Int => i.toString; case _ => "none" }
inline def fromBody: String =
  import G.{hidden as _, *}
  int + "," + str
@main def run(): Unit =
  locally { import A.{given Int}; println(summon[Int]); println(str) }
  locally { import A.{given List[?]}; println(summon[List[Int]]); println(summon[List[String]]); println(int + "," + str) }
  locally { import A.{given_String as _, given}; println(str); println(summon[Int]) }
  locally { import A.{given_String as renamed, given}; println(str) }
  locally { import A.{given Int, given String}; println(int + "," + str) }
  locally { import G.{hidden as _, *}; println(int + "," + str) }
  locally { import G.{shown as _, *}; println(int + "," + str) }
  println(fromBody)
