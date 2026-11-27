trait API {
  def m(a: Int)(b: Int = a + 7): Int
}
object Lib {
  def test(): Int = {
    trait T extends API {
      override def m(a: Int)(b: Int = a + 8): Int = a + b
    }
    trait U extends T
    class C extends U
    val c: API = new C
    c.m(2)()
  }
}
