#!/usr/bin/env -S teq interp
// After a script header and comments, `//> using files` includes several sources: one word each,
// or a quoted string, white space or commas between them, which Scala CLI's parser warns of (teq's warning first).
//> using files lib/Helper.scala, "lib/deep/Other.scala"
object Main:
  def main(args: Array[String]): Unit = println(Helper.value + ", " + Other.value)
