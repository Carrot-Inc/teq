// A `return` of a call of the def by itself inside an argument of a call of itself under a `return`: the inner jump
// goes round the loop with the parameters it assigns, the outer call is never made; the outer arguments read the
// iteration's parameters. A hundred thousand deep.
object Par {
  final def argJump(n: Int, m: Int): Int = {
    if n > 0 then return argJump(n - 1, if n == 2 then return argJump(0, n * 100) else m + n)
    m
  }
  final def argJumpDeep(n: Int, m: Int): Int = {
    if n > 0 then return argJumpDeep(n - 1, if n % 1000 == 0 then return argJumpDeep(n - 1, m + 1) else m)
    m
  }
}
object Main {
  def main(args: Array[String]): Unit = {
    println(Par.argJump(3, 1))
    println(Par.argJumpDeep(100000, 0))
  }
}
