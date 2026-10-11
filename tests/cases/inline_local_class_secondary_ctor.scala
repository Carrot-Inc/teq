// A local class an inline body makes, with secondary constructors that read the method's parameter:
// the expansion's copy of the class has its constructors, and each construction through one is the
// copy's (dotty's `TreeTypeMap` maps a class's constructors with its other members).
inline def make(n: Int): Int =
  class C(val v: Int):
    def this() = this(n)
    def get: Int = v
  new C().get
inline def make2(n: Int): String =
  class D(val v: Int, val w: String):
    def this(s: String) = this(n, s)
    def this() = this("d")
    def get: String = w + v
  new D().get + new D("e").get + new D(1, "f").get
@main def run(): Unit =
  println(make(7))
  println(make2(4))
