package app

import lib.Tests

@main def run(): Unit =
  println(Tests.matchConstant("s"))
  println(Tests.matchConstant(new Object))
  println(Tests.isConstant("s"))
  println(Tests.isConstant(new Object))
  println(Tests.matchDescriptor("s"))
  println(Tests.matchDescriptor(new Object))
  println(Tests.isDescriptor("s"))
  println(Tests.isDescriptor(new Object))
