// Inline methods whose mirror argument is bound and never read, upstream of their calls, where
// the binding's path stands only in a type annotation's argument, as a path or inside a call
// (`Int @Ann(List(m))`): dotty's `Annotated` tree keeps the annotation's tree whole, whose
// `RefTree`s `Inliner.dropUnusedDefs` counts; the pickle writes it whole and the reader reads
// it whole, so a build over the products keeps the binding the whole build keeps.
package aaa
import scala.deriving.Mirror
final case class C(x: Int)
object C { def noop = 1 }
class Ann(x: Any) extends scala.annotation.StaticAnnotation
object Defs:
  inline def annotationPath[A](using m: Mirror.ProductOf[A]): Int =
    val x: Int @Ann(m) = 1
    x
  inline def annotationCall[A](using m: Mirror.ProductOf[A]): Int =
    val x: Int @Ann(List(m)) = 1
    x
  inline def annotationSelect[A](using m: Mirror.ProductOf[A]): Int =
    val x: Int @Ann(Some(m).isEmpty) = 1
    x
