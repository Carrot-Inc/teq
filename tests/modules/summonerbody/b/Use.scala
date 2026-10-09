package mgb

import mga.*
import mga.DatabaseUtils.given

@main def run(): Unit =
  println(summon[Meta[Uri]].get.read("x"))
  println(summon[Get[Uri]].read("z"))
  println(InlineUtils.metaUri.put.write(Uri("y")))
