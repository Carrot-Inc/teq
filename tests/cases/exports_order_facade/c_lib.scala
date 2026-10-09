package lib

trait Show[A]:
  def show(a: A): String

trait Ops:
  extension (i: Int) def twice: Int = i * 2

trait Labels:
  def tag: String
  var count: Int = 0
  given Show[Int] = a =>
    count += 1
    s"$tag$a"

object Impl extends Ops, Labels:
  def tag: String = "impl:"

trait TagsBase:
  def main: String = "main-tag"
  def cls: String = "cls"
  def named(s: String): String = s"<$s>"

object Tags extends TagsBase
