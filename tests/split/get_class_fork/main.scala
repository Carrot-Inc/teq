package getclassfork

import getclassfork.box.Box

object Main:
  def main(args: Array[String]): Unit =
    println(Box.primitive(1) + " " + Box.reference(Box()) + " " + Box.widened(2L))
