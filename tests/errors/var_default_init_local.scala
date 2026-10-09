// `var x: T = _` is for fields: a local variable cannot be left to its default (scala3's
// neg/t2368).
// expect: Unbound placeholder parameter; incorrect use of _
class C:
  def foo(): Unit =
    var x: String = _
    println(x)
