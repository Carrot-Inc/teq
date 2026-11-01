// teq: --wunused all --werror
// A parameter accessor read in the constructor's code, which reads the parameter (`Constructors.intoConstr`), is the
// accessor's read to `CheckUnused` (`Ident(_) => rhs.symbol.is(ParamAccessor)`): a local function returning it is no
// use of its own parameter to warn of, a val, a private val and a plain parameter alike.
class A(val s: Int, private val p: Int, q: Int):
  val v = { def g(k: Int) = s; g(1) }
  val w = { def h(k: Int) = p; h(1) }
  val u = { def i(k: Int) = q; i(1) }
@main def run(): Unit =
  val a = A(1, 2, 3)
  println(a.v + a.w + a.u)
