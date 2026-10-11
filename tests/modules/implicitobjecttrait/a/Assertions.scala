package lib

// A library's trait with an implicit object (ScalaTest's `Assertions` and its
// `UseDefaultAssertions`), extended downstream.
trait Assertions:
  implicit object UseDefault:
    def name = "default"
  def check(implicit use: UseDefault.type): String = "ok " + use.name
  inline def icheck(inline x: Boolean)(implicit use: UseDefault.type): String = "ok " + x + " " + use.name
