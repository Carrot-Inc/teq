// A conversion method to `Null` is an implicit function value into any reference type, as `Null`
// conforms to them without explicit nulls (scalac: "null").
implicit def c(x: Int): Null = null
@main def run(): Unit =
  val f = summon[Int => String]
  println(f(1))
