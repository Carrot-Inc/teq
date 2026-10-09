trait Marked { override def toString = super.toString + " marked" }
trait Starred { override def toString = super.toString + " starred" }
class Named(val n: String) { override def toString = "Named(" + n + ")" }
class Both(n: String) extends Named(n) with Marked with Starred
class Plain extends Marked with Starred:
  override def toString = "[" + (if super.toString.contains("@") then "Plain@x marked starred" else "?") + "]"
class Token(val id: Int):
  override def equals(that: Any) = that match
    case t: Token => t.id == id || super.equals(that)
    case _ => false
  override def hashCode = if id < 0 then super.hashCode else id
case object Nil2:
  override def equals(that: Any) = that match
    case _: this.type => true
    case _ => super.equals(that)
@main def run(): Unit =
  println(Both("b"))
  println(Plain())
  println(Token(1) == Token(1))
  println(Token(1) == Token(2))
  println(Token(3).hashCode)
  val t = Token(-1)
  println(t.hashCode == t.hashCode)
  println(Nil2 == Nil2)
