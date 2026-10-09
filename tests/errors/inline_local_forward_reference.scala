// expect: 12:15: error: forward reference to source (defined on line 13) extends over the definition of first (on line 12)
// expect: 19:11: error: forward reference to box extends over the definition of box (on line 20)
// expect: 2 errors found
// A local inline method called before its definition, whose body reads an import from a val,
// a lazy val or a given the block defines between the call and the definition, reads that
// binding before it is defined: scalac 3.8.4's E039 (reported at the reference in the body,
// teq's at the call). A call after the definition is the definition's.
class Box:
  val k = 2

def one(): Int =
  val first = g()
  lazy val source = new Box
  import source.k
  inline def g(): Int = k
  first + g()

def two(): Unit =
  println(f())
  val box = new Box
  import box.k
  inline def f(): Int = k

@main def run(): Unit = println(one())
