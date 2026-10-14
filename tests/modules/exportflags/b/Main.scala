package xflagsc

import xflags.*

@main def run(): Unit =
  val v: T = 1
  println(f(v))
  println(new Wrap(new Impl).f(v))
  println(n + new Wrap(impl).n)
  println(xflagsm.Inspect.flags)
