package mirror
import scala.deriving.Mirror
case class Box[A](x: A)
class Base(using val m: Mirror.ProductOf[Box[Int]])
trait T extends Base
class K extends T
