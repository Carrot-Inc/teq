package mltb

import mlta.*

// The fixing trait wins and the keeping trait holds no m(int): no forwarder of it.
class KeepThenFix extends Keep[Int] with Fix
// The keeping trait wins and the fixing trait's bridge m(Object) meets its m(Object): the class forwards it.
class FixThenKeep extends Fix with Keep[Int]

@main def run(): Unit =
  val a = new KeepThenFix
  println(a.m(1) + (a: Keep[Int]).m(2) + (a: Fix).m(3) + (a: A[Int]).m(4))
  val b = new FixThenKeep
  println(b.m(5) + (b: Keep[Int]).m(6) + (b: Fix).m(7) + (b: A[Int]).m(8))
