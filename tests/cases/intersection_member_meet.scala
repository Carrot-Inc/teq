// A member both parts of an intersection declare with results neither conforming to the other is
// one member of the meet of the results (dotty's `Denotation.meet`, `infoMeet`): `x.value` of an
// `A & B` is a `Product & Serializable`, called once on the instance, whichever part comes first.
// The member is merged where it is looked up: applied, partly applied, expanded to a function or
// generic, its binders aligned; of an overloaded part, the alternative matching it (`mergeDenot`
// into a `MultiDenotation`), the other alternative left as it is; of two generic ones where one's
// information overrides the other's (its bounds the wider, its result conforming), that one whole.
trait A:
  def value: Product
  def pick(i: Int): Product
  def pair(i: Int)(s: String): Product
  def poly[T](t: T): Product
trait B:
  def value: Serializable
  def pick(i: Int): Serializable
  def pair(i: Int)(s: String): Serializable
  def poly[T](t: T): Serializable
class C(val n: Int) extends A with B with Product with Serializable:
  def value: Product & Serializable = this
  def pick(i: Int): Product & Serializable = C(n + i)
  def pair(i: Int)(s: String): Product & Serializable = C(n + i + s.length)
  def poly[T](t: T): Product & Serializable = C(n + t.toString.length)
  def canEqual(that: Any) = that.isInstanceOf[C]
  def productArity = 1
  def productElement(i: Int): Any = n
  override def toString = s"C($n)"

def f(x: A & B): Product & Serializable = x.value
def g(x: A & B): Product & Serializable = x.pick(2)
def h(x: B & A): Serializable & Product = x.value
def eta(x: A & B): Int => Product & Serializable = x.pick
def partial(x: A & B): String => Product & Serializable = x.pair(1)
def generic(x: A & B): Product & Serializable = x.poly[String]("ab")
trait O1:
  def over(i: Int): Product
trait O2:
  def over(i: Int): Serializable
  def over(s: String): String
def overloaded(x: O1 & O2): Product & Serializable = x.over(1)
def overEta(x: O1 & O2): Int => Product & Serializable = x.over
trait W1:
  def wide[T <: AnyRef](t: T): Int
trait W2:
  def wide[T <: Product](t: T): Int
// Typed, not called: no class implements both.
def wider(x: W1 & W2): Int = x.wide("ok")
class D extends O1 with O2:
  def over(i: Int): Product & Serializable = C(i)
  def over(s: String): String = s + "!"

@main def run() =
  println(f(C(1)))
  println(g(C(1)))
  println(h(C(3)))
  val s: Serializable = f(C(4))
  println(s)
  println(f(C(5)).productArity)
  println(eta(C(1))(5))
  println(partial(C(1))("xyz"))
  println(generic(C(1)))
  println(overloaded(D()))
  println((D(): O1 & O2).over("s"))
  println(overEta(D())(2))
