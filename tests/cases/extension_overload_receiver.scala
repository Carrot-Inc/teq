// Same-named extensions are overloads whose first argument list is the receiver: the receiver
// ranks them first, and the lists after it decide only between equally specific receivers, over
// every alternative the receiver applies to, the generic ones included (scalac's
// `resolveOverloaded`), whether the call names the extension directly or selects it.
object A:
  extension (s: String)(using x: Int) def g(y: Int): Int = 1
  extension [T](s: T)(using x: Int) def g(y: String): Int = 2
  extension (s: String)(using x: Int) def g(y: Boolean): Int = 3

object B:
  extension (s: String)(using x: Int) def g(y: Int, z: Int): Int = 1
  extension [T](s: T)(using x: Int) def g(y: String): Int = 2
  extension (s: String)(using x: Int) def g(y: Boolean): Int = 3

object C:
  extension (s: String) def t(p: (Int, Int)): Int = p._1 + p._2
  extension (s: String) def u(p: Int): Int = p
  extension (s: String) def u(p: Int, q: Int): Int = p * q
  extension [T](s: T) def u(p: Int, q: Int, r: Int): Int = p + q + r

object D:
  extension (s: String) def f: Int => Int = x => x + 1
  extension (s: Any) def f(x: Int, y: Int): Int = 9

object E:
  extension (s: String)
    def f: Int => Int = x => x + 1
  extension (s: String)
    def f(x: Int, y: Int): Int = 9

object Main:
  def main(args: Array[String]): Unit =
    given Int = 3
    println(List(A.g("a")("b"), A.g("a")(true), A.g("a")(1), A.g(5)("b")))
    println(List(B.g("a")("b"), B.g("a")(true), B.g("a")(1, 2)))
    locally {
      import A.*
      println(List("a".g("b"), "a".g(true), "a".g(1), 5.g("b")))
    }
    locally {
      import B.*
      println(List("a".g("b"), "a".g(true), "a".g(1, 2)))
    }
    locally {
      import D.*
      println(List("a".f(1), 5.f(1, 2)))
    }
    locally {
      import E.*
      println(List("a".f(1), "a".f(1, 2)))
    }
    import C.*
    println(List("a".t(1, 2), "a".u(3), "a".u(3, 4), "a".u(1, 2, 3)))
