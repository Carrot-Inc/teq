#!/usr/bin/env -S teq interp
// After a script header and comments, `//> using files` includes several sources: one word each,
// or a quoted string, white space or (deprecated in Scala CLI) commas between them.
//> using files lib/Helper.scala, "lib/deep/Other.scala"
object Main:
  def main(args: Array[String]): Unit = println(Helper.value + ", " + Other.value)
