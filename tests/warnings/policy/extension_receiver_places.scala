object A:
  extension (n: Int = 1) def +:(s: String): String = s * n
  extension (n: Int) def +::(s: String = "q"): String = s * n
  extension (n: Int = 1) def +:::(s: String = "q"): String = s * n
  extension [T](t: T = 5) def show2[U](u: U = 1): String = ""
  extension (using DummyImplicit)(t: Int = 0)(using x: DummyImplicit) def show3: String = ""
trait T1:
  extension (s: String = "a") def t: Int
object O1 extends T1:
  extension (s: String) def t: Int = 1
object B:
  def m(): Int =
    extension (s: String = "x") def loc: Int = s.length
    "a".loc
  given AnyRef with
    extension (s: String = "x") def gg: Int = s.length
object E:
  extension (s: String = "x") def te: Int = s.length
object F:
  export E.te
class Cls:
  extension (s: String = "x") def ct: Int = s.length
object G:
  extension (s: String = "x") inline def it: Int = 3
  extension (inline s: String = "x") inline def it2: Int = 3
@main def run(): Unit = println(B.m())
