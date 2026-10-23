class Ann(x: Any) extends scala.annotation.StaticAnnotation
class Box[A]
class C { type T }
object Args {
  def map[X, Y](a: X)(k: X => Y): Y = k(a)
  val avoided = map(new C)(x => (??? : Box[x.T @Ann(x)]))
  val constant = map(new C)(x => (??? : Box[Int @Ann(1)]))
}
