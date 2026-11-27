// A local class with a method of its own spelled as its default's getter (`m$default$1` taking a String): a call
// leaving the argument out runs the default, never that method; scalac prints 7 then 99.
package esc
trait API { def m(x: Int = 6): Int; def other(): Int }
object Lib {
  def make(): API = {
    class C extends API {
      override def m(x: Int = 7): Int = x
      def m$default$1(s: String): Int = 99
      def other(): Int = m$default$1("s")
    }
    new C
  }
}
