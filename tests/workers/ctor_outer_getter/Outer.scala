class Outer(val x: Int):
  class Inner(val a: Int):
    def this(s: String, k: Int = x) = this(s.length + k)
