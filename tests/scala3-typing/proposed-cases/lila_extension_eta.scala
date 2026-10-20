// lila: Icon.scala:19 (`Iso.string(apply, value)`), ModTimeline.scala:31 (`Ordering.by(at)`),
//       LangPicker.scala:20 (`.flatMap(toLang)`, an exported extension)
// An extension method named alone, unapplied, is a method reference and eta-expands to a function.
object Icon:
  extension (icon: String) def value: Int = icon.length
  val byValue: String => Int = value

object Timeline:
  extension (e: String) def at: Int = e.length

object Exts:
  extension (s: String) def toLang: Option[String] = Option.when(s.nonEmpty)(s.toUpperCase)
export Exts.*

class User:
  import Timeline.*
  def sorted(xs: List[String]) = xs.sorted(using Ordering.by(at))
  def langs(xs: List[String]) = xs.map(Option(_)).flatMap(_.flatMap(toLang))

@main def run() =
  println(Icon.byValue("abc"))
  println(User().sorted(List("ccc", "a", "bb")))
  println(User().langs(List("en", "")))
