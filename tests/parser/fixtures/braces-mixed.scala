// A template whose `:` was made `{` and one whose `{` was made `:`.
object A {
  object Inner :
    val foo: Int = 42
  }
  class InnerClass(val foo: Int) {
    val bad: String = 1
  }
}
object B:
  object Inner {
    val foo: Int = 42
  object Next:
    val bad: String = 2
