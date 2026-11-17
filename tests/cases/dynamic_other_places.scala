// The other places that look a member up rewrite it too: a right-associative operator whose right operand is the
// Dynamic value (the left operand evaluated first), `super.name` (the parent's method, not the override), a
// selection inside a pattern (an extractor reached by `selectDynamic`, a stable identifier pattern).
import scala.language.dynamics

class Echo(val tag: String) extends Dynamic:
  def selectDynamic(name: String): String = s"$tag.select($name)"
  def applyDynamic(name: String)(args: Any*): String = s"$tag.apply($name)(${args.mkString(",")})"

class Base extends Dynamic:
  def selectDynamic(name: String): String = s"base.select($name)"
  def applyDynamic(name: String)(args: Any*): String = s"base.apply($name)(${args.mkString(",")})"

class Sub extends Base:
  override def selectDynamic(name: String): String = s"sub.select($name)"
  override def applyDynamic(name: String)(args: Any*): String = s"sub.apply($name)(${args.mkString(",")})"
  def viaSuper: String = super.field + "|" + super.call(1)
  def viaThis: String = this.field + "|" + this.call(1)

object Half { def unapply(n: Int): Option[Int] = if n % 2 == 0 then Some(n / 2) else None }
class Finder extends Dynamic:
  def selectDynamic(name: String): Half.type = { println(s"select($name)"); Half }

@main def run(): Unit =
  val d = new Echo("d")
  println({ println("left"); 1 } +: { println("right"); d })
  println(2 :: d)
  val s = new Sub
  println(s.viaSuper)
  println(s.viaThis)
  val f = new Finder
  8 match
    case f.Halved(n) => println(n)
    case _ => println("other")
  "d.select(name)" match
    case d.name => println("matched")
    case _ => println("other")
