// `substring` and `codePointAt` outside the string throw `StringIndexOutOfBoundsException` with
// the JVM's message, as `charAt` does.
object Main:
  def attempt(label: String)(f: => Any): Unit =
    try println(label + " = " + f)
    catch case e: StringIndexOutOfBoundsException => println(label + " threw " + e.getClass.getName + ": " + e.getMessage)
  def main(args: Array[String]): Unit =
    attempt("sub01")("".substring(0, 1))
    attempt("sub1")("".substring(1))
    attempt("sub-1")("abc".substring(-1))
    attempt("sub21")("abc".substring(2, 1))
    attempt("sub04")("abc".substring(0, 4))
    attempt("cpa0")("".codePointAt(0).toLong)
    attempt("cpa3")("abc".codePointAt(3))
    attempt("cpa-1")("abc".codePointAt(-1))
    attempt("charAt3")("abc".charAt(3))
    attempt("sub33")("abc".substring(3, 3))
    attempt("sub3")("abc".substring(3))
    attempt("sub12")("abc".substring(1, 2))
    attempt("cpa1")("a𝄞".codePointAt(1))
    attempt("take")("abc".take(5) + "abc".drop(-1) + "abc".take(-1))
