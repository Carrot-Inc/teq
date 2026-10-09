// A stored inline body keeps the type arguments of every call in it, a call an implicit search
// built included (`tc[Int]` here, beside `use[Int]`): the census checks it.
trait TC[A]
implicit def tc[A]: TC[A] = new TC[A] {}
def use[A](x: A)(using TC[A]): Int = 1
inline def f(x: Int): Int = use(x)
@main def run(): Unit = println(f(1))
