// Type arguments and a using clause given to methods that take neither, overloaded or not: a method of no type
// parameters is never applied to type arguments (Applications.typedTypeApply, resolveOverloaded keeping the
// alternatives a PolyProto canInstantiate), nor one of a plain clause to a using list (resolveOverloaded of an
// ApplyKind.Using prototype, then tryParameterless and missingArgs), as scalac 3.8.4 refuses them.
class G:
  def f(): Unit = ()
  def f(x$0: Long): Unit = ()
  def f(x$0: Long, x$1: Int): Unit = ()
  def n(): Unit = ()
  def k(x: Long): Unit = ()
  def k(x: Long, y: Int): Unit = ()
object Main:
  def main(args: Array[String]): Unit =
    val g = new G
    g.f[Int](1L)
    g.n[Int]()
    g.f(using 1L)
    g.n(using 1L)
    g.k(using 1L)
// expect: 15:5: error: None of the overloaded alternatives of method f in class G with types
// expect:  (x$0: Long, x$1: Int): Unit
// expect:  (x$0: Long): Unit
// expect:  (): Unit
// expect: match type arguments [Int] and arguments (1L)
// expect: 16:5: error: method n in class G does not take type parameters
// expect: 17:5: error: Missing arguments for method f in class G
// expect: 18:5: error: Missing arguments for method n in class G
// expect: 19:5: error: None of the overloaded alternatives of method k in class G with types
// expect:  (x: Long, y: Int): Unit
// expect:  (x: Long): Unit
// expect: match arguments (1L)
// expect: 5 errors found
