// An inner class's own `==` and `!=` are found where the class is named through a path, the parameter's type
// seen from the path: through a value, a value import, the class's own inner object, a subclass's `this`, a local
// class's instance; an inherited one and a generic class's too. Where the path's instances differ, where the
// prefix is a type's projection, and where the class's operator may not be accessed from the site (private,
// protected outside a subclass, a qualified private outside its scope), equality is `Any`'s; of an overloaded
// operator the accessible alternative that takes the operand is the one.
class Ops(val seed: Int):
  class Tag(val t: Int)
  object nums:
    class Money(val n: Int) { def ==(o: Money): Boolean = n == o.n; def !=(o: Money): Boolean = n != o.n }
    class Coin(val n: Int) { def ==(o: Coin): Boolean = n == o.n }
    class Counter(val n: Int) { def ==(o: Int): Boolean = n == o; def !=(o: Int): Boolean = n != o }
    class Note(val n: Int) { override def equals(o: Any): Boolean = o match { case x: Note => x.n == n; case _ => false }; override def hashCode: Int = n }
    class Base(val k: Int) { def ==(o: Base): Boolean = k == o.k }
    class Derived(k: Int) extends Base(k)
    class Cell[T](val get: T) { def ==(o: Cell[T]): Boolean = get == o.get }
    class Keyed(val n: Int) { def ==(t: Tag): Boolean = n == t.t }
    class Priv(val n: Int) { private def ==(o: Priv): Boolean = true; private def !=(o: Priv): Boolean = false; def inside(o: Priv): String = s"${this == o} ${this != o}" }
    class Prot(val n: Int) { protected def ==(o: Prot): Boolean = true; def inside(o: Prot): String = s"${this == o}" }
    class Below(n: Int) extends Prot(n) { def own(o: Below): String = s"${this == o}"; def other(o: Below, p: Below): String = s"${o == p}" }
    class Scoped(val n: Int) { private[nums] def ==(o: Scoped): Boolean = true }
    class Mixed(val n: Int) { private def ==(o: Mixed): Boolean = true; def ==(o: Int): Boolean = n == o }
  def coin: nums.Money = new nums.Money(1)
  def inside(): String = s"${new nums.Money(1) == new nums.Money(1)} ${new nums.Money(1) != new nums.Money(1)}"
  def across(other: Ops): Boolean = new nums.Money(1) == new other.nums.Money(1)
  class Price(val n: Int) { def ==(o: Price): Boolean = n == o.n; def !=(o: Price): Boolean = n != o.n }

class Sub extends Ops(5):
  def own(): Boolean = new Price(1) == new Price(1)

@main def run(): Unit =
  val o = new Ops(2)
  val p = new Ops(10)
  println(s"${new o.nums.Money(1) == new o.nums.Money(1)} ${new o.nums.Money(1) != new o.nums.Money(1)}")
  println(s"${new o.nums.Coin(1) == new o.nums.Coin(1)} ${new o.nums.Coin(1) != new o.nums.Coin(1)}")
  println(s"${new o.nums.Counter(3) == 3} ${new o.nums.Counter(3) != 3}")
  println(s"${new o.nums.Note(1) == new o.nums.Note(1)} ${new o.nums.Note(1) != new o.nums.Note(1)}")
  println(s"${new o.nums.Derived(1) == new o.nums.Derived(1)} ${new o.nums.Cell(1) == new o.nums.Cell(1)}")
  println(s"${new o.nums.Keyed(1) == new o.Tag(1)} ${new o.nums.Keyed(1) == new p.Tag(1)}")
  val a = new o.nums.Money(1)
  val any: Any = new o.nums.Money(1)
  println(s"${a == any} ${new o.nums.Money(1) == new p.nums.Money(1)} ${new o.nums.Money(1) == new o.nums.Money(1)}")
  locally:
    import o.nums.*
    println(s"${new Money(1) == new Money(1)} ${new Money(1) != new Money(2)}")
    println(s"${new Priv(1) == new Priv(1)} ${new Priv(1) != new Priv(1)} ${new Priv(1).inside(new Priv(2))}")
    println(s"${new Prot(1) == new Prot(1)} ${new Prot(1).inside(new Prot(2))}")
    println(s"${new Below(1) == new Below(1)} ${new Below(1).own(new Below(2))} ${new Below(1).other(new Below(2), new Below(3))}")
    println(s"${new Scoped(1) == new Scoped(1)} ${new Mixed(3) == 3} ${new Mixed(3) == new Mixed(3)}")
  println(s"${o.inside()} ${o.across(p)} ${new Sub().own()}")
  println(s"${o.coin == o.coin} ${new Ops(2).coin == new Ops(2).coin}")
  val x: Ops#Price = new o.Price(1)
  val y: Ops#Price = new p.Price(1)
  println(s"${x == y} ${x != y} ${new o.Price(1) == new o.Price(1)}")
  class Scope:
    class Leaf(val v: Int) { def ==(l: Leaf): Boolean = v == l.v }
  val s = new Scope
  println(new s.Leaf(1) == new s.Leaf(1))
