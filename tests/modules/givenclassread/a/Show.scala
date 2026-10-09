package gca

// Given classes of a trait and of an object, generic and of a using clause, which a downstream
// built over the products summons and calls: the given def typed as the given was declared, its
// body making the class read from the products, as the whole build makes the source's.
trait Show[A]:
  def show(a: A): String

trait LowShows:
  given listShow[A](using s: Show[A]): Show[List[A]] with
    def show(as: List[A]): String = as.map(s.show).mkString("[", ",", "]")
  given pairShow[A, B]: Show[(A, B)] with
    def show(p: (A, B)): String = s"<${p._1}|${p._2}>"

object Show extends LowShows:
  given intShow: Show[Int] with
    def show(a: Int): String = "#" + a
  given optShow[A](using s: Show[A]): Show[Option[A]] with
    def show(o: Option[A]): String = o.fold("none")(s.show)
