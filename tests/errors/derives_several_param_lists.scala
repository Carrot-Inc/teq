// expect: error: No given instance of type Mirror.ProductOf[Two] was found.
// expect: 1 error found
// A case class with several parameter lists is no generic product, as under scalac.
import scala.deriving.Mirror
trait Eq[A] { def eqv(a: A, b: A): Boolean }
object Eq:
  inline def derived[A](using m: Mirror.ProductOf[A]): Eq[A] = (a, b) => a == b
case class Two(a: Int)(val b: Int) derives Eq
