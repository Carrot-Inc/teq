// An object whose parent is a class of an instance is made with that instance: its load evaluates the parent's
// prefix (the holder's load, a lazy val), and so does the load of an object with such a class further up. An
// argument bound ahead of the call and a right-associative operator's left operand are evaluated before it. A
// trait of an instance gives the object a method for its outer instance, and a class of an object has none:
// those loads run nothing. The expectation is scalac 3.8.4's.
class Outer(tag: String) {
  println("  outer " + tag + " made")
  class Inner { def ::(x: Any): Unit = println("  :: called") }
  trait Mixed { def ::(x: Any): Unit = println("  :: called") }
  object Mid { class Deep { def ::(x: Any): Unit = println("  :: called") } }
}
object StaticA { println("  StaticA loaded"); class Nested { def ::(x: Any): Unit = println("  :: called") } }
object StaticB { println("  StaticB loaded"); class Nested { def ::(x: Any): Unit = println("  :: called") } }
object H1 { println("  H1 loaded"); val o = new Outer("h1") }
object H2 { println("  H2 loaded"); val o = new Outer("h2") }
object H3 { println("  H3 loaded"); val o = new Outer("h3") }
object H4 { println("  H4 loaded"); val o = new Outer("h4") }
object H5 { println("  H5 loaded"); val o = new Outer("h5") }
object H6 { println("  H6 loaded"); val o = new Outer("h6") }
object H7 { println("  H7 loaded"); val o = new Outer("h7") }
object H8 { println("  H8 loaded"); val o = new Outer("h8") }
object H9 { lazy val o = { println("  H9's lazy val made"); new Outer("h9") } }
object H10 { println("  H10 loaded"); val o = new Outer("h10") }
object H11 { println("  H11 loaded"); val o = new Outer("h11") }
object Aliases { type Named = H10.o.Inner }
object InnerN extends H1.o.Inner
object InnerR extends H2.o.Inner
object DeepN extends H3.o.Mid.Deep
object DeepR extends H4.o.Mid.Deep
class MidN extends H5.o.Inner
object GrandN extends MidN
class MidR extends H6.o.Inner
object GrandR extends MidR
object TraitN extends H7.o.Mixed
object TraitR extends H8.o.Mixed
object LazyN extends H9.o.Inner
object AliasR extends Aliases.Named
object NestedR extends H11.o.Inner
object StaticN extends StaticA.Nested
object StaticR extends StaticB.Nested
object Main {
  def effect(tag: String): Int = { println("  effect " + tag); 1 }
  def pair(a: Any, b: Any): Unit = println("  pair called")
  def main(args: Array[String]): Unit = {
    println("01 a class of an instance: a bound argument"); pair(b = effect("b"), a = InnerN)
    println("02 the same: a right-associative operand"); effect("l") :: InnerR
    println("03 a class of an object of an instance: a bound argument"); pair(b = effect("b"), a = DeepN)
    println("04 the same: an operand"); effect("l") :: DeepR
    println("05 a class of an instance two parents up: a bound argument"); pair(b = effect("b"), a = GrandN)
    println("06 the same: an operand"); effect("l") :: GrandR
    println("07 a trait of an instance: a bound argument"); pair(b = effect("b"), a = TraitN)
    println("08 the same: an operand"); effect("l") :: TraitR
    println("09 the prefix a lazy val: a bound argument"); pair(b = effect("b"), a = LazyN)
    println("10 the parent named through an alias: an operand"); effect("l") :: AliasR
    println("11 a class of an object: a bound argument"); pair(b = effect("b"), a = StaticN)
    println("12 the same: an operand"); effect("l") :: StaticR
    println("13 an operand inside a bound argument"); pair(b = effect("b"), a = effect("l") :: NestedR)
    println("end")
  }
}
