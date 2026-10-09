trait Show[A]:
  def show(a: A): String
object T:
  given a: Show[Int] = _.toString
  given b: Show[Int] = x => s"<$x>"
  def s = summon[Show[Int]].show(1)
object U:
  given [T](using Show[T]): Show[List[T]] = xs => xs.map(summon[Show[T]].show).mkString(",")
  given [T](using Show[List[T]]): Show[T] = x => summon[Show[List[T]]].show(List(x))
  def s = summon[Show[Int]].show(1)
@main def run(): Unit = println(T.s)
