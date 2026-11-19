package uob

import uoa.*

class C extends B with A
class C2 extends B2 with A2

@main def run(): Unit =
  println(C().f)
  println(C2().f)
