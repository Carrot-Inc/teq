// A member an anonymous `Selectable` declares and no parent has refines the instance's type (dotty's
// `TypeOps.classBound` admits fresh members where a parent derives from `Selectable`), so it is called
// through the instance as scalac calls it, reflectively. A private alias in its type is what the alias stands for
// (`avoid(decl.info, missingDecls)`).
import scala.reflect.Selectable.reflectiveSelectable

@main def main(): Unit =
  val x = new reflect.Selectable { def f(): Int = 7; val name = "x"; def twice(n: Int): Int = n * 2 }
  println(x.f())
  println(x.name)
  println(x.twice(21))
  val y = new reflect.Selectable { private type Hidden = String; def f: Hidden = "hidden" }
  val h: String = y.f
  println(h)
