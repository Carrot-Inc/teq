// teq: --wunused all
// A method's parameter is unused unless its body consumes nothing (CheckUnused.isUnconsuming), as scalac reads the
// body last, after Inlining and FirstTransform: an operation over expanded inline calls folded, a plain inline call's
// expansion of its declared result, a parameterless method's reference no constant, an ascription its own type and
// then its operand's, a literal (a `classOf` too). scalac 3.8.4 under -Wunused:all warns on the a* methods alone; it
// prints the lines of the .run.expected file.
object C:
  final val t = true
  def one: 1 = 1
  def oneP(): 1 = 1
  inline def three: Int = 3
  inline def five: 5 = 5
  inline def s: String = "s"
class U:
  def a1(x: Int) = C.one
  def a2(x: Int) = { println(); C.one }
  def a3(x: Int) = C.three
  def a4(x: Int) = { println(); (1: Int) }
  def a5(x: Int) = "a".length
  def a6(x: Int) = { println(); () }
  def a8(x: Int) = { println(); null }
  def a9(x: Int): Unit = "a".length
  def b1(x: Int) = C.t
  def b2(x: Int) = C.oneP()
  def b3(x: Int) = C.three + 0
  def b4(x: Int) = -C.three
  def b5(x: Int) = C.s + "t"
  def b6(x: Int) = { println(); C.three + 1 }
  def b7(x: Int) = C.three == 3
  def b8(x: Int) = C.five
  def b9(x: Int) = { println(); C.five }
  def b10(x: Int) = (C.five: Int)
  def b11(x: Int) = { println(); 1 }
  def b12(x: Int) = (C.one: 1)
  def b13(x: Int) = summon[ValueOf[1]].value
  def b14(x: Int) = classOf[Int]
  def b15(x: Int) = if true then () else ()
@main def run(): Unit = println(U() != null)
