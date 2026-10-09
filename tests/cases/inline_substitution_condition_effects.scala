// A constant condition stays before its branch unless scalac's `isIdempotentExpr` holds of it:
// a primitive operation is pure only where its application is a constant (`1 / n` of a
// parameter runs, and throws for 0), a string conversion of a value runs its `toString`, a
// conditional and a statement are kept, a constant operation is dropped. scalac prints `zero`,
// `toString`, 1, `toString`, 3, `flag`, 5, 7, 1, 9.
class W:
  override def toString: String = { println("toString"); "w" }
var count = 0
def flag(): Boolean = { println("flag"); true }
inline def divide(n: Int): Int = if ({ val q = 1 / n; true }: true) then 1 else 2
def viaParam(n: Int): Int = divide(n)
inline def concat(w: W): Int = if ({ val s = "v=" + w; true }: true) then 1 else 2
inline def stringOf(w: W): Int = if ({ val s = w.toString; true }: true) then 3 else 4
inline def nested: Int = if ((if flag() then true else true): true) then 5 else 6
inline def statement: Int = if ({ count += 1; true }: true) then 7 else 8
inline def pureOps(n: Int): Int = if ({ val k = n + 1; true }: true) then 9 else 10
@main def run(): Unit =
  try println(viaParam(0))
  catch case _: ArithmeticException => println("zero")
  println(concat(new W))
  println(stringOf(new W))
  println(nested)
  println(statement)
  println(count)
  println(pureOps(4))
