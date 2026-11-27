object CP {
 def run(): Int = {
  val a = 100
  class C(a: Int)(val b: Int = a + 8)
  new C(2)().b
 }
}
