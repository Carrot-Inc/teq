package ec

import ea.*

// A module scalac compiles against the upstream, run over the upstream's class files: the calls name the
// descriptors scalac's erasure of the declarations gives.
object Client:
  def attempt(name: String)(f: => Any): Unit =
    try println(name + ": " + f)
    catch
      case e: LinkageError => println(name + ": " + e.getClass.getSimpleName)
      case e: IllegalStateException => println(name + ": " + e.getMessage)

  def main(args: Array[String]): Unit =
    val api = new API
    attempt("union")(api.union[V](new V(2)).x)
    attempt("param")(api.param(new V(3)).x)
    attempt("aliasU")(api.aliasU(new V(4)).x)
    attempt("strs")(api.strs("s"))
    attempt("alias")(api.alias((1, "a")))
    attempt("second")(api.second((2, "b")))
    attempt("generic")(api.generic(Tuple1(3)))
    attempt("arr")(api.arr(Array((4, "c"))).toList)
    attempt("tbound")(api.tbound((5, "d")))
    attempt("charBound")(api.charBound('e'))
    attempt("inferredNull")(api.inferredNull)
    attempt("inferredNothing")(api.inferredNothing(true))
    attempt("holder")(new Holder((6, "f")).chain)
    val r = new Reviewed
    attempt("applied")(r.applied(new V(7)).x)
    attempt("bounded")(r.bounded(new V(8)).x)
    attempt("inter")(r.inter(new V(9)).x)
    attempt("chainInter")(r.chainInter(Tuple1(10)))
    attempt("refine")(r.refine(Tuple1(11).asInstanceOf[Types.G[Int] { type T = Int }]))
    attempt("byName")(r.byName(new V(12)).x)
    attempt("n")(r.n)
    attempt("p")(r.p(null))
    attempt("nb")(r.nb(null))
    attempt("v")(r.v)
    val abs = new Abs
    attempt("dep")(r.dep(abs)(new V(13)).x)
    val bs = new Bottoms
    attempt("bottomInter")(bs.bottomInter(null))
    attempt("bottomRefine")(bs.bottomRefine(null))
    val bi = new BottomImpl
    attempt("bottomDep")(bs.bottomDep(bi)(null))
