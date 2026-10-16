// Inline methods whose mirror argument is bound and never read, upstream of their calls, where
// the binding's path stands only in a written tree whose type the expansion reduces: a union's
// operand (`Tuple.Size[m.MirroredElemTypes] | String`), an intersection's, a local alias's
// right-hand side, as a val's type, an ascription and a type argument. dotty's typed tree keeps
// the written children (`scala.|` applied to them), whose `RefTree` `Inliner.dropUnusedDefs`
// counts; the pickle keeps them as trees (`TreePickler.pickleTpt`), so a build over the products
// keeps the binding the whole build keeps.
package wua
import scala.deriving.Mirror
import scala.compiletime.*
final case class C(x: Int)
object C { def noop = 1 }
object Defs:
  inline def unionVal[A](using m: Mirror.ProductOf[A]): Int =
    val x: Tuple.Size[m.MirroredElemTypes] | String = "a"
    1
  inline def unionAscribed[A](using m: Mirror.ProductOf[A]): Int =
    val y: Any = ("a": Tuple.Size[m.MirroredElemTypes] | String)
    1
  inline def unionTarg[A](using m: Mirror.ProductOf[A]): Int =
    val y: Any = List.empty[Tuple.Size[m.MirroredElemTypes] | String]
    1
  inline def interVal[A](using m: Mirror.ProductOf[A]): Int =
    val x: Tuple.Size[m.MirroredElemTypes] & Any = constValue[Tuple.Size[m.MirroredElemTypes]]
    1
  inline def aliasMatch[A](using m: Mirror.ProductOf[A]): Int =
    type T = Tuple.Size[m.MirroredElemTypes]
    1
