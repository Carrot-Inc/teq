// expect: 'this' expected, but another expression found
// expect: '=' expected, but ':' found
class A(x: Int):
  def this(s: String) = { println(1); this(s.length) }
class F(x: Int):
  def this(s: String): Unit = this(s.length)
