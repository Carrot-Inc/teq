object SC {
 def run(): Int = {
  val a = 100
  class C { def m(a: Int, b: Int = a + 8): Int = a + b }
  new C().m(2)
 }
}
