// An annotation's arguments with what they take: an implicit argument, a conditional.
object Lib {
  class Ann(x: Int) extends scala.annotation.StaticAnnotation
  given n: Int = 3
  def value(using x: Int): Int = x
  val N = 3
}
import Lib.{Ann, n, value, N}
@Ann(value) class One
@Ann(if true then N else 0) class Two

import scala.collection.mutable.Stack

// teq: --werror --wunused imports
// expect: unused_imports_annotation_args.scala:12:33: warning: unused import
