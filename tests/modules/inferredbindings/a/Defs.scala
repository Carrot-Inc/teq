// Inline methods whose mirror argument is bound and never read, upstream of their calls, where
// the only type trees that name it are ones the typer inferred, or a written refinement's
// members: dotty's expansion normalizes those (`x: (1 : Int)` for a pattern definition's binder,
// `ReTyper.typedRefinedTypeTree` making a refinement the `TypeTree` of its type), so
// `Inliner.dropUnusedDefs` finds no reference there and drops the binding; an inferred lambda
// parameter keeps its path, and a written type argument its `RefTree`. The downstream's bindings
// over the products are the whole build's.
package iia
import scala.deriving.Mirror
import scala.compiletime.*
final case class C(x: Int)
object C { def noop = 1 }
object Defs:
  inline def inferredPat[A](using m: Mirror.ProductOf[A]): Int =
    val (x, y) = (constValue[Tuple.Size[m.MirroredElemTypes]], 1)
    x + y
  inline def refinedMatch[A](using m: Mirror.ProductOf[A]): Int =
    val x: AnyRef { type T = Tuple.Size[m.MirroredElemTypes] } = null
    1
  inline def targWritten[A](using m: Mirror.ProductOf[A]): Int =
    val y: Any = List.empty[Tuple.Size[m.MirroredElemTypes]]
    1
  inline def targInferred[A](using m: Mirror.ProductOf[A]): Int =
    val y: Any = List(constValue[Tuple.Size[m.MirroredElemTypes]])
    1
  inline def lambdaInferred[A](using m: Mirror.ProductOf[A]): Int =
    List(1).map(x => constValue[Tuple.Size[m.MirroredElemTypes]]).head
  inline def mapIdentity[A](using m: Mirror.ProductOf[A]): Int =
    List(constValue[Tuple.Size[m.MirroredElemTypes]]).map(x => x).head
