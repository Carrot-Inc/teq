// A call of a def by itself in the right operand of `&&` or `||` under a `return` is the loop's jump (dotty's
// TailRec takes both operators' right operand as a tail position): a hundred thousand deep it runs.
object A {
  final def andOr(n: Int): Boolean = { if n > 0 then return (n > -1 && andOr(n - 1)); true }
}
object B {
  final def orElse(n: Int): Boolean = { if n > 0 then return (n < -1 || orElse(n - 1)); false }
}
object Main {
  def main(args: Array[String]): Unit = {
    println(A.andOr(3))
    println(B.orElse(3))
    println(A.andOr(100000))
    println(B.orElse(100000))
  }
}
