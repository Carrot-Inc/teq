// A member, an extension method or a conversion that gives the name comes before the dynamic rewrite, as do `->`,
// the members every value has and an enum companion's `valueOf`; only a name nothing gives is rewritten.
import scala.language.dynamics
import scala.language.implicitConversions

class Plain(val tag: String) extends Dynamic:
  def selectDynamic(name: String): String = s"$tag.select($name)"
  def applyDynamic(name: String)(args: Any*): String = s"$tag.apply($name)(${args.mkString(",")})"
  def member: String = "member"
  override def toString: String = tag

extension (p: Plain) def viaExt: String = "extension"

class Rich(p: Plain) { def viaClass: String = "implicit class" }
implicit def enrich(p: Plain): Rich = new Rich(p)

class Wide { def viaGiven: String = "given conversion" }
given Conversion[Plain, Wide] = _ => new Wide

enum Shade:
  case Red, Green
object Tint extends Dynamic:
  def selectDynamic(name: String): String = s"tint.select($name)"
  val of: String => Shade = Shade.valueOf

@main def run(): Unit =
  val p = new Plain("p")
  println(p.member)
  println(p.viaExt)
  println(p.viaClass)
  println(p.viaGiven)
  println(p.other)
  println(p -> 1)
  println(p.hashCode == p.hashCode)
  println(p.toString)
  println(p == p)
  println(Tint.of("Green"))
  println(Tint.other)
