// jars: fixtures scala-library
// The fixtures of tests/tasty as a jar (Circle and Empty of Shape are not among them): givens
// and Scala 2 implicits of a companion, enum cases, patterns over case classes of a jar,
// opaque types, extension methods, exports, top-level definitions of a package object; a cake
// whose platform trait implements an abstract type member beside an object of the same name.
import fix.shapes.*
import fix.members.{Registry, Holder}
import fix.rassoc.{Vec, RightAssoc}
import fix.cake.Cake

object Main:
  def area(s: Shape): Double = s match
    case Rect(w, h) => w * h
    case _ => 0.0

  def rgb(c: Color): Int = c match
    case Color.Red => 1
    case Color.Green => 2
    case Color.Custom(v) => v

  def size[A](t: Tree[A]): Int = t match
    case Tree.Leaf => 0
    case Tree.Node(l, _, r) => size(l) + 1 + size(r)

  def main(args: Array[String]): Unit =
    val shown: String = Show[Int].show(1)
    val ls: String = summon[Show[List[Int]]].show(List(1, 2))
    val os: String = summon[Show[Option[Int]]].show(Some(1))
    val ss: String = summon[Show[String]].show("s")
    val ps: String = summon[Show[(Int, String)]].show((1, "a"))
    val ex: String = Show[Int].shown(1)
    val a: Double = area(Rect(2.0)) + area(Rect(2.0, 3.0))
    val n: Int = rgb(Color.Custom(3)) + Color.Red.rgb + size(Tree.Node(Tree.Leaf, 1, Tree.Leaf))
    val t: Int = topLevel(topVal)
    val sh: String = "a".shout
    val al: Alias[Int] = Map(("k", 1))
    val e: Registry.Entry = Registry.make("k")
    val m: Opaques.Meters = Opaques.Meters(1.5)
    val v: Double = m.value
    import Opaques.*
    val sec: Int = List(1, 2).second
    val more: List[Int] = List(1) +:+ 2
    val r: Int = Exports.run(1) + Exports.c
    val box = new Box[String, List, Null]("a", 1)
    val bn: Int = box.byName(1, "x", "y")
    val bc: String = box.curried[String](x => x)((b, i) => b)
    val bt: (Int, String, Double) = box.tuple
    val u: Int = box + 1
    val bs: Int = box.size
    val plain: Option[String] = Holder.plain(List(1))
    val copied: Rect = Rect(2.0).copy(h = 4.0)
    val applied: Rect = Rect.apply(2.0)
    import RightAssoc.*
    val vec: Vec = new Vec(List(2)) +: 1
    val vec2: Vec = vec :+ 3
    val cake = new Cake
    val boxed: List[Int] = cake.twice(cake.boxed(3))
    val unboxed: Int = cake.unwrap(cake.boxed(7))
