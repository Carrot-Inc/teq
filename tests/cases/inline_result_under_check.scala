// An inline method whose result type is inferred, calling another inline method whose result is inferred too and
// whose definition has not been checked yet (it is written after its caller, or is first needed by it): the
// callee's definition is checked where the caller's body needs its result, and the caller's result is the type
// its body has with it.

// Expects 16: the pair written after the value that calls it, the value's type declared. scalac prints 16.
object ValueDeclared:
  object Test:
    val held: Int = Lib.outer(5)
    def main(args: Array[String]): Unit = println(held)
  object Lib:
    inline def inner(x: Int) = x * 3
    inline def outer(x: Int) = inner(x) + 1

// Expects 16: the caller written before its callee. scalac prints 16.
object CallerFirst:
  object Lib:
    inline def outer(x: Int) = inner(x) + 1
    inline def inner(x: Int) = x * 3
  object Test:
    def main(args: Array[String]): Unit = println(Lib.outer(5))

// Expects 31: three inline methods, each written before the one it calls. scalac prints 31.
object ThreeLevels:
  object Lib:
    inline def top(x: Int) = middle(x) + 1
    inline def middle(x: Int) = bottom(x) * 2
    inline def bottom(x: Int) = x + 10
  object Test:
    def main(args: Array[String]): Unit = println(Lib.top(5))

// Expects 16: both methods transparent. scalac prints 16.
object BothTransparent:
  object Lib:
    transparent inline def outer(x: Int) = inner(x) + 1
    transparent inline def inner(x: Int) = x * 3
  object Test:
    val held = Lib.outer(5)
    def main(args: Array[String]): Unit = println(held)

// Expects "got 16": the pair in a block, the caller first. scalac prints got 16.
object InABlock:
  object Test:
    def run(n: Int): String =
      inline def outer(x: Int) = inner(x) + 1
      inline def inner(x: Int) = x * 3
      "got " + outer(n)
    def main(args: Array[String]): Unit = println(run(5))

// Expects "got 16": the pair in a class of a block. scalac prints got 16.
object InAClassOfABlock:
  object Test:
    def run(n: Int): String =
      class Keeper:
        inline def outer(x: Int) = inner(x) + 1
        inline def inner(x: Int) = x * 3
      "got " + new Keeper().outer(n)
    def main(args: Array[String]): Unit = println(run(5))

// Expects "string": the callee's result picks the overload its caller's body calls. scalac prints string.
object PicksAnOverload:
  object Lib:
    def which(x: Int): String = "int"
    def which(x: String): String = "string"
    def which(x: Any): String = "any"
    inline def outer(x: Int) = which(inner(x))
    inline def inner(x: Int) = "s" + x
  object Test:
    def main(args: Array[String]): Unit = println(Lib.outer(5))

// Expects 16: the callee written before its caller, as it always printed. scalac prints 16.
object CalleeFirst:
  object Lib:
    inline def inner(x: Int) = x * 3
    inline def outer(x: Int) = inner(x) + 1
  object Test:
    val held = Lib.outer(5)
    def main(args: Array[String]): Unit = println(held)

@main def run(): Unit =
  ValueDeclared.Test.main(Array())
  CallerFirst.Test.main(Array())
  ThreeLevels.Test.main(Array())
  BothTransparent.Test.main(Array())
  InABlock.Test.main(Array())
  InAClassOfABlock.Test.main(Array())
  PicksAnOverload.Test.main(Array())
  CalleeFirst.Test.main(Array())
