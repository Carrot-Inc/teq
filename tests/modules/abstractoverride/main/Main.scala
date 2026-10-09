package aom

import aoa.*

class Stacked extends Impl with Plus with Loud

@main def run(): Unit =
  println(new Stacked().f)
  println(new Stacked().name)
  println((new Impl with Plus).f)
  println(aob.Use.n)
