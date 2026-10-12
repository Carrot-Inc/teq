package eb

import ea.*

@main def run(): Unit =
  val api = new API
  println(api.union[V](new V(2)).x)
  println(api.param(new V(3)).x)
  println(api.aliasU(new V(4)).x)
  println(api.strs("s"))
  println(api.alias((1, "a")))
  println(api.second((2, "b")))
  println(api.generic(Tuple1(3)))
  println(api.tbound((5, "d")))
  println(api.charBound('e'))
  println(api.inferredNull)
  try api.inferredNothing(true) catch case _: IllegalStateException => println("thrown")
  println(new Holder((6, "f")).chain)
  val r = new Reviewed
  println(r.applied(new V(7)).x)
  println(r.bounded(new V(8)).x)
  println(r.inter(new V(9)).x)
  println(r.chainInter(Tuple1(10)))
  println(r.refine(Tuple1(11).asInstanceOf[Types.G[Int] { type T = Int }]))
  println(r.byName(new V(12)).x)
  println(r.n)
  println(r.p(null))
  println(r.nb(null))
  println(r.v)
  val abs = new Abs
  println(r.dep(abs)(new V(13)).x)
  val bs = new Bottoms
  println(bs.bottomInter(null))
  println(bs.bottomRefine(null))
  val bi = new BottomImpl
  println(bs.bottomDep(bi)(null))
