import scala.annotation.targetName

// An overloaded value used as a qualifier keeps the alternatives that have the member selected, taking
// its arguments (dotty's `SelectionProto.isMatchedBy`), as tyrian's attributes of one name and
// several types are written `width := 5` and `style := Style(..)`: an operator, a selection with
// arguments and one without, a member several alternatives have, and one only an alternative has.
final class NameString(name: String):
  def :=(value: String): String = s"$name=$value"
  def describe: String = s"$name as text"
final class NameInt(name: String):
  def :=(value: Int): String = s"$name:$value"
  def :=(value: Long): String = s"$name:${value}L"
final class NameStyle(name: String):
  def :=(value: List[String]): String = s"$name;${value.mkString(",")}"
  def set(values: String*): String = s"$name;${values.mkString("+")}"

trait Attrs:
  @targetName("width-String") val width: NameString = NameString("width")
  @targetName("width-Int") val width: NameInt = NameInt("width")
  @targetName("style-String") val style: NameString = NameString("style")
  @targetName("style-Style") val style: NameStyle = NameStyle("style")
object Html extends Attrs

@main def run(): Unit =
  import Html.*
  println(width := 5)
  println(width := 6L)
  println(width := "5px")
  println(width.:=(7))
  println(style := List("a", "b"))
  println(style.set("x", "y"))
  println(List(style := "color", width := 3))
  println(Html.width := "8em")
