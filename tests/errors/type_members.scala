// expect: 23:18: error: type mismatch: found Container.this.Elem, required Int
// expect: 24:20: error: type mismatch: found Int, required Container.this.Elem
// expect: 26:8: error: illegal cyclic type reference: upper bound Cyclic.this.T of type T refers back to the type itself
// expect: 28:8: error: illegal cyclic type reference: upper bound Mutual.this.Y of type X refers back to the type itself
// expect: 39:19: error: type mismatch: found c.Elem, required Int
// expect: 41:22: error: type mismatch: found c.Elem, required d.Elem
// expect: 43:13: error: (v : Container) is not a valid type prefix, since it is not an immutable path
// expect: 44:13: error: (make : => Container) is not a valid type prefix, since it is not an immutable path
// expect: 45:19: error: type mismatch: found Container#Elem, required Int
// expect: 47:19: error: type mismatch: found c.Elem, required Int
// expect: 48:22: error: type mismatch: found String, required c.Elem
// expect: 49:46: error: type mismatch: found Names, required Container{type Elem = Int}
// expect: 50:46: error: type mismatch: found Container{type Elem = String}, required Container{type Elem = Int}
// expect: 51:56: error: type mismatch: found A, required (a : A)
// expect: 53:23: error: type mismatch: found (a2 : A), required (a : A)
// expect: 54:45: error: type mismatch: found Container#Elem, required b.Elem
// expect: 16 errors found

trait Container:
  type Elem
  def first: Elem
  def all: List[Elem]
  def bad: Int = first
  def bad2: Elem = 1
trait Cyclic:
  type T <: T
trait Mutual:
  type X <: Y
  type Y <: X
class Names extends Container:
  type Elem = String
  def first: Elem = "a"
  def all: List[Elem] = List("a")
def make: Container = Names()
class A

@main def run(): Unit =
  val c: Container = Names()
  val bad3: Int = c.first
  val d: Container = Names()
  val bad4: d.Elem = c.first
  var v: Container = c
  val bad5: v.Elem = v.first
  val bad6: make.Elem = make.first
  val bad7: Int = make.first
  def pick(x: Container): x.Elem = x.first
  val bad8: Int = pick(c)
  val bad9: c.Elem = Names().first
  val bad10: Container { type Elem = Int } = Names()
  val bad11: Container { type Elem = Int } = new Container { type Elem = String; def first = "s"; def all = Nil }
  val bad12: A = A(); val a = A(); val bad13: a.type = A()
  val a2 = A()
  val bad14: a.type = a2
  val b: Container = c; val bad15: b.Elem = a.asInstanceOf[Container].first
