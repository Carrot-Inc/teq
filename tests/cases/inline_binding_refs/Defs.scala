// The binding of an inline call's argument that `Inliner.dropUnusedDefs` keeps although the
// expansion never reads it: its count (`countRefs`, `updateTermRefCounts`) meets the binding's path
// in a type tree of the expansion. The mirror is the companion of a case class of no initialisers,
// an elideable argument, so its binding stays only where something refers to it: a val's written
// type, a call's type arguments, an ascription (nested too), a `new`, a refinement, a match type, a
// union's operand, a singleton, a nested inline method's type parameter, a local def's result or
// by-name parameter, a local type alias, a type annotation's argument (a path or a call over one),
// a var, a lambda's parameter, the result type a method no transparent one ascribes, a cast and a
// type test, a lambda's parameter its expected type gave. A type the typer inferred, normalized as
// the expansion's terms reduce `Tuple.Size[m.MirroredElemTypes]` (a val's, a call's type arguments,
// a pattern's binders, a written refinement's members), keeps none; a declaration's annotations, a
// transparent method's result and a stable argument drop it too. tests/support/inline_bindings
// lists, per method of `Use`, the companion's loads against scalac's class files.
package ib
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
    List.empty[m.MirroredElemTypes]
    1
  inline def ascribed[A](using m: Mirror.ProductOf[A]): Int =
    val y: Any = (None: Option[m.MirroredElemTypes])
    1
  inline def layered[A](using m: Mirror.ProductOf[A]): Int =
    val y: Any = ((None: Option[m.MirroredElemTypes]): Any)
    1
  inline def fresh[A](using m: Mirror.ProductOf[A]): Int =
    new Box[m.MirroredElemTypes]
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
    None.asInstanceOf[Option[m.MirroredElemTypes]]
    1
  inline def test[A](using m: Mirror.ProductOf[A]): Int =
    if (None: Any).isInstanceOf[Option[m.MirroredElemTypes] @unchecked] then 1 else 2
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
  inline def lambdaWritten[A](using m: Mirror.ProductOf[A]): Int =
    val f = (x: Tuple.Size[m.MirroredElemTypes]) => 1
    1
  inline def lambdaInferred[A](using m: Mirror.ProductOf[A]): Int =
    List(1).map(x => constValue[Tuple.Size[m.MirroredElemTypes]]).head
  inline def mapIdentity[A](using m: Mirror.ProductOf[A]): Int =
    List(constValue[Tuple.Size[m.MirroredElemTypes]]).map(x => x).head
  inline def valInferredList[A](using m: Mirror.ProductOf[A]): Int =
    val xs = List(constValue[Tuple.Size[m.MirroredElemTypes]])
    1
  inline def casePat[A](using m: Mirror.ProductOf[A]): Int =
    Option(constValue[Tuple.Size[m.MirroredElemTypes]]) match
      case Some(x) => x
      case None => 0
  inline def unionVal[A](using m: Mirror.ProductOf[A]): Int =
    val x: Tuple.Size[m.MirroredElemTypes] | String = "a"
    1
  inline def unionAscribed[A](using m: Mirror.ProductOf[A]): Int =
    val y: Any = ("a": Tuple.Size[m.MirroredElemTypes] | String)
    1
  inline def unionTarg[A](using m: Mirror.ProductOf[A]): Int =
    val y: Any = List.empty[Tuple.Size[m.MirroredElemTypes] | String]
    1
  inline def aliasMatch[A](using m: Mirror.ProductOf[A]): Int =
    type T = Tuple.Size[m.MirroredElemTypes]
    1
  inline def annotationCall[A](using m: Mirror.ProductOf[A]): Int =
    val x: Int @Ann(List(m)) = 1
    x
  inline def annotationSelect[A](using m: Mirror.ProductOf[A]): Int =
    val x: Int @Ann(Some(m).isEmpty) = 1
    x
  inline def nothing[A](using m: Mirror.ProductOf[A]): Int = 1
