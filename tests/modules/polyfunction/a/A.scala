package pfa

object A:
  type Pick = [T] => Int => List[T] => Option[T]
  val pick: Pick = [T] => (i: Int) => (xs: List[T]) => xs.drop(i).headOption
  def use(f: Pick): Option[String] = f[String](1)(List("a", "b"))
