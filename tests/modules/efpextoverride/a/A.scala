trait EAPI { extension (x: Int) def m(y: Int = 7): Int }
object Ext {
 def make(): EAPI = {
  class C extends EAPI { extension (x: Int) override def m(y: Int = 8): Int = x + y }
  new C
 }
}
