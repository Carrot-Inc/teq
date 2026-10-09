// expect: 'erased' is not part of the supported Scala subset
inline def twice(inline x: Int) = x + x
transparent inline def t(x: Int): Int = x
inline val MV = 1
transparent trait Ok
def f(erased x: Int): Int = 1
erased def g: Int = 2
inline given Int = 3
object O:
  inline def m = 1
extension (x: Int)
  inline def plus1 = x + 1
def h(x: Int) =
  inline if x > 0 then 1 else 2
def k(x: Int) =
  inline x match
    case 1 => "one"
    case _ => "other"
@main def run(): Unit = println(twice(1))
