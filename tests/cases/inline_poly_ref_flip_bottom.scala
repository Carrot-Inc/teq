// An inline method named without a parameter list whose type argument nothing bounds below:
// dotty's `Inliner` defines the call's type arguments fully under `ForceDegree.flipBottom`, so
// `meth[R <: Int]` expected an `Int` is `meth[Int]` and its cast is to `Int` (scala3's i7798), and
// one bounded below by its expected type keeps that.
object Test:
  inline def meth[R <: Int]: R = 3.asInstanceOf[R]
  inline def show[S <: AnyVal]: String = (7: AnyVal).asInstanceOf[S].toString
  def main(args: Array[String]): Unit =
    val t: Int = meth
    println(t + 1)
    val s: String = show
    println(s)
