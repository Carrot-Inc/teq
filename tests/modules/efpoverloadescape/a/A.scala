trait API {
 def m(a: Int = 21): Int
 def m(s: String): Int
}
object OE {
 def make(): API = {
  class C extends API {
   override def m(a: Int = 22): Int = a
   def m(s: String): Int = 99
  }
  new C
 }
}
