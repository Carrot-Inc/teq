// A case class local to a method has a type with no prefix, so its equals compares no enclosing instances, though an
// inline method naming the enclosing instance in a type gives the class an outer accessor; a member case class's
// equals compares them.
import scala.compiletime.summonInline

class Outer(val n: Int):
  def make: Any =
    case class Local(x: Int):
      inline def read: Int =
        summonInline[ValueOf[Outer.this.type]].value.n
    new Local(1)
  def plain: Any =
    case class Plain(x: Int)
    new Plain(1)
  case class Member(x: Int)

@main def test(): Unit =
  println(new Outer(1).make == new Outer(2).make)
  println(new Outer(1).plain == new Outer(2).plain)
  val o = new Outer(1)
  println(new o.Member(1) == new o.Member(1))
  println(new o.Member(1) == new Outer(2).Member(1))
