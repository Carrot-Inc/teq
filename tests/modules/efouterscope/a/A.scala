trait API {
  def m(a: Int)(b: Int = a + 8): Int
}
object Lib {
  def make(x: Int): API = {
    class O(val v: Int) {
      def get(): API = {
        trait T extends API {
          override def m(a: Int)(b: Int = a + 8): Int =
            a + b + O.this.v
        }
        class C extends T
        new C
      }
    }
    new O(x).get()
  }
}
