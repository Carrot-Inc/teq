// `import language.strictEquality` switches strict equality on for its file: a type
// parameter cannot be compared with null or a String, as scalac reports.
import language.strictEquality
object Main:
  def f[T](x: T): Int =
    if x == null then 1
    else if x == "abc" then 2
    else 3
// expect: values of types T and Null cannot be compared with == or !=
// expect: values of types T and String cannot be compared with == or !=
