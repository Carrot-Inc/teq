import java.time.ZoneId
import scala.quoted.*

// An object whose initialiser keeps the machine's zone, as reactivemongo's default handlers of
// `java.time` values do, read by a macro for its other values: the macro reads the zone as the
// build's JVM has it, and on the JVM target the JDK's `ZoneId.systemDefault` has no body.
object Handlers:
  val zone: ZoneId = ZoneId.systemDefault()
  val names: List[String] = List("date", "time", "zone")

object Macros:
  inline def count: Int = ${ countImpl }
  def countImpl(using Quotes): Expr[Int] = Expr(Handlers.names.size)
  inline def sameZone: Boolean = ${ sameZoneImpl }
  def sameZoneImpl(using Quotes): Expr[Boolean] = Expr(Handlers.zone.getId == ZoneId.systemDefault().getId)
