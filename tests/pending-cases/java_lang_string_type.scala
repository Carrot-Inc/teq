// `java.lang.String` written as a type: scalac 3.8.4
// resolves it (Predef's `String` is its alias).
class C:
  def f: java.lang.String = "a"
@main def run(): Unit = println(new C().f)
