// A tuple index is typed by `Tuple.Elem`'s reduction, as scala-library's `Tuple.apply` is, on a tuple whose type gives
// its elements only up to the index too: element 0 of `Int *: Tuple` is an `Int`, element 1 of `Int *: String *:
// Tuple` a `String`; past what the type gives the element is read at run time, as on an abstract `T <: Tuple`.
def first(t: Int *: Tuple): Int = t(0)
def second(t: Int *: String *: Tuple): String = t(1)
def later(t: Int *: Tuple): Any = t(1)
def generic[T <: Tuple](t: T): Any = t(1)
@main def run(): Unit =
  println(first((1, "a")) + 1)
  println(second((2, "bee", 3.0)).length)
  println(later((3, "c")))
  println(generic((4, "d")))
  try println(later(Tuple1(5))) catch case e: IndexOutOfBoundsException => println("throws")
