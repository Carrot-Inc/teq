object Lib {
  def test(): Int = {
    class C {
      extension (x: Int)
        def m(y: Int = 7): Int = x + y
      def call(): Int = 2.m()
    }
    new C().call()
  }
}
