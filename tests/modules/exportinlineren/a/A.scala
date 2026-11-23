package lib

trait Show[A] { def show(a: A): String }
object Show:
  given Show[Int] = a => "#" + a
trait Size[A] { def size(a: A): Int }
object Size:
  given Size[Int] = a => a * 10

object Codecs:
  inline def mt1[A](a: A): A = a
  inline def mt2[A, B](a: A, b: B): String = a.toString + b.toString
  inline def mbnd[A <: AnyVal](a: A): String = a.toString + "!"
  inline def mcbo[A: Show](a: A): String = summon[Show[A]].show(a)
  inline def mcba[A: {Show, Size}](a: A): String = summon[Show[A]].show(a) + summon[Size[A]].size(a)
  inline def musg[A](a: A)(using s: Show[A]): String = s.show(a)
  inline def mbnm[A](a: => A): A = a
  inline def mplain(x: Int): Int = x + 1
  def mnoinl[A](a: A): A = a
