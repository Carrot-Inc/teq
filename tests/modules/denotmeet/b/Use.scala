package dmb

import dma.*

// Selections on intersections of the upstream's types, met as dotty's `Denotation.meet` meets
// them: one symbol through two instances meets its information, an overloaded name meets its
// matching alternatives and keeps the others, an ascription is the prefix, a branching
// expression's union too.
def same(x: G[Any] & G[String]): String = x.get
def overInt(x: Over & Under): String = x.g(1)
def overString(x: Over & Under): String = x.g("s")
def ascribed(flag: Boolean, a: PAB, b: PBA): Int = ((if flag then a else b): PA & PB).n
def joined(flag: Boolean, a: PAB, b: PBA): Int = (if flag then a else b).n

@main def run(): Unit =
  println(same((new GS).asInstanceOf[G[Any] & G[String]]).length)
  val ou = new OverUnder
  println(overInt(ou).toUpperCase)
  println(overString(ou))
  println(ascribed(true, new PAB, new PBA) + ascribed(false, new PAB, new PBA))
  println(joined(false, new PAB, new PBA))
