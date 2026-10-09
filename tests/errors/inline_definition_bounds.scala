// expect: 12:16: error: type argument Int does not conform to upper bound String
// expect: 15:16: error: type argument Int does not conform to upper bound String
// expect: 20:16: error: type argument Int does not conform to upper bound String
// expect: 3 errors found
// A type argument's bounds in an inline body are checked at the definition once, called or not,
// as scalac 3.8.4 checks them in `PostTyper` (the method stays inline): no expansion checks them
// again, so the twice-called `called` reports one error.
class Box[A <: String]

object Lib:
  inline def called(): Int =
    val x: Box[Int] = null
    1
  inline def uncalled(): Int =
    val x: Box[Int] = null
    2

def local(): Int =
  inline def f(): Int =
    val y: Box[Int] = null
    3
  f()

@main def run(): Unit = println(Lib.called() + Lib.called() + local())
