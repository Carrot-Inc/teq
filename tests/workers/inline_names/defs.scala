package names

trait Show:
  def show(a: Int): String
trait Step:
  def next(i: Int): Int
inline def make(label: String): Show = new Show { def show(a: Int) = label + a }
inline def stepper(k: Int): Step = i => i * k
inline def two(label: String): List[Show] = List(make(label + "a"), make(label + "b"))
inline def chosen(inline b: Boolean, label: String): Show = if b then make("dead") else make(label)
inline def plain(b: Boolean, label: String): Show = if b then make("dead") else make(label)
inline def steps(k: Int): Int = stepper(k).next(1) + stepper(k + 1).next(1)
def pair[A](f: Int => A, b: A): String = f(1).toString + b
def mapped[A, B](xs: List[A], f: A => B, extra: B): List[B] = xs.map(f) :+ extra
inline def capturing(u: Int, v: Int): Show =
  val first = u * 10
  val second = v * 100
  new Show { def show(a: Int) = pair(x => x + first, second) + mapped(List(a), y => y + first, second).mkString }
trait Read:
  def value: Int
inline def pass(inline x: Int): Int = x
inline def passing(u: Int, v: Int): Read =
  val first = u * 10
  val second = v * 100
  new Read:
    def value: Int = pass(first) + second
class Holder(val elem: Show)
inline def holding(label: String): Holder =
  new Holder(new Show { def show(a: Int) = label + a }):
    def tag: Int = 0
