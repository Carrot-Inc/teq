package mub

import mua.*

// Selections on intersections of the upstream's types: one member through two instances of its
// class seen from the whole receiver (`get` takes `"k" | "v"`, either order, a polymorphic member
// too), two traits' members met (`f`), an overriding member beside the overridden one seen from
// the whole receiver kept as two alternatives (`S`'s `get` and `R`'s).
def both(r: R["k"] & R["v"]): String = r.get("k").toString + " " + r.get("v")
def reversed(r: R["v"] & R["k"]): Any = r.get("k")
def poly(r: R["k"] & R["v"]): Int = r.poly("v", 4)
def met(r: A & B): Product & Serializable = r.f("m")
def overriding(r: S["k"] & R["v"]): String = r.get("k")
def overridden(r: S["k"] & R["v"]): Any = r.get("v")

@main def run(): Unit = {
  println(both(Store("p").asInstanceOf[R["k"] & R["v"]]))
  println(reversed(Store("q").asInstanceOf[R["v"] & R["k"]]))
  println(poly(Store("n").asInstanceOf[R["k"] & R["v"]]))
  println(met(new AB))
  val ss = (new SS).asInstanceOf[S["k"] & R["v"]]
  println(overriding(ss))
  println(overridden(ss))
}
