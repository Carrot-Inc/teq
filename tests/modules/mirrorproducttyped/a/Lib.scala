package mpta

import scala.deriving.Mirror

// scala-library's `fromProductTyped` (`Mirror.scala`), its bound naming the receiver and its using
// clause the argument's mirror refined by the receiver's element types, in a body the products
// carry; the downstream calls it over classes of its own (a mirror of an upstream class it
// summons is the mirrors part's, tests/modules/known.txt's downstreammirror).
case class Q(a: Int, b: String)
case class R(n: Int, s: String)

object Convert:
  def toQ(r: R): Q = summon[Mirror.ProductOf[Q]].fromProductTyped(r)
