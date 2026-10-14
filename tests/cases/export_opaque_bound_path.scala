// An opaque type exported through an object that fixes its bound's member (`Top = AnyRef`): the
// type forwarder's bounds are seen from the path (dotty's `pathType.select`, `etaExpand`).
trait TagLite:
  type Top
  opaque type Tag[+N <: Top] = String
  def make[N <: Top](value: String): Tag[N] = value
object HtmlTagOf extends TagLite:
  type Top = AnyRef
export HtmlTagOf.{Tag as HtmlTagOf}
@main def run(): Unit =
  val t: HtmlTagOf[String] = HtmlTagOf.make[String]("ok")
  println(t)
