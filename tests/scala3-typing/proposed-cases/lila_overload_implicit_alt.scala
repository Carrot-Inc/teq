// lila: PlanUi: strong(money.display) with Money.display(locale: Locale) and
// Money.display(using Lang), where the expected type is a scalatags Frag reached by a
// String => Frag conversion. No alternative is compatible with Frag, so scalac
// (Applications.resolveOverloaded, "convertible" branch) keeps the single alternative that
// is not a method after normalization (the using-only one) and converts its result.
import scala.language.implicitConversions
case class Lang(code: String)
case class Frag(s: String)
given Conversion[String, Frag] = Frag(_)

case class Money(amount: Int):
  def display(locale: String): String = s"$amount ($locale)"
  def display(using lang: Lang): String = display(lang.code)

def strong(f: Frag): String = s"<strong>${f.s}</strong>"

@main def run() =
  given Lang = Lang("en")
  println(strong(Money(5).display))
