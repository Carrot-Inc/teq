// A method whose type parameter's bound names the trait's abstract member, exported through an object
// that fixes it: the forwarder's parameter is bounded as the path sees it (`mbr.info` through the path).
trait TagLite:
  type Top
  opaque type Tag[+N <: Top] = String
  def make[N <: Top](value: String): Tag[N] = value
object HtmlTagOf extends TagLite:
  type Top = AnyRef
object API:
  export HtmlTagOf.{Tag, make}
@main def run(): Unit =
  val t: API.Tag[String] = API.make[String]("ok")
  val u: HtmlTagOf.Tag[String] = t
  println(u)
