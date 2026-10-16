package inlineuse

import inlineabi.Factory
import inlinelib.Lib

object Main:
  def main(args: Array[String]): Unit = println(List(Lib.byInline(Factory.make()), Lib.byValue(Factory.make())))
