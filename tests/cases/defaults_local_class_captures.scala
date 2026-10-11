// A local class's constructor defaults read the locals it captures; where a later clause's effect
// keeps an earlier default's getter application in front of it, the getter is called with the
// captures, a captured var's cell too, as the constructor takes them.
def mark(s: String): Int = { print(s); 1 }

def vals(): Unit =
  val k = 10
  class C(a: Int = k + mark("A"), b: Int)(c: Int = a + k):
    override def toString = s"C($a,$b,$c)"
  println(new C(b = mark("B"))())
  println(new C(b = 2)(c = mark("c")))

def vars(): Unit =
  var n = 1
  class V(a: Int = { n += 1; n }, b: Int)(c: Int = a * 10):
    override def toString = s"V($a,$b,$c)"
  println(new V(b = mark("B"))())
  println(new V(b = 2)(c = mark("c")))
  n = 7
  println(new V(b = 3)(c = mark("d")))
  println(n)

@main def main(): Unit =
  vals()
  vars()
