// A conversion is credited to the import of the object it was read on.
import scala.language.implicitConversions
trait Base {
  given cv: Conversion[Int, String] = _.toString
}
object A extends Base
object B extends Base
object Use {
  import A.cv
  val keep = cv
  def f: String = {
    import B.given
    2
  }
}
