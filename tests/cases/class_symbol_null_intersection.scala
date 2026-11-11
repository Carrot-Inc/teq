// `classSymbol(Null & A)` is `Null`: `isSubClass` puts `Null` below every class but `Nothing`
// (SymDenotations.scala:2073-2081), so the extracted `v: T <: Null & A` finds the given `Show[Null]`.
class A
class Box[T](val t: T)
object Box:
  def unapply[T <: Null & A](b: Box[T]): Some[T] = Some(b.t)
trait Show[X]:
  def s: String
given Show[Null]:
  def s = "null-show"
object Main:
  def show[X](x: X)(using sh: Show[X]): String = sh.s
  def main(args: Array[String]): Unit =
    val x: Any = new Box[Null & A](null)
    x match
      case Box(v) => println(show(v))
      case _ => ()
