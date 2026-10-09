// teq: --dialect strict
// expect: a second alternative of `go` is not allowed under the dialect flag `no-overloading`
// expect: `count` needs a declared type under the dialect flag `explicit-result-types`
// expect: values of types Int and String cannot be compared with == or !=
// expect: the call of the inline method `twice` is not allowed under the dialect flag `no-inline`
object O:
  def go(x: Int): Int = x
  def go(x: String): Int = x.length
  val count = 3
  inline def twice(x: Int): Int = x + x

@main def main(): Unit =
  println(O.go(1) + O.count + O.twice(1))
  println(1 == "a")
