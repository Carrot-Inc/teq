// The lowered code's statistics tell a unit's runs apart from its lowering (tests/fold.sh): `f`, promoted at its
// second call, runs lowered once of two calls and nine of ten, finding its code at each call after the first run.
object Main:
  def f(n: Int): Int = n + 1
  def main(args: Array[String]): Unit =
    val count = if args.isEmpty then 2 else 10
    var n = 0
    while n < count do n = f(n)
    println("done")
