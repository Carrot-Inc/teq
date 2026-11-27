trait API {
  def m(a: Int)(b: Int = a + 7): Int
}
object Lib {
  def test(): Int = {
    trait T extends API {
      override def m(a: Int)(b: Int = a + 8): Int
    }
    class C extends T {
      def m(a: Int)(b: Int): Int = a + b
    }
    val c: API = new C
    c.m(2)()
  }
}
