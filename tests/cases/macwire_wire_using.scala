//> using dep com.softwaremill.macwire::macros:2.6.7
// jars: macwire-macros
// macwire's wire[C] of a class of several parameter clauses, a using clause among them: macwire
// applies the constructor one clause at a time (`ConstructorCrimper`, `Apply(acc, args)`), each
// application of the method type of its clause and of its result (`TypeAssigner.assignType`), the
// given of a using clause found by `Implicits.search`; the last clause's makes the instance.
import com.softwaremill.macwire.*

class Coll:
  override def toString = "coll"
class Api(coll: Coll)(using s: String):
  def show = s"api $coll $s"
class Plain(coll: Coll)(s: String):
  def show = s"plain $coll $s"
class Three(coll: Coll)(n: Int)(using s: String):
  def show = s"three $coll $n $s"
class Gen[A](a: A)(using s: String):
  def show = s"gen $a $s"

object Env:
  val coll = new Coll
  given s: String = "x"
  val n: Int = 3
  lazy val api = wire[Api]
  lazy val plain = wire[Plain]
  lazy val three = wire[Three]
  lazy val gen = wire[Gen[Coll]]

@main def main() =
  println(Env.api.show)
  println(Env.plain.show)
  println(Env.three.show)
  println(Env.gen.show)
