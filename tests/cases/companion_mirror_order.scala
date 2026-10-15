// The companions' mirror parents come after their written ones, as `SyntheticMembers.addMirrorSupport`
// adds them after typing: a case class whose constructor reads its companion's inherited type, and a
// sealed trait whose child reads it, each complete before its companion's mirror is decided. A case
// class beside a method of its name keeps its constructor proxy, teq's namer holding no module there.
import scala.deriving.Mirror

trait Typed[Tag] { type Id = String }
case class Owner(owner: Owner.Id)
object Owner extends Typed[Owner]

trait Keys { type Key = Int; trait Named { def name: String } }
sealed trait Shape extends Shape.Named
object Shape extends Keys
case class Sq(k: Shape.Key) extends Shape { def name = "sq" }
case object Dot extends Shape { def name = "dot" }

object T {
  def A(s: String): A = new A(3, s)
  case class A(i: Int, s: String)
}

@main def run(): Unit =
  println(Owner("x"))
  println(summon[Mirror.ProductOf[Owner]].fromProduct(Tuple1("y")))
  println(summon[Mirror.SumOf[Shape]].ordinal(Dot))
  println(summon[Mirror.SumOf[Shape]] eq Shape)
  println(List[Shape](Sq(2), Dot).map(_.name))
  println(T.A("x"))
