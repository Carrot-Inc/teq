// jars: fixtures
// std: lean scala-library
// Overloads of a jar of which one is `protected[C]` or `private[p]` (http4s's deprecated
// `protected[CORSPolicy] def apply` and `private[client] def translate`): outside `C` and `p`
// only the public alternative is taken, where both would apply and the hidden one is more
// specific; and of two conversions to a member, the one whose member is `private[p]` is none.
import fix.shapes.{QaClient, QaPolicy}
import fix.shapes.QaSyntax.*

object Main:
  def main(args: Array[String]): Unit =
    println(QaPolicy()(Option(1)))
    println(QaClient().tr(Option(1)))
    println(List(1, 2).contains_(2))
