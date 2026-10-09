package tam

import taa.*
import tab.{given, *}

@main def run(): Unit =
  println(Use.read(Impl))
  println(Impl.twice)
  println(Use.label(3))
  println((new Unused { def never(x: String) = x + "!" }).never("called"))
