import scala.annotation.StaticAnnotation
class OrAnn extends StaticAnnotation
class OrC { type T }
class OrOut[+A]
class OrInv[A]
object OrType {
  def map[A, B](a: A)(f: A => B): B = f(a)
  val union = map(new OrC)(x => (??? : OrInv[(x.T @OrAnn) | String]))
  val unionOut = map(new OrC)(x => (??? : OrOut[(x.T @OrAnn) | String]))
}
