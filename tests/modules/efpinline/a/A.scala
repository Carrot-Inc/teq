object IL {
 inline def run(): Int = {
  class C { def m(a: Int)(b: Int = a + 8): Int = a + b }
  new C().m(2)()
 }
}
