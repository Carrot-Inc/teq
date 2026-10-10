package dpb

import dpa.*

def poly(x: C & R): String = x.f("ok")

@main def run(): Unit =
  println(poly(new Both))
