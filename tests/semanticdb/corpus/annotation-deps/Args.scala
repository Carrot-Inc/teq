class Ann(x: Any) extends scala.annotation.StaticAnnotation
class Box[A]
class C { type T }
object Args {
  def map[X, Y](a: X)(k: X => Y): Y = k(a)
  val direct = map(new C)(x => (??? : Box[Int @Ann(x)]))
  val block = map(new C)(x => (??? : Box[Int @Ann({ val y = x; 1 })]))
  val cases = map(new C)(x => (??? : Box[Int @Ann(1 match { case _ => x })]))
  val attempt = map(new C)(x => (??? : Box[Int @Ann(try x finally ())]))
}
