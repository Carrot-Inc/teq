class TbC { type T }
class TbIn[-A]
class TbOut[+A]
class TbInv[A]
object TypeBounds {
  def map[A, B](a: A)(f: A => B): B = f(a)
  val upperIn = map(new TbC)(x => (??? : TbInv[? <: TbIn[x.T]]))
  val lowerOut = map(new TbC)(x => (??? : TbInv[? >: TbOut[x.T]]))
}
