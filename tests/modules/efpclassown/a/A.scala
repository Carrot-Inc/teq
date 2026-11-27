trait API { def m(a: Int)(b: Int = a + 7): Int }
object CO {
 def make(): API = {
  trait T extends API { override def m(a: Int)(b: Int = a + 8): Int = a + b }
  class C extends T { override def m(a: Int)(b: Int = a + 9): Int = a + b }
  new C
 }
}
