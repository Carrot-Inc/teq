// lila: `import lila.db.dsl.{*, given}` then `import toBSONValueOption.given`,
// where toBSONValueOption is an object member of trait dsl, inherited by object dsl.
import scala.language.implicitConversions

trait BSONValue
case class BSONString(s: String) extends BSONValue
trait BSONWriter[T]:
  def writeOpt(t: T): Option[BSONValue]
given BSONWriter[String] = s => Some(BSONString(s))

trait dsl:
  def setsAndUnsets(items: (String, Option[BSONValue])*): String =
    items.map((k, v) => s"$k=${v.getOrElse("unset")}").mkString(", ")
  object toBSONValueOption:
    given [V](using w: BSONWriter[V]): Conversion[Option[V], Option[BSONValue]] =
      _.flatMap(w.writeOpt)
object dsl extends dsl

import dsl.*

@main def run() =
  import toBSONValueOption.given
  val name: Option[String] = Some("x")
  val note: Option[String] = None
  println(setsAndUnsets("name" -> name, "note" -> note))
