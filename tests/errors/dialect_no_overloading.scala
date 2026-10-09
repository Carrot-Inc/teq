// teq: --dialect no-overloading
// expect: a second alternative of `go` is not allowed under the dialect flag `no-overloading`: every call of an overloaded name is resolved among its alternatives by shape and by applicability; give the alternatives distinct names
object O:
  def go(x: Int): Int = x
  def go(x: String): Int = x.length

@main def main(): Unit =
  println(O.go(1))
