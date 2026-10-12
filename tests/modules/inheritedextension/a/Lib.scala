package iea

trait Results:
  class Status(val code: Int):
    def apply(s: String): String = s"$code $s"
  val Ok = new Status(200)
  def status(code: Int): Status = new Status(code)

trait CtrlPage extends Results:
  extension (s: Status)
    def page(p: String): String = s(p)
    def twice: Status = status(s.code * 2)

abstract class Base extends Results with CtrlPage
