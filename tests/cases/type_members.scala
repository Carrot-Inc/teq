// Abstract type members, bounded and higher-kinded, path-dependent types, refinements and
// singleton paths, checked against scalac's answers.
trait Store:
  type Key
  type Value <: AnyRef
  def put(k: Key, v: Value): Unit
  def get(k: Key): Option[Value]
  def keys: List[Key]

class Names extends Store:
  type Key = Int
  type Value = String
  private var items: Map[Int, String] = Map.empty
  def put(k: Key, v: Value): Unit = items = items + (k -> v)
  def get(k: Key): Option[Value] = items.get(k)
  def keys: List[Key] = items.keys.toList.sorted

trait Codec:
  type Repr
  def encode(s: String): Repr
  def decode(r: Repr): String
  def roundTrip(s: String): String = decode(encode(s))

object Upper extends Codec:
  type Repr = String
  def encode(s: String): Repr = s.toUpperCase
  def decode(r: Repr): String = r.toLowerCase

trait Graph:
  type Node
  type Edge <: (Node, Node)
  def nodes: List[Node]
  def edges: List[Edge]
  def degree(n: Node): Int = edges.count(e => e._1 == n || e._2 == n)

class Triangle extends Graph:
  type Node = Char
  type Edge = (Char, Char)
  def nodes: List[Node] = List('a', 'b', 'c')
  def edges: List[Edge] = List(('a', 'b'), ('b', 'c'), ('c', 'a'))

trait Wrapper:
  type F[_]
  def wrap[A](a: A): F[A]
  def size[A](fa: F[A]): Int

class ListWrapper extends Wrapper:
  type F[A] = List[A]
  def wrap[A](a: A): F[A] = List(a, a)
  def size[A](fa: F[A]): Int = fa.length

class OptionWrapper extends Wrapper:
  type F = Option
  def wrap[A](a: A): F[A] = Some(a)
  def size[A](fa: F[A]): Int = fa.size

def firstKey(s: Store): Option[s.Key] = s.keys.headOption

def stored(s: Store)(k: s.Key): Option[s.Value] = s.get(k)

def wrapTwice(w: Wrapper)(n: Int): w.F[w.F[Int]] = w.wrap(w.wrap(n))

trait Ops[A]:
  type TC
  def value: A
  def tc: TC

type Aux[A, T0] = Ops[A] { type TC = T0 }

def opsOf[A](a: A, label: String): Aux[A, String] = new Ops[A]:
  type TC = String
  def value: A = a
  def tc: TC = label

def describe(o: Ops[Int] { type TC = String }): String = o.tc + "=" + o.value

class Counter(val start: Int):
  def same(other: this.type): Boolean = other eq this
  def self: this.type = this

trait Registry:
  type Entry
  val entries: List[Entry]
  def show(e: Entry): String
  def all: String = entries.map(show).mkString(",")

@main def main(): Unit =
  val names = Names()
  names.put(2, "two")
  names.put(1, "one")
  println(names.keys)
  println(names.get(1))
  val store: Store = names
  val k: store.Key = store.keys.head
  println(store.get(k))
  println(firstKey(store))
  println(stored(names)(2))
  val fk: Option[store.Key] = firstKey(store)
  println(fk.map(x => store.get(x)))
  println(Upper.roundTrip("Mixed"))
  val enc: Upper.Repr = Upper.encode("abc")
  println(enc)
  val tri = Triangle()
  println(tri.degree('a'))
  val g: Graph = tri
  println(g.edges.map(e => g.degree(e._1)))
  val lw = ListWrapper()
  println(lw.size(lw.wrap(3)))
  val w: Wrapper = lw
  val ww: w.F[w.F[Int]] = wrapTwice(w)(7)
  println(w.size(ww))
  println(wrapTwice(OptionWrapper())(1))
  val ops = opsOf(41, "answer")
  val label: String = ops.tc
  println(label + ops.value)
  println(describe(ops))
  val plain: Ops[Int] = ops
  val t: plain.TC = plain.tc
  println(t)
  val c = Counter(5)
  println(c.same(c))
  println(c.self.start)
  val reg = new Registry:
    type Entry = (String, Int)
    val entries: List[Entry] = List(("a", 1), ("b", 2))
    def show(e: Entry): String = e._1 + e._2
  println(reg.all)
  val e: reg.Entry = ("c", 3)
  println(reg.show(e))
  val proj: Names#Key = 9
  println(proj + 1)
