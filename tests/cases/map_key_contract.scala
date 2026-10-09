//> using platform js
// std: lean
// The keys of the immutable Map and Set compare as the target's store compares them:
// on JavaScript 1 and 1.0 are one key (one JS number) and 1L another,
// NaN one key; on the JVM and the interpreter the cooperative equality of boxed numbers makes
// 1, 1L and 1.0 one key and NaN two; 0.0 and -0.0 are one key everywhere, null is a key, a case
// class compares structurally, keys with one hash stay apart, an update of a present key keeps
// its slot and its stored key, a removed and re-added key goes to the end. The expectations are the documented contract of each target
// (master's output), not scala-cli's: Scala.js would merge 1 and 1L, so `tests/run.sh --regen`
// must leave this file alone.
final case class P(x: Int, y: Int)

final class Const(val n: Int):
  override def hashCode: Int = 1
  override def equals(o: Any): Boolean = o match
    case c: Const => c.n == n
    case _ => false
  override def toString: String = "Const(" + n + ")"

def show(label: String, xs: Iterable[Any]): Unit = println(label + ": " + xs.mkString("[", ", ", "]") + " size " + xs.size)

@main def main(): Unit =
  val nan = Double.NaN
  val forward = Map[Any, String](1 -> "int", 1L -> "long", 1.0 -> "double")
  val backward = Map[Any, String](1.0 -> "double", 1L -> "long", 1 -> "int")
  show("forward", forward)
  show("backward", backward)
  println("lookups " + forward.get(1) + " " + forward.get(1L) + " " + forward.get(1.0) + " " + backward.get(1) + " " + backward.get(1L) + " " + backward.get(1.0))
  show("mixed set", Set[Any](1, 1L, 1.0, 2))
  show("nan set", Set(nan, nan))
  val nans = Map(nan -> 1) + (nan -> 2)
  println("nan map " + nans.size + " " + nans.contains(nan) + " " + nans.get(nan).isDefined)
  show("zero set", Set(0.0, -0.0))
  val zeros = Map(0.0 -> "a").updated(-0.0, "b")
  println("zero map " + zeros.size + " " + zeros.get(0.0) + " " + zeros.get(-0.0))
  val nulls = Map[String, Int](null -> 1, "a" -> 2)
  println("null map " + nulls.size + " " + nulls.get(null) + " " + nulls.contains("a") + " " + (nulls - null).size)
  show("null set", Set[String](null, null, "b"))
  val structural = Map(P(1, 2) -> "a").updated(P(1, 2), "b")
  println("case keys " + structural.size + " " + structural.get(P(1, 2)) + " " + structural.contains(P(2, 1)) + " " + Set(P(1, 2), P(1, 2), P(2, 1)).size)
  val chars = Map[Any, String]('a' -> "char", 97 -> "int")
  println("char keys " + chars.size + " " + chars.get('a') + " " + chars.get(97))
  var consts = Map.empty[Const, Int]
  var i = 0
  while i < 40 do
    consts = consts.updated(new Const(i % 20), i)
    i += 1
  println("constant hash " + consts.size + " " + consts.get(new Const(3)) + " " + consts.get(new Const(20)) + " " + (consts - new Const(3)).size + " " + (consts - new Const(3)).contains(new Const(3)))
  var constSet = Set.empty[Const]
  i = 0
  while i < 30 do
    constSet = constSet + new Const(i % 10)
    i += 1
  println("constant hash set " + constSet.size + " " + constSet.contains(new Const(9)) + " " + (constSet - new Const(9)).size)
  println("stored key " + Map[Any, String](1.0 -> "a").updated(1, "b").keys + " " + Map[Any, String](1L -> "a").updated(1.0, "b").keys)
  show("slot kept", Map("a" -> 1, "b" -> 2, "c" -> 3).updated("a", 5))
  show("re-added", Map("a" -> 1, "b" -> 2, "c" -> 3) - "a" + ("a" -> 9))
  show("set re-added", Set("a", "b", "c") - "a" + "a")
  println("hashes " + Map("a" -> 1, "b" -> 2).hashCode + " " + Map("b" -> 2, "a" -> 1).hashCode + " " + Set(1, 2, 3).hashCode + " " + Map.empty[Int, Int].hashCode + " " + Set.empty[Int].hashCode + " " + Map(P(1, 2) -> List(1)).hashCode)
  println("equality " + (Map("a" -> 1, "b" -> 2) == Map("b" -> 2, "a" -> 1)) + " " + (Map("a" -> 1) == Map("a" -> 2)) + " " + (Set(1, 2) == Set(2, 1)) + " " + (Map[Any, Int](1 -> 1) == Map[Any, Int](1L -> 1)))
