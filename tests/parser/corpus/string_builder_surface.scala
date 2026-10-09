// The members of `StringBuilder`, with the builders they return and the ones they leave alone.
final class Tagged(val tag: String):
  override def toString: String = "<" + tag + ">"

def show(label: String, values: Any*): Unit = println(label + ": " + values.mkString(" | "))

def appends(): Unit =
  val sb = new StringBuilder
  sb.append(1).append(' ').append(2L).append(' ').append(1.5).append(' ').append(2.5f).append(' ').append(true)
  sb.append(' ').append('c').append(' ').append("s").append(' ').append(3.toByte).append(' ').append(4.toShort)
  sb.append(' ').append(new Tagged("t")).append(' ').append(Some(1)).append(' ').append(()).append(' ').append(List(1, 2))
  show("appends", sb.toString, sb.length, sb.size)
  val any: Any = 42
  val union: Int | String = "u"
  show("any", new StringBuilder("x").append(any).append(union).append(BigInt(7)).toString)
  val other = new StringBuilder("inner")
  show("builder in builder", new StringBuilder("outer ").append(other).toString)
  val self = new StringBuilder("ab")
  self.append(self)
  show("self", self.toString)
  val chained = new StringBuilder("a")
  (chained ++= "bc") += 'd'
  chained.addOne('e').addAll("fg").appendAll("hi")
  show("chained", chained.result(), chained.mkString)

def edits(): Unit =
  val sb = new StringBuilder("hello world")
  sb.insert(0, 42).insert(2, ' ').insert(0, 1.5).insert(0, true).insert(4, "str").insert(0, 7L).insert(sb.length, new Tagged("end"))
  show("insert", sb.toString)
  sb.deleteCharAt(0).deleteCharAt(sb.length - 1)
  show("deleteCharAt", sb.toString)
  sb.delete(0, 4).delete(sb.length - 3, sb.length + 10).delete(2, 2)
  show("delete", sb.toString)
  sb.setLength(5)
  show("setLength shorter", sb.toString, sb.length)
  sb.setLength(7)
  show("setLength longer", sb.length, sb.charAt(6) == '\u0000', sb.charAt(5).toInt)
  sb.setLength(5)
  sb.update(0, 'X')
  sb.setCharAt(1, 'Y')
  sb(2) = 'Z'
  show("update", sb.toString)
  sb.replace(1, 3, "--long--").replace(sb.length - 1, sb.length + 5, "!").replace(0, 0, "^")
  show("replace", sb.toString)
  sb.clear()
  show("clear", sb.isEmpty, sb.nonEmpty, sb.length, "[" + sb.toString + "]")
  sb.append("again")
  show("after clear", sb.toString, sb.isEmpty, sb.nonEmpty)

def reads(): Unit =
  val sb = new StringBuilder("abcabc")
  show("chars", sb.charAt(0), sb(1), sb.head, sb.last, sb.apply(5))
  show("indexOf", sb.indexOf("bc"), sb.lastIndexOf("bc"), sb.indexOf("zz"), sb.lastIndexOf("zz"))
  show("substring", sb.substring(1, 3), sb.subSequence(2, 5).toString, sb.subSequence(0, 0).length)
  show("startsWith", sb.startsWith("abc"), sb.startsWith("bc"), sb.endsWith("abc"), sb.endsWith("ab"))
  show("toList", sb.toList, new StringBuilder().toList)
  var seen = ""
  sb.foreach(c => seen = seen + c.toUpper)
  show("foreach", seen)
  var count = 0
  sb.foreach { c => sb.append('!'); count += 1 }
  show("foreach over a snapshot", count, sb.toString)
  val bounds =
    try sb.charAt(99).toString
    catch case e: IndexOutOfBoundsException => "out of bounds"
  show("charAt past the end", bounds)

def copies(): Unit =
  val sb = new StringBuilder("hello")
  val reversed = sb.reverse
  sb.append("!")
  reversed.append("?")
  show("reverse", sb.toString, reversed.toString)
  val taken = sb.take(2)
  val dropped = sb.drop(2)
  val takenRight = sb.takeRight(3)
  val droppedRight = sb.dropRight(3)
  sb.append("+")
  taken.append("1")
  dropped.append("2")
  takenRight.append("3")
  droppedRight.append("4")
  show("slices", sb.toString, taken.toString, dropped.toString, takenRight.toString, droppedRight.toString)
  show("slices past the ends", sb.take(-1).toString, sb.take(99).toString, sb.drop(-1).toString, sb.drop(99).toString, sb.takeRight(99).toString, sb.dropRight(99).toString)
  val pairs = new StringBuilder("a😀b").reverse
  show("reverse keeps a surrogate pair", pairs.length, pairs.charAt(0), pairs.charAt(3))

def constructed(): Unit =
  val sized = new StringBuilder(64)
  show("capacity", sized.length, sized.append("x").toString)
  val from = new StringBuilder("init")
  show("init", from.toString, from.length)
  val cs: java.lang.CharSequence = from
  show("as a CharSequence", cs.length, cs.charAt(1), cs.subSequence(1, 3).toString, cs.toString)
  val anyRef: Any = from
  show("type test", anyRef.isInstanceOf[StringBuilder], anyRef.isInstanceOf[String])
  show("equality", from == from, from == new StringBuilder("other"))

// The JDK builder's `append` of a range: of chars from an offset and a count, of a CharSequence
// between two indices.
def rangeAppends(): Unit =
  val sb = new java.lang.StringBuilder()
  sb.append(Array[Char](), 0, 0)
  sb.append(Array('a', 'b', 'c', 'd'), 1, 2)
  sb.append("wxyz", 1, 3)
  sb.append(new java.lang.StringBuilder("pqrs"), 0, 1)
  show("range appends", sb.toString, sb.length)
  // A range outside the array appends nothing and throws, an end past Int.MaxValue included.
  for (offset, len) <- List((1, 2), (-1, 0), (3, 0), (0, -1), (1, Int.MaxValue), (2, 0)) do
    val b = new java.lang.StringBuilder("x")
    val thrown = try { b.append(Array('a', 'b'), offset, len); "none" } catch case e: IndexOutOfBoundsException => e.getMessage
    show(s"range append $offset $len", b.toString, thrown)

@main def run(): Unit =
  appends()
  rangeAppends()
  edits()
  reads()
  copies()
  constructed()
