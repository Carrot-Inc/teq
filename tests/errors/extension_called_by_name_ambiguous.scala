// expect: 11:33: error: reference to tag is ambiguous: it is both imported by import A.* and imported subsequently by import B.*
// Two wildcard imports of one scope that each bring an extension method called `tag`: named as a
// method, the reference is ambiguous, as for any two terms of one name (scalac's E049, which adds
// that the extension may be called as a normal method).
object A:
  extension (n: Int) def tag: String = "a"
object B:
  extension (s: String) def tag: String = "b"
import A.*
import B.*
@main def run(): Unit = println(tag(1))
