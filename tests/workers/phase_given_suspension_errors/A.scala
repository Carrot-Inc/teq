import scala.compiletime.summonInline
import X.given
object A:
  inline def fetch: Int = summonInline[Box].v
  val first: Int = fetch
  val late: Int = scala.compiletime.error("A_LATE")
