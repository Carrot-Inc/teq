// `java.lang.String` written as a type: scalac 3.8.4 resolves it (Predef's `String` is its alias), teq reports "type
// String not found" in any position (`java.lang.Integer` resolves).
class C:
  def f: java.lang.String = "a"
@main def run(): Unit = println(new C().f)
