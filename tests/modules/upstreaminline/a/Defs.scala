// Inline methods whose mirror argument is bound and never read, upstream of their calls:
// `Inliner.dropUnusedDefs` keeps the binding where a type tree of the expansion names it,
// written (a val's type, an ascription, a refinement, a match type, a local type alias, a
// type annotation's argument) or inferred, and drops it where none does or an inferred one
// reduces. The downstream reads these bodies from the products, whose pickle keeps a written
// type tree apart from an inferred one (`TreePickler.pickleTpt`), so that its bindings are the
// whole build's.
package uia
import scala.deriving.Mirror
import scala.compiletime.*
final case class C(x: Int)
object C { def noop = 1 }
class Box[A]
class TAnn[A] extends scala.annotation.StaticAnnotation
class Ann(x: Any) extends scala.annotation.StaticAnnotation
object Defs:
  inline def written[A](using m: Mirror.ProductOf[A]): Int =
    val x: Option[m.MirroredElemTypes] = None
    1
  inline def inferred[A](using m: Mirror.ProductOf[A]): Int =
    val x = constValue[Tuple.Size[m.MirroredElemTypes]]
    x
  inline def inferredCall[A](using m: Mirror.ProductOf[A]): Int =
    val x = List.empty[m.MirroredElemTypes]
    1
  inline def tapply[A](using m: Mirror.ProductOf[A]): Int =
    val y: Any = List.empty[m.MirroredElemTypes]
    1
  inline def ascribed[A](using m: Mirror.ProductOf[A]): Int =
    val y: Any = (None: Option[m.MirroredElemTypes])
    1
  inline def layered[A](using m: Mirror.ProductOf[A]): Int =
    val y: Any = ((None: Option[m.MirroredElemTypes]): Any)
    1
  inline def fresh[A](using m: Mirror.ProductOf[A]): Int =
    val y: Any = new Box[m.MirroredElemTypes]
    1
  inline def refined[A](using m: Mirror.ProductOf[A]): Int =
    val x: Option[AnyRef { type T = m.MirroredElemTypes }] = None
    1
  inline def matched[A](using m: Mirror.ProductOf[A]): Int =
    val x: Tuple.Size[m.MirroredElemTypes] = constValue[Tuple.Size[m.MirroredElemTypes]]
    1
  inline def singleton[A](using m: Mirror.ProductOf[A]): Int =
    val x: Option[m.type] = None
    1
  inline def inner[T]: Int =
    val x: Option[T] = None
    1
  inline def nested[A](using m: Mirror.ProductOf[A]): Int = inner[m.MirroredElemTypes]
  inline def innerCall[T]: Int =
    List.empty[T]
    1
  inline def nestedCall[A](using m: Mirror.ProductOf[A]): Int = innerCall[m.MirroredElemTypes]
  inline def localdef[A](using m: Mirror.ProductOf[A]): Int =
    def local: Option[m.MirroredElemTypes] = None
    local
    1
  inline def byname[A](using m: Mirror.ProductOf[A]): Int =
    def local(x: => Option[m.MirroredElemTypes]): Int = 1
    local(None)
  inline def alias[A](using m: Mirror.ProductOf[A]): Int =
    type T = m.MirroredElemTypes
    1
  inline def typeAnnotated[A](using m: Mirror.ProductOf[A]): Int =
    @TAnn[m.MirroredElemTypes] val x = 1
    x
  inline def termAnnotated[A](using m: Mirror.ProductOf[A]): Int =
    @Ann(m) val x = 1
    x
  inline def annotatedType[A](using m: Mirror.ProductOf[A]): Int =
    val x: Int @Ann(m) = 1
    x
  inline def variable[A](using m: Mirror.ProductOf[A]): Int =
    var x: Option[m.MirroredElemTypes] = None
    x = None
    1
  inline def lambda[A](using m: Mirror.ProductOf[A]): Int =
    val f = (x: Option[m.MirroredElemTypes]) => 1
    f(None)
  inline def returned[A](using m: Mirror.ProductOf[A]): Option[m.MirroredElemTypes] = None
  inline def returnedInferred[A](using m: Mirror.ProductOf[A]) = List.empty[m.MirroredElemTypes]
  transparent inline def returnedTransparent[A](using m: Mirror.ProductOf[A]): Option[m.MirroredElemTypes] = None
  inline def dependent[A](using m: Mirror.ProductOf[A])(x: Option[m.MirroredElemTypes]): Int = 1
  inline def cast[A](using m: Mirror.ProductOf[A]): Int =
    val y: Any = None.asInstanceOf[Option[m.MirroredElemTypes]]
    1
  inline def nothing[A](using m: Mirror.ProductOf[A]): Int = 1
