package rrb

import rra.*

object UseRepeated:
  val p: Parent { def rep(xs: Int*): Int; def gen[A]: A } = new Parent { def rep(xs: Int*): Int = xs.sum; def gen[A]: A = null.asInstanceOf[A] }
  val kept: Parent { def rep(xs: Int*): Int; def gen[A]: A } = Repeated.keep(p)
  def main(args: Array[String]): Unit = println(kept.rep(1, 2, 3))
