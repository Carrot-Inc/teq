// A match type reduces before ElimOpaque (Types.scala:5259), so a cast to `M[O.OS]` keeps the typer's answer, which
// `later` reads after it. Past ElimOpaque an opaque alias read through an object is a plain alias seen from it
// (ElimOpaque.scala:54-55), so a cast to it checks the alias's class.
class C[T]:
  opaque type OL = List[T]
  opaque type OS = String
trait Tr:
  opaque type TO = String
object K extends C[String]
object Q extends Tr
object O:
  opaque type OS = String
type M[X] = X match
  case String => Int
  case _ => Boolean
object Main:
  def cc(f: => Any): String = try { f; "cast" } catch { case _: ClassCastException => "cce" }
  def first(o: Any): String = cc(o.asInstanceOf[M[O.OS]])
  def later(x: M[O.OS]): String = x.toString
  def main(args: Array[String]): Unit =
    val o = new Object
    println(List(cc(o.asInstanceOf[K.OL]), cc(o.asInstanceOf[K.OS]), cc(o.asInstanceOf[Q.TO]), cc("s".asInstanceOf[Q.TO])).mkString(" "))
    println(List(first(o), first("s"), cc(o.asInstanceOf[M[String]])).mkString(" "))
