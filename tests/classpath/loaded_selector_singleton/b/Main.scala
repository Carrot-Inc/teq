package lss.use

import lss.lib.{Box, Holder, Lib}

@main def run(): Unit =
  val full = Box(Holder(Some(("ab", 1))))
  val empty = Box(Holder(None))
  println(Lib.both(full))
  println(Lib.size(full))
  println(Lib.size(empty))
