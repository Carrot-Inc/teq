import scala.annotation.nowarn
trait A; trait B
@nowarn("id=E003") object P { type T = A with B }
object Q { type U = A with B }
