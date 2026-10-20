import scala.compiletime.summonInline
import X.given
object A:
  val early: Int = scala.compiletime.error("A_EARLY")
  inline def fetch: Int = summonInline[Box].v
  val first: Int = fetch
