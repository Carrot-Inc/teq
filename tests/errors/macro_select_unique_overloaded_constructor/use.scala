// expect: use.scala:8:33: error: exception in macro: java.lang.AssertionError: assertion failed: The symbol `<init>` is overloaded. The method Select.unique can only be used for non-overloaded symbols.
// expect: 1 error found
// `Select.unique` asserts that the member is not overloaded (`QuotesImpl`), and a class with a second
// constructor's `<init>` is: scalac's expansion fails with this assertion.
class Two(val s: String):
  def this(n: Int) = this(n.toString)

@main def run(): Unit = println(make[Two]("abc").s)
