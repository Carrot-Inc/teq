package gpa

trait Show[A]:
  def show(a: A): String

trait Base:
  given sized(using n: Int): Show[String] with
    def show(a: String): String = a.take(n)
  def inside: String = sized(using 1).show("abc")

object Plain extends Base
