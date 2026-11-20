// A trait's stored given implementing another trait's method in the class that mixes both: the call
// through the method's trait reads it through its accessor.
trait T { def f: Int }
trait O { given f: Int = 2 }
class C extends T with O
object D extends T with O

@main def run(): Unit =
  val c = C()
  println("C " + c.f)
  println("T " + (c: T).f)
  println("O " + (c: O).f)
  println("object " + (D: T).f)
  println("anonymous " + (new T with O {}: T).f)
