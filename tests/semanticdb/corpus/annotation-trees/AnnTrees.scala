import scala.annotation.StaticAnnotation
class Ann(val x: Any) extends StaticAnnotation
class Box[A]
class C { type T }
object AnnTrees {
  def map[X, Y](a: X)(k: X => Y): Y = k(a)
  def id[A]: Int = 1
  val typeArg = map(new C)(x => (??? : Box[x.T @Ann(id[x.T])]))
  val typeTest = map(new C)(x => (??? : Box[x.T @Ann(1.isInstanceOf[x.T])]))
  val localType = map(new C)(x => (??? : Box[x.T @Ann({ type L = x.T; 1 })]))
}
