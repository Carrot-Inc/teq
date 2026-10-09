trait Show[A]:
  def show(a: A): String
given Show[Int] = new AnyRef with Show[Int]:
  def show(a: Int): String = "int " + a
object Lock extends AnyRef
class Token extends AnyRef:
  override def toString = "token"
@main def main(): Unit =
  println(summon[Show[Int]].show(1))
  val anon = new AnyRef { override def toString = "anon" }
  println(anon.toString)
  println(new Token)
  println(Lock eq Lock)
