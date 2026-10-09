// Polymorphic function types and literals: values, application with inferred and explicit
// type arguments, parameters of a polymorphic function type, and one built for an expected type.
object Main:
  val wrap: [T] => T => List[T] = [T] => (x: T) => List(x)
  val pairUp: [A, B] => (A, B) => (B, A) = [A, B] => (a: A, b: B) => (b, a)
  def twice[T](f: [X] => X => List[X], t: T): List[T] = f(t) ++ f[T](t)
  def foldAll[Acc](xs: List[Any], init: Acc)(f: [t] => (Acc, t) => Acc): Acc =
    xs.foldLeft(init)((acc, x) => f(acc, x))
  def main(args: Array[String]): Unit =
    println(wrap(1))
    println(wrap[String]("s"))
    println(pairUp(1, "a"))
    println(twice(wrap, 2.5))
    println(twice([X] => (x: X) => List(x, x), "z"))
    println(foldAll(List(1, "two", 3), "")([t] => (acc: String, x: t) => acc + x.toString))
    val g: [T] => (T, Int) => Option[T] = [T] => (t: T, n: Int) => if n > 0 then Some(t) else None
    println(s"${g("k", 1)} ${g(2, 0)}")
