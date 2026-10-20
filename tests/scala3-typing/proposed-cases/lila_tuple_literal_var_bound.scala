// lila: `Json.obj(tags.value.map(t => (t.name.name, t.value))*)` where Json.obj takes
// `(String, JsValueWrapper)*` and a String becomes a JsValueWrapper by an implicit conversion.
// The lambda's result is map's B, bounded by (String, Wrapper); scalac types the tuple literal
// as Tuple2.apply against that bound, so each element is adapted. teq leaves the elements as
// they are when the tuple's expected type is a type variable.
import scala.language.implicitConversions

class Wrapper(val json: String)
object Wrapper:
  implicit def fromWrites[T](t: T)(implicit w: Writes[T]): Wrapper = Wrapper(w.write(t))
trait Writes[T]:
  def write(t: T): String
given Writes[String] = s => s"\"$s\""

def obj(fields: (String, Wrapper)*): String =
  fields.map((k, w) => s"\"$k\":${w.json}").mkString("{", ",", "}")

case class Tag(name: String, value: String)
val tags = List(Tag("White", "Carlsen"), Tag("Black", "Caruana"))

def widened: Option[(Int, Long)] = Option(1).map(x => (x, x))

@main def run() =
  println(obj(tags.map(t => (t.name, t.value))*))
  println(widened)
