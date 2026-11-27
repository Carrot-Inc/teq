object Lib {
  def test(): Int = {
    class C {
      def m(x: Int): Int = x
      def m$default$1: Int = 42
    }
    new C().m$default$1
  }
}
