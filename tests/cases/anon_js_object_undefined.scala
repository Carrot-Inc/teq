//> using platform js
//> using jsVersion 1.22.0

import scala.scalajs.js

// A trait's js.undefined vals are absent from an anonymous object until the body gives or a
// later statement assigns them, as Scala.js leaves its optional members.
trait OptionsVal extends js.Object:
  val x: js.UndefOr[Int] = js.undefined
  val y: String

trait OptionsVar extends js.Object:
  var x: js.UndefOr[Int] = js.undefined
  val y: String

trait Base extends js.Object:
  val shared: js.UndefOr[String] = js.undefined

trait T1 extends Base:
  val a: String

trait T2 extends Base:
  var v: js.UndefOr[Int] = js.undefined
  val b: String

trait Independent1 extends js.Object:
  val common: js.UndefOr[String] = js.undefined
  val a: String

trait Independent2 extends js.Object:
  val common: js.UndefOr[String] = js.undefined
  val b: String

trait Gen[A] extends js.Object:
  val item: A
  val opt: js.UndefOr[A] = js.undefined

trait CustomAttr extends js.Object:
  val `data-cy`: js.UndefOr[String] = js.undefined
  val `aria-label`: String

trait KeyOrderTrait extends js.Object:
  val first: String
  val second: String
  var laterStr: js.UndefOr[String] = js.undefined
  var `2`: js.UndefOr[String] = js.undefined
  var `10`: js.UndefOr[String] = js.undefined

trait InterleavedTrait extends js.Object:
  var x: js.UndefOr[Int] = js.undefined
  val y: String

trait ProtoNamed extends js.Object:
  var x: js.UndefOr[Int] = js.undefined
  val `__proto__`: Int

trait TwoOfEach extends js.Object:
  var x: js.UndefOr[Int] = js.undefined
  var z: js.UndefOr[Int] = js.undefined
  val y: String
  val w: String

var trace: List[String] = Nil
def f(): Int = { trace = trace :+ "f"; 1 }
def g(): String = { trace = trace :+ "yVal"; "y" }
def note[A](s: String, a: A): A = { trace = trace :+ s; a }

@main def main(): Unit =
  val p1 = new OptionsVal { val y = "hello" }
  println(s"p1 in: ${js.special.in("x", p1)}")
  println(s"p1 keys: ${js.Object.keys(p1).mkString(",")}")
  println(s"p1 json: ${js.JSON.stringify(p1)}")

  val p2a = new OptionsVal {
    override val x = js.undefined
    val y = "hello"
  }
  println(s"p2a in: ${js.special.in("x", p2a)}")
  println(s"p2a keys: ${js.Object.keys(p2a).mkString(",")}")
  println(s"p2a json: ${js.JSON.stringify(p2a)}")

  val p2b = new OptionsVar { val y = "world" }
  p2b.x = js.undefined
  println(s"p2b in: ${js.special.in("x", p2b)}")
  println(s"p2b keys: ${js.Object.keys(p2b).mkString(",")}")
  println(s"p2b json: ${js.JSON.stringify(p2b)}")

  val p2c = new OptionsVar {
    x = js.undefined
    val y = "again"
  }
  println(s"p2c in: ${js.special.in("x", p2c)}")
  println(s"p2c keys: ${js.Object.keys(p2c).mkString(",")}")
  println(s"p2c json: ${js.JSON.stringify(p2c)}")

  val o1 = new T1 with T2 {
    val a = "aVal"
    val b = "bVal"
  }
  println(s"o1 initial in shared: ${js.special.in("shared", o1)}")
  println(s"o1 initial in v: ${js.special.in("v", o1)}")
  println(s"o1 initial keys: ${js.Object.keys(o1).mkString(",")}")
  println(s"o1 initial json: ${js.JSON.stringify(o1)}")

  o1.v = 42
  println(s"o1 after v in: ${js.special.in("v", o1)}")
  println(s"o1 after v keys: ${js.Object.keys(o1).mkString(",")}")
  println(s"o1 after v json: ${js.JSON.stringify(o1)}")

  val o2 = new Independent1 with Independent2 {
    override val common = js.undefined
    val a = "a2"
    val b = "b2"
  }
  println(s"o2 in common: ${js.special.in("common", o2)}")
  println(s"o2 keys: ${js.Object.keys(o2).mkString(",")}")
  println(s"o2 json: ${js.JSON.stringify(o2)}")

  val gen = new Gen[Int] { val item = 100 }
  println(s"gen opt in: ${js.special.in("opt", gen)}")
  println(s"gen keys: ${js.Object.keys(gen).mkString(",")}")
  println(s"gen json: ${js.JSON.stringify(gen)}")

  val custom = new CustomAttr { val `aria-label` = "test-button" }
  println(s"custom data-cy in: ${js.special.in("data-cy", custom)}")
  println(s"custom keys: ${js.Object.keys(custom).mkString(",")}")
  println(s"custom json: ${js.JSON.stringify(custom)}")

  val dyn = js.Dynamic.literal(foo = "bar", baz = js.undefined)
  println(s"dyn baz in: ${js.special.in("baz", dyn)}")
  println(s"dyn keys: ${js.Object.keys(dyn).mkString(",")}")
  println(s"dyn json: ${js.JSON.stringify(dyn)}")

  // Keys in insertion order, the integer-like ones first.
  val p5 = new KeyOrderTrait {
    val first = "1st"
    val second = "2nd"
  }
  println(s"p5 initial keys: ${js.Object.keys(p5).mkString(",")}")
  p5.laterStr = "later"
  println(s"p5 after laterStr keys: ${js.Object.keys(p5).mkString(",")}")
  p5.`2` = "two"
  println(s"p5 after '2' keys: ${js.Object.keys(p5).mkString(",")}")
  p5.`10` = "ten"
  println(s"p5 after '10' keys: ${js.Object.keys(p5).mkString(",")}")

  // The body's vals take their keys before its assignments to optional vars, while every
  // initialiser runs in source order.
  val p6a = new InterleavedTrait {
    x = 1
    val y = "y"
  }
  println(s"p6a keys: ${js.Object.keys(p6a).mkString(",")}")

  trace = Nil
  val p6b = new InterleavedTrait {
    x = f()
    val y = g()
  }
  println(s"p6b side-effects: ${trace.mkString(",")}")
  println(s"p6b keys: ${js.Object.keys(p6b).mkString(",")}")

  trace = Nil
  val twice = new TwoOfEach { x = note("f", 1); val y = note("g", "y"); z = note("h", 3); val w = note("k", "w") }
  println(s"twice side-effects: ${trace.mkString(",")}")
  println(s"twice keys: ${js.Object.keys(twice).mkString(",")}")
  println(s"twice json: ${js.JSON.stringify(twice)}")

  // A val named __proto__ sets no property, as under Scala.js, and holds no key ahead of the assignment.
  val proto = new ProtoNamed { x = 1; val `__proto__` = 2 }
  println(s"proto keys: ${js.Object.keys(proto).mkString(",")}")
  println(s"proto json: ${js.JSON.stringify(proto)}")
