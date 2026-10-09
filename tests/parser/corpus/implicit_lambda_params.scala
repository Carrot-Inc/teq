// `implicit x => ...`: the parameter of the function literal is a given inside its body, as
// in Scala 2 and still in Scala 3.
class Unsafe(val level: Int)
object Main:
  def withUnsafe[A](f: Unsafe => A): A = f(new Unsafe(3))
  def run[A](f: Int => A): A = f(3)
  def needs(implicit n: Int): Int = n * 2
  def danger(implicit u: Unsafe): String = s"level ${u.level}"
  def both(implicit n: Int, u: Unsafe): String = s"$n/${u.level}"
  def main(args: Array[String]): Unit =
    println(run { implicit n => needs })
    println(run { implicit (n: Int) => needs + 1 })
    println(withUnsafe { implicit unsafe => danger })
    println(withUnsafe { implicit unsafe =>
      implicit val n: Int = 7
      both
    })
    val f: Int => Int = implicit x => implicitly[Int] + 1
    println(f(10))
    println(List(1, 2).map(implicit i => needs))
