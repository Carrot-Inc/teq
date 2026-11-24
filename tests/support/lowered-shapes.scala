//> using file ../../tools/script
// The generated proof of a slice of the lowered IR (the JVM back end's lowering, src/jvm/lower.rs): every shape the
// lowering takes, in every type wanted of it and in every position, built by two binaries whose class files must be
// the same byte for byte. The programs come from the three tables below, shapes by wants by positions, for four
// kinds of host (a class, a class with a subclass, an object, a trait): one program per host and shape, each
// (want, position) a method of the host that `main` calls, so that a build for a closed world keeps every one.
// Two more hosts hold what a method cannot: a class whose constructor has the expression as its superclass's
// argument, as a field's initialiser and as a statement, and a value class, whose `this` is its underlying value.
// Each program is built four ways by each binary: as a program and as products (`--products`, an open world), with
// one worker and with eight. A program that either binary does not build is a failure, never identical: the set
// proves nothing through a program that does not compile. A slice that moves a shape adds its rows to SHAPES, and
// to WANTS and POSITIONS where it brings a new type or place.
// Run from the repository's root:
// `./teq interp tests/support/lowered-shapes.scala -- --reference <binary> [--teq <binary>] [--out <dir>] [--jobs <n>]
// [--shapes <regex>]`, the reference the base's compiler, the compiler under test `--teq`, else $TEQ, else
// target/release/teq; the programs and the outputs under `--out` (out/lowered-shapes); `--jobs` builds at once (the
// processors); `--shapes` the shapes whose names the expression matches whole (all by default); `--after-escape` the
// escape (`escapes`) every method and constructor of the programs takes first, with the forms that hold a shape
// after it (`escapePositions`); `--family-sample` the number of shapes of each family taken.
import java.nio.file.{Files, Path, Paths}

// What a shape is: the statements before it, the expression, its type, and the hosts it needs ("" a class and a
// constructor, "method" a class alone, "members" the four hosts with members, "class" the two with constructor
// parameters, "value" a value class, "constructor" a constructor alone). `String.formatted` is "method": in a
// constructor the typer does not find it with eight workers today.
final case class Shape(name: String, before: String, expr: String, tpe: String, hosts: String)

// A position: the text of a method `$n` whose body holds the expression `$E` where `$W` is wanted; `$P` the
// parameters, `$S` the statements before.
final case class Position(name: String, text: String, fits: String = "")

// An escape: a statement the lowering leaves to the generator of the typed IR (a `Kind::Legacy` node, a type test's
// node, a `try`'s cases the lowering does not take), after which the rest of the method's lowering sees typed IR left
// (`Gen::analysis_in_order`). `$X` is an operand the escaped tree holds, `topv` where no shape stands there; `operand`
// whether the tree holds one, `expression` whether it may stand in a field's initialiser or a constructor, `host` the
// one host whose methods it escapes in where it escapes in no other.
final case class Escape(name: String, text: String, operand: Boolean = true, expression: Boolean = true, host: String = "")

object LoweredShapes:
  val params = "pi: Int, pl: Long, pd: Double, pf: Float, pb: Boolean, pc: Char, pby: Byte, psh: Short, ps: String, pa: Any, pt: T, " +
    "pv: V, pbox: Box[Int], psb: Box[String], pfn: Int => Int, pf0: () => String, pf2: (Int, String) => String, " +
    "other: Peer, sub: Sub, tr: Tr"
  val arguments = "1, 2L, 3.5, 4.5f, true, 'c', 5.toByte, 6.toShort, \"s\", \"a\", \"t\", new V(8), new Box[Int](9), new Box[String](\"b\"), " +
    "(x: Int) => x + 1, () => \"f0\", (x: Int, y: String) => y + x, new Peer(1, \"s\", 2, \"q\"), new Sub(3), new Sub(4)"

  val members = """
  var fv: Long = 1L
  private val fpriv: Double = 2.5
  private[this] val fthis: Boolean = true
  val fbox: Box[Int] = new Box[Int](1)
  val fsbox: Box[String] = new Box[String]("b")
  val fvc: V = new V(7)
  lazy val lz: String = "lz"
  lazy val lzmi: Int = fi + 2
  lazy val lzml: Long = fi * 3L
  lazy val lzmd: Double = fi / 2.0
  lazy val lzmz: Boolean = fi > 0
  lazy val lzmc: Char = 'm'
  lazy val lzmu: Unit = ()
  lazy val lzma: Any = fs
  lazy val lzmr: AnyRef = fs
  lazy val lzmv: V = new V(fi)
  lazy val lzmn: String = null
  val ff: Int => Int = (x: Int) => x + 1
  def abs: Int = fi
  def m0(): Int = fi
  def m1(x: Int): Long = x.toLong
  def ms(x: String, y: Int): String = x + y
  def mu(): Unit = ()
  def mg[A](a: A): A = a
  def dflt(x: Int = 4, y: String = "d"): String = y + x
  def dfp(a: Int)(b: Int = a + 1): Int = a + b
  def dfr(a: Int, b: Int = 2, c: String): String = c + a + b
  def va(xs: Int*): Int = xs.length
  final def tl(n: Int, acc: Long): Long = if n <= 0 then acc else tl(n - 1, acc + n)
  final def tls(n: Int, s: String): String = { val t: String = s + n; if n <= 0 then t else tls(n - 1, t) }
  def vl(xs: Long*): Int = xs.length
  def vd(xs: Double*): Int = xs.length
  def vf(xs: Float*): Int = xs.length
  def vb(xs: Boolean*): Int = xs.length
  def vc(xs: Char*): Int = xs.length
  def vby(xs: Byte*): Int = xs.length
  def vsh(xs: Short*): Int = xs.length
  def vu(xs: Unit*): Int = xs.length
  def vs(xs: String*): Int = xs.length
  def vaa(xs: Any*): Int = xs.length
  def vg[A](xs: A*): Int = xs.length
  def vv(xs: V*): Int = xs.length
  def arr(xs: Array[Int]): Int = xs.length
  def arrs(xs: Array[String]): Int = xs.length
  class Mem(val z: Int) { def both: Int = z + fi }
"""

  val prelude = """final class V(val u: Int) extends AnyVal {
  def twice: V = new V(u * 2)
}
final class VS(val s: String) extends AnyVal
final class VL(val l: Long) extends AnyVal
object HoldsV {
  val bv: Box[V] = new Box[V](new V(3))
}
final class VG[A](val g: A) extends AnyVal
trait Universal extends Any
final class VU(val n: Int) extends AnyVal with Universal
final class VO(val o: Universal) extends AnyVal
class Box[A](val item: A) {
  def get: A = item
  def put(a: A): Box[A] = new Box[A](a)
  var held: A = item
  def dfg(n: Int = 1): Int = n
}
trait Tr {
  def abs: Int
  def im(x: Int): Int = x + abs
  def sd: Int = 7
  def td(x: Int = 2): Int = x + 1
  def tdp(a: Int)(b: Int = a + 1): Int = a + b
  val tv: String = "tv"
  var tvar: Int = 0
}
class CBase[A](val got: A)
trait Gives[A] { def give(): A }
class GivesV extends Gives[V] { def give(): V = new V(1) }
trait TakesIn[A] { def take(a: A): Int }
class TakesV extends TakesIn[V] { def take(a: V): Int = a.u }
class CloneCell(var n: Int) extends scala.collection.mutable.Cloneable[CloneCell]
class Base(val bn: Int) {
  def inherited(x: Int): Int = x + bn
  def over(x: Int): Any = x
  var bvar: Int = 0
}
class Sub(n: Int) extends Base(n) with Tr {
  def abs: Int = bn
  override def over(x: Int): String = x.toString
}
object O {
  val k: Int = 3
  var kv: String = "k"
  def m(x: Int): Int = x + 1
  def gen[A](a: A): A = a
  extension (x: Int) def ++:(s: String): String = s + x
  enum Tone { case Low, High }
  given og: Ordering[Shade] with { def compare(a: Shade, b: Shade): Int = 0 }
}
enum Shade { case Warm, Cold }
enum Sized(val n: Int) { case Small extends Sized(1); case Big extends Sized(2) }
given topgiven: Ordering[Shade] with { def compare(a: Shade, b: Shade): Int = 1 }
inline val topc = 42
inline val topl = 7L
inline val topdv = 2.5
inline val topfl = 1.5f
inline val topb = true
inline val topno = false
inline val topch = 'q'
inline val topstr = "is"
inline val topempty = ""
// A class no program makes, and a value class with a right-associative method.
final class NeverMade
final class VR(val n: Int) extends AnyVal { def +:(x: Int): Int = n + x }
inline val topk = classOf[Peer]
extension (x: Int) def +:(s: String): String = s + x
// Two interfaces the superclass brings, which a class lists for its super calls in the order of the calls' ends:
// a self tail call evaluates the arguments before the receiver, so the later call is written first.
abstract class TailStart extends java.util.Iterator[String] with java.util.function.Predicate[Int] {
  def hasNext(): Boolean = false
  def next(): String = "x"
  def test(x: Int): Boolean = x > 1
}
class TailBoth extends TailStart {
  final def loop(n: Int, acc: Boolean): Boolean = if (n <= 0) acc else { super.negate().test(n); new TailBoth }.loop(n - 1, { try super.remove() catch { case _: UnsupportedOperationException => () }; acc })
}
object Even {
  def unapply(x: Int): Boolean = x % 2 == 0
}
object Halves {
  def unapply(x: Int): (Int, Int) = (x / 2, x % 2)
}
object Twice {
  def unapply(x: Int): Option[(Int, Int)] = Some((x, x))
}
object Digits {
  def unapplySeq(x: Int): Option[Seq[Int]] = Some(Seq(x / 10, x % 10))
}
enum Color {
  case Red, Green
}
enum Figure {
  case Circle(r: Int)
  case Square(s: Int)
}
final case class Nest(p: Pair, v: V)
class Made(val n: Int)
object Made {
  val count: Int = 1
}
case class Pair(a: Int, b: String)
final case class Prims(j: Long, d: Double, f: Float, i: Int, z: Boolean, c: Char, s: String, v: V)
case class Open(a: Int, s: String)
case class Refs(a: String, b: List[Int])
class Holder {
  case class In(x: Int)
  val made: In = In(1)
}
final class VD(val d: Double) extends AnyVal
final class VJ(val j: Long) extends AnyVal
final case class CV(u: Int) extends AnyVal
final case class CVS(s: String) extends AnyVal
enum Paint { case Red, Green, Blue }
enum Form { case Dot; case Circle(r: Int) }
class Palette { enum Tone { case Light, Dark } }
enum Suit { case Hearts, Spades }
object Suit {
  val count: Int = 2
  def first: Suit = Hearts
}
enum Coin { case Heads, Tails }
object Coin {
  lazy val face: Int = Heads.ordinal + 1
}
case class HoldsValues(v: V, s: VS)
final class GO[A](val g: A) extends AnyVal { def holder: O.type = O }
final class G[A](val g: A) extends AnyVal { def pair(x: Int): String = s"$g/$x" }
final class VB(val u: Int) extends AnyVal { def plus(x: Int): Int = u + x }
object VB { val k: Int = 2 }
object Mk {
  def go(s: String): GO[String] = new GO(s)
  def g(s: String): G[String] = new G(s)
}
enum Hue { case Red, Green }
class Tones { enum Tone { case Low, High } }
class Two(val a: Int, val b: String) {
  def this(a: Int) = this(a, "z")
}
class Outer {
  class Inner(val z: Int) {
    def this() = this(7)
  }
}
val outer: Outer = new Outer
class Dc(val x: Int = 4, val y: String = "d")
class Dm(val a: Int)(val b: Int = a + 1)
class Dq(val a: Int)(val b: String)(val c: Int = a + b.length)
case class Cp(a: Int)(b: Int = a * 2)
class Ou {
  class In(val x: Int = 5)
  class Ip(val a: Int)(val b: Int = a)
}
val ou: Ou = new Ou
class Hn {
  enum En {
    case P, Q
  }
}
val hn: Hn = new Hn
def topd(x: Int = 4, s: String = "t"): String = s + x
def topp(a: Int)(b: Int = a + 1): Int = a + b
object Om { def g(a: Int)(b: Int = a * 2): Int = a + b }
extension (s: String) def extd(n: Int = s.length): Int = n
class Chs(val name: String)
trait EncP { def cs: Chs }
object EncP {
  given enc(using cs0: Chs = new Chs("u")): EncP with { def cs = cs0 }
}
inline def cvo(x: Int): Int = scala.compiletime.constValueOpt[x.type].getOrElse(-1)
final val cvb: 9 = 9
val topsrc: scala.io.Source = scala.io.Source.fromString("a\nb")
def topf(x: Int): String = "t" + x
def topg[A](a: A): A = a
def toploop(n: Int, acc: Int): Int = if n <= 0 then acc else toploop(n - 1, acc + n)
val topv: Int = 5
var topw: String = "w"
class Peer(val fi: Int, val fs: String, pp: Int, pq: String) extends Tr {""" + members + """}
object Takes {
  def any(w: Any): Unit = ()
  def id[A](a: A): A = a
  def lazily[A](a: => A): Unit = { a; () }
  def one[A](a: A): Array[A] = Array[Any](a).asInstanceOf[Array[A]]
}
// Templates of the program's own, for the forms no stand-in of the standard library reaches.
@jvm("invokestatic java/lang/Math.abs(D)D")
def tabs(d: Double): Double = d
@jvm("invokevirtual java/lang/String.concat(Ljava/lang/String;)Ljava/lang/String;")
def tconcat(a: String, b: String): String = a + b
@jvm("$0 invokestatic java/lang/String.valueOf(Ljava/lang/Object;)Ljava/lang/String;")
def tshow[A](a: A): String = "" + a
@jvm("new java/lang/StringBuilder dup $0 invokespecial java/lang/StringBuilder.<init>(Ljava/lang/String;)V")
def tnew(s: String): java.lang.StringBuilder = new java.lang.StringBuilder(s)
@jvm("$0 $1:I invokevirtual java/util/ArrayList.get(I)Ljava/lang/Object; cast_result")
def tget[A](xs: java.util.ArrayList[A], i: Int): A = xs.get(i)
@jvm("$0 $1:L invokevirtual java/lang/StringBuilder.append(Ljava/lang/Object;)Ljava/lang/StringBuilder; pop")
def tdrop(b: java.lang.StringBuilder, x: Any): Unit = { b.append(x); () }
@jvm("new java/lang/IllegalStateException dup $0 invokespecial java/lang/IllegalStateException.<init>(Ljava/lang/String;)V athrow")
def tthrow(s: String): Nothing = throw new IllegalStateException(s)
@jvm("$0:I $1:I iadd")
def tadd(a: Int, b: Int): Int = a + b
@jvm("$0:J $1:J lxor")
def txor(a: Long, b: Long): Long = a ^ b
@jvm("$0:D $1:D dmul")
def tmul(a: Double, b: Double): Double = a * b
@jvm("$0:I ipush 3 ishl iconst_1 isub")
def tshift(a: Int): Int = (a << 3) - 1
@jvm("$0:I i2l")
def twide(a: Int): Long = a.toLong
@jvm("$0:D d2i i2c")
def tnarrow(a: Double): Char = a.toInt.toChar
"""

  // A type, and the types wanted of a value of it as Scala writes them. THIS is the host's own type.
  val wants: Vector[(String, Vector[String])] = Vector(
    "Int" -> Vector("Int", "Long", "Float", "Double", "Any", "AnyVal"),
    "Long" -> Vector("Long", "Float", "Double", "Any"),
    "Float" -> Vector("Float", "Double", "Any"),
    "Double" -> Vector("Double", "Any"),
    "Boolean" -> Vector("Boolean", "Any"),
    "Char" -> Vector("Char", "Int", "Long", "Double", "Any"),
    "Byte" -> Vector("Byte", "Short", "Int", "Long", "Double", "Any"),
    "Short" -> Vector("Short", "Int", "Long", "Any"),
    "String" -> Vector("String", "Any", "AnyRef", "CharSequence", "Comparable[String]"),
    "Unit" -> Vector("Unit", "Any"),
    "Any" -> Vector("Any"),
    "V" -> Vector("V", "Any"),
    "VG[Int]" -> Vector("VG[Int]", "Any"),
    "VG[String]" -> Vector("VG[String]", "Any"),
    "VO" -> Vector("VO", "Any"),
    "T" -> Vector("T", "Any"),
    "THIS" -> Vector("THIS", "Any", "AnyRef", "Tr"),
    "O.type" -> Vector("O.type", "Any", "AnyRef"),
    "Box[Int]" -> Vector("Box[Int]", "Any", "AnyRef"),
    "Peer" -> Vector("Peer", "Any", "AnyRef", "Tr"),
    "Sub" -> Vector("Sub", "Any", "Base", "Tr"),
    "Made" -> Vector("Made", "Any"),
    "Pair" -> Vector("Pair", "Any", "Product"),
    "Two" -> Vector("Two", "Any"),
    "Tr" -> Vector("Tr", "Any", "AnyRef"),
    "Int => Int" -> Vector("Int => Int", "Any", "AnyRef"),
    "java.util.List[String]" -> Vector("java.util.List[String]", "Any"),
    "Null" -> Vector("Null", "String", "Any", "AnyRef", "CharSequence", "Peer", "Tr", "Box[Int]", "Int => Int", "java.util.List[String]"),
    "Nothing" -> Vector("Nothing", "Int", "Long", "Double", "Boolean", "String", "Any", "Unit", "V", "Peer"),
    "Base" -> Vector("Base", "Any"),
    "VS" -> Vector("VS", "Any"),
    "VL" -> Vector("VL", "Any"),
    "CV" -> Vector("CV", "Any", "Product"),
    "Shade" -> Vector("Shade", "Any", "AnyRef"), "Sized" -> Vector("Sized", "Any"), "O.Tone" -> Vector("O.Tone", "Any"),
    "Ordering[Shade]" -> Vector("Ordering[Shade]", "Any", "AnyRef"))
  // What the typer narrows an integer literal to, beside the types an Int is wanted as.
  val narrowed = Vector("Byte", "Short", "Char")
  val numeric = Set("Int", "Long", "Float", "Double", "Char", "Byte", "Short")

  // A function type past `Function22`.
  val wideFunction = Vector.fill(23)("Int").mkString("(", ", ", ") => Int")

  val shapes: Vector[Shape] = Vector(
    Shape("lit-int", "", "1", "Int", ""), Shape("lit-long", "", "1L", "Long", ""), Shape("lit-double", "", "1.5", "Double", ""),
    Shape("lit-float", "", "1.5f", "Float", ""), Shape("lit-bool", "", "true", "Boolean", ""), Shape("lit-char", "", "'a'", "Char", ""),
    Shape("lit-string", "", "\"s\"", "String", ""), Shape("lit-unit", "", "()", "Unit", ""), Shape("lit-null", "", "null", "Null", ""),
    // A code template of the std that is a null (`@jvm("aconst_null")`): at the program's own call, compared with
    // `eq` (its two sides loaded, `aconst_null; if_acmpXX`, as the class file's template writes it, not `ifnull`),
    // and in the runtime's methods that return it on a branch (in a method host: a products build of the
    // constructor host fails on both binaries, `cannot write TASTy for a reference to the type parameter A`).
    Shape("template-null", "", "js.undefined", "Any", ""), Shape("template-null-compared", "", "(pa eq js.undefined)", "Boolean", ""),
    Shape("template-null-int", "", "scala.runtime.intOrNull(ps)", "Any", "method"),
    Shape("template-null-long", "", "scala.runtime.longOrNull(ps)", "Any", "method"),
    Shape("template-null-double", "", "scala.runtime.doubleOrNull(ps)", "Any", "method"),
    Shape("template-null-regex", "", "scala.runtime.regexExec(ps, \"\", ps)", "Any", "method"),
    Shape("param-int", "", "pi", "Int", ""), Shape("param-long", "", "pl", "Long", ""), Shape("param-double", "", "pd", "Double", ""),
    Shape("param-float", "", "pf", "Float", ""), Shape("param-bool", "", "pb", "Boolean", ""), Shape("param-char", "", "pc", "Char", ""),
    Shape("param-byte", "", "pby", "Byte", ""), Shape("param-short", "", "psh", "Short", ""), Shape("param-string", "", "ps", "String", ""),
    Shape("param-any", "", "pa", "Any", ""), Shape("param-generic", "", "pt", "T", ""), Shape("param-value-class", "", "pv", "V", ""),
    Shape("param-box", "", "pbox", "Box[Int]", ""), Shape("param-function", "", "pfn", "Int => Int", ""),
    Shape("local-int", "val li: Int = pi + 1", "li", "Int", ""), Shape("local-string", "val ls: String = ps + \"l\"", "ls", "String", ""),
    Shape("local-value-class", "val lv: V = pv", "lv", "V", ""), Shape("local-generic", "val lt: T = pt", "lt", "T", ""),
    Shape("local-var", "var vi: Int = pi; vi = vi + 1", "vi", "Int", ""),
    Shape("local-captured-var", "var cv: Int = pi; val bump = () => { cv = cv + 1 }; bump()", "cv", "Int", ""),
    Shape("local-lazy", "lazy val ll: String = ps + \"z\"", "ll", "String", ""),
    Shape("this", "", "this", "THIS", "members"),
    Shape("module", "", "O", "O.type", ""),
    Shape("field-own-int", "", "fi", "Int", "members"), Shape("field-own-string", "", "fs", "String", "members"),
    Shape("field-own-var", "", "fv", "Long", "members"), Shape("field-private", "", "fpriv", "Double", "members"),
    Shape("field-private-this", "", "fthis", "Boolean", "members"),
    Shape("field-private-param-int", "", "pp", "Int", "class"), Shape("field-private-param-string", "", "pq", "String", "class"),
    Shape("field-generic-int", "", "fbox.item", "Int", "members"), Shape("field-generic-string", "", "fsbox.item", "String", "members"),
    Shape("field-value-class", "", "fvc", "V", "members"), Shape("field-underlying", "", "fvc.u", "Int", "members"),
    Shape("field-lazy", "", "lz", "String", "members"), Shape("field-lazy-int", "", "lzmi", "Int", "members"),
    Shape("field-lazy-long", "", "lzml", "Long", "members"), Shape("field-lazy-double", "", "lzmd", "Double", "members"),
    Shape("field-lazy-boolean", "", "lzmz", "Boolean", "members"), Shape("field-lazy-char", "", "lzmc", "Char", "members"),
    Shape("field-lazy-unit", "", "lzmu", "Unit", "members"), Shape("field-lazy-any", "", "lzma", "Any", "members"),
    Shape("field-lazy-anyref", "", "lzmr", "AnyRef", "members"), Shape("field-lazy-value-class", "", "lzmv", "V", "members"),
    Shape("field-lazy-null", "", "lzmn", "String", "members"), Shape("field-other-lazy-int", "", "other.lzmi", "Int", ""), Shape("field-function", "", "ff", "Int => Int", "members"),
    Shape("field-other-int", "", "other.fi", "Int", ""), Shape("field-other-string", "", "other.fs", "String", ""),
    Shape("field-other-var", "", "other.fv", "Long", ""), Shape("field-other-value-class", "", "other.fvc", "V", ""),
    Shape("field-other-lazy", "", "other.lz", "String", ""), Shape("field-box-int", "", "pbox.item", "Int", ""),
    Shape("field-box-string", "", "psb.item", "String", ""), Shape("field-inherited", "", "sub.bn", "Int", ""),
    Shape("field-trait-val", "", "tr.tv", "String", ""), Shape("field-trait-def", "", "tr.abs", "Int", ""),
    Shape("field-object-val", "", "O.k", "Int", ""), Shape("field-object-var", "", "O.kv", "String", ""),
    Shape("field-param-underlying", "", "pv.u", "Int", ""),
    Shape("field-given-object", "", "O.og", "Ordering[Shade]", ""),
    Shape("field-given-object-receiver", "", "{ other.mu(); O }.og", "Ordering[Shade]", ""),
    // The receiver's code holds a slot (a generic value class's receiver that is no path), freed once it is evaluated.
    Shape("field-given-object-receiver-held", "", "Mk.go(ps).holder.og", "Ordering[Shade]", ""),
    // A generic value class's method on a receiver that is no path: the receiver held in a slot to the call's end.
    Shape("call-generic-value-class-held", "", "Mk.go(ps).holder", "O.type", ""),
    Shape("field-java-instance", "val tok = new java.io.StreamTokenizer(new java.io.StringReader(ps))", "tok.ttype", "Int", ""),
    Shape("field-java-instance-string", "val tok = new java.io.StreamTokenizer(new java.io.StringReader(ps))", "tok.sval", "String", ""),
    Shape("field-java-static", "", "java.io.File.separator", "String", ""), Shape("field-java-static-long", "", "java.lang.Long.MAX_VALUE", "Long", ""),
    Shape("assign-own-var", "", "(fv = pl + 1L)", "Unit", "members"), Shape("assign-own-var-constant", "", "(fv = 2L)", "Unit", "members"),
    Shape("assign-own-var-read", "", "{ fv = pl; fv }", "Long", "members"),
    Shape("assign-other-var", "", "(other.fv = pl)", "Unit", ""), Shape("assign-other-var-read", "", "{ other.fv = pl; other.fv }", "Long", ""),
    Shape("assign-object-var", "", "(O.kv = ps)", "Unit", ""), Shape("assign-top-var", "", "(topw = ps)", "Unit", ""),
    Shape("assign-trait-var", "", "(tr.tvar = pi)", "Unit", ""), Shape("assign-inherited-var", "", "(sub.bvar = pi)", "Unit", ""),
    Shape("assign-generic-var", "", "(pbox.held = pi)", "Unit", ""), Shape("assign-generic-var-string", "", "(psb.held = ps)", "Unit", ""),
    Shape("assign-java-field", "val tok = new java.io.StreamTokenizer(new java.io.StringReader(ps))", "{ tok.nval = pd; tok.nval }", "Double", ""),
    Shape("assign-java-field-unit", "val tok = new java.io.StreamTokenizer(new java.io.StringReader(ps))", "(tok.sval = ps)", "Unit", ""),
    Shape("call-own", "", "m0()", "Int", "members"), Shape("call-own-arg", "", "m1(pi)", "Long", "members"),
    Shape("call-own-two", "", "ms(ps, pi)", "String", "members"), Shape("call-own-unit", "", "mu()", "Unit", "members"),
    Shape("call-own-generic-int", "", "mg(pi)", "Int", "members"), Shape("call-own-generic-string", "", "mg(ps)", "String", "members"),
    Shape("call-own-generic-value-class", "", "mg(pv)", "V", "members"),
    Shape("call-default", "", "dflt()", "String", "members"), Shape("call-default-named", "", "dflt(y = \"z\")", "String", "members"),
    Shape("call-varargs", "", "va(1, pi, 3)", "Int", "members"), Shape("call-varargs-none", "", "va()", "Int", "members"),
    Shape("call-other", "", "other.m0()", "Int", ""), Shape("call-other-two", "", "other.ms(\"a\", pi)", "String", ""),
    Shape("call-other-generic", "", "other.mg(pd)", "Double", ""),
    Shape("call-inherited", "", "sub.inherited(pi)", "Int", ""), Shape("call-covariant", "", "sub.over(pi)", "String", ""),
    Shape("call-covariant-base", "", "(sub: Base).over(pi)", "Any", ""), Shape("call-interface", "", "tr.im(pi)", "Int", ""),
    Shape("call-java-int", "", "ps.length", "Int", ""), Shape("call-java-char", "", "ps.charAt(0)", "Char", ""),
    Shape("call-java-string", "", "ps.substring(1)", "String", ""), Shape("call-java-bool", "", "ps.isEmpty", "Boolean", ""),
    Shape("call-any-string", "", "pa.toString", "String", ""), Shape("call-any-hash", "", "pa.hashCode", "Int", ""),
    Shape("call-box-get-int", "", "pbox.get", "Int", ""), Shape("call-box-get-string", "", "psb.get", "String", ""),
    Shape("call-box-put", "", "pbox.put(pi)", "Box[Int]", ""), Shape("call-object", "", "O.m(pi)", "Int", ""),
    Shape("call-object-generic-string", "", "O.gen(ps)", "String", ""), Shape("call-object-generic-int", "", "O.gen(pi)", "Int", ""),
    Shape("call-value-class", "", "pv.twice", "V", ""), Shape("call-right-associative", "", "(pi +: ps)", "String", ""),
    // A value class's method through its companion's instance (`Gen::extension_method`): the instance first, the
    // receiver in place; a generic class's receiver that is no path is evaluated first and held in a slot.
    Shape("call-value-class-companion", "", "new VB(pi).plus(pi)", "Int", ""),
    Shape("call-value-class-local", "val lb: VB = new VB(pi)", "lb.plus(2)", "Int", ""),
    Shape("call-generic-value-class-local", "val lg: G[String] = new G(ps)", "lg.pair(pi)", "String", ""),
    Shape("call-generic-value-class-new", "", "new G(ps).pair(pi)", "String", ""),
    Shape("call-generic-value-class-held-pair", "", "Mk.g(ps).pair(pi)", "String", ""),
    Shape("call-jar-value-class", "", "pi.max(3)", "Int", ""), Shape("call-jar-value-class-string", "", "ps.reverse", "String", ""),
    Shape("call-value-class-right-assoc", "", "(1 +: new VR(pi))", "Int", ""),
    Shape("call-jar-generic-value-class-held", "", "(pi -> ps)", "(Int, String)", ""),
    Shape("call-right-associative-member", "import O.*", "(pi ++: ps)", "String", ""),
    Shape("call-java-array", "", "java.util.Arrays.asList(ps, \"x\")", "java.util.List[String]", ""),
    Shape("call-java-array-member", "", "ps.formatted(pa, ps)", "String", "method"),
    Shape("apply-one", "", "pfn(pi)", "Int", ""), Shape("apply-none", "", "pf0()", "String", ""),
    Shape("apply-two", "", "pf2(pi, ps)", "String", ""), Shape("apply-field", "", "ff(pi)", "Int", "members"),
    Shape("static-java-field", "", "java.lang.Integer.MAX_VALUE", "Int", ""), Shape("static-java-call", "", "java.lang.String.valueOf(pi)", "String", ""),
    Shape("static-java-call-two", "", "java.lang.Math.max(pl, 3L)", "Long", ""), Shape("static-top-val", "", "topv", "Int", ""),
    Shape("static-top-var", "", "topw", "String", ""), Shape("static-top-call", "", "topf(pi)", "String", ""),
    Shape("static-top-generic", "", "topg(pd)", "Double", ""),
    Shape("static-enum-value", "", "Shade.Warm", "Shade", ""), Shape("static-enum-value-parameter", "", "Sized.Big", "Sized", ""),
    Shape("static-enum-value-nested", "", "O.Tone.High", "O.Tone", ""),
    Shape("static-given-object", "", "topgiven", "Ordering[Shade]", ""), Shape("static-given-summoned", "", "summon[Ordering[Shade]]", "Ordering[Shade]", ""),
    Shape("static-inline-int", "", "topc", "Int", ""), Shape("static-inline-long", "", "topl", "Long", ""),
    Shape("static-inline-double", "", "topdv", "Double", ""), Shape("static-inline-float", "", "topfl", "Float", ""),
    Shape("static-inline-bool", "", "topb", "Boolean", ""), Shape("static-inline-char", "", "topch", "Char", ""),
    // An inline val's boolean left as a condition: a right operand, a loop's, a guard's (an `if` over one is its branch).
    Shape("static-inline-bool-and", "", "(pb && topb)", "Boolean", ""), Shape("static-inline-bool-or", "", "(pb || topno)", "Boolean", ""),
    Shape("static-inline-bool-loop", "var k = pi", "{ while (topno) k += 1; k }", "Int", ""),
    Shape("static-inline-bool-guard", "", "(pi match { case 0 if topb => 1; case _ => 2 })", "Int", ""),
    // The read on a line of its own, whose position the condition's writer marks.
    Shape("static-inline-bool-and-line", "", "(pb &&\n      topb)", "Boolean", ""),
    Shape("static-inline-string", "", "topstr", "String", ""), Shape("static-inline-class", "", "topk", "Class[Peer]", ""),
    Shape("static-inline-empty", "", "topempty", "String", ""),
    Shape("local-function-value", "val lf: Int => Int = (x: Int) => x + 1", "lf(pi)", "Int", ""),
    Shape("local-function-lazy", "lazy val lzf: Int => Int = (x: Int) => x + 2", "lzf(pi)", "Int", ""),
    // A function value held in a cell (a `var` a closure assigns), in a local class's field, and a lazy one read
    // inside a local class.
    Shape("local-function-cell", "var cf: Int => Int = (x: Int) => x + 3; val bump = () => { cf = (x: Int) => x + 4 }; bump()", "cf(pi)", "Int", ""),
    Shape("local-function-field", "val ff2: Int => Int = (x: Int) => x + 5; class Uf { def g: Int = ff2(pi) }", "new Uf().g", "Int", ""),
    Shape("local-function-lazy-field", "lazy val lzg: Int => Int = (x: Int) => x + 6; class Ug { def g: Int = lzg(pi) }", "new Ug().g", "Int", ""),
    // A lazy function value called after the code before it was left to the old generator (a `return` in a value).
    Shape("local-function-lazy-after-escape", "lazy val lzh: Int => Int = (x: Int) => x + 7; def eh(n: Int): Int = { val q: Int = (if (n > 100) return 0 else n); lzh(q) }", "eh(pi)", "Int", ""),
    Shape("local-def", "def ld(x: Int): Int = x + pi", "ld(3)", "Int", ""),
    // A local trait's member that captures takes its proxies first; through `super[T]` it is the trait's static
    // `m$`. The proxies are the capture analysis's, which a lowering that left typed IR asks out of order.
    Shape("local-trait-member", "val cap: Int = pi + 1; trait Lt { def lm(x: Int): Int = x + cap }; val lt: Lt = new Lt {}", "lt.lm(2)", "Int", ""),
    Shape("local-trait-super", "val cap: Int = pi + 1; trait Lt { def lm(x: Int): Int = x + cap }; class Lc extends Lt { def call(x: Int): Int = super[Lt].lm(x) }",
      "new Lc().call(2)", "Int", ""),
    Shape("local-trait-member-after-match",
      "val q: Int = pi + 2; def ev(n: Int): Boolean = if n == 0 then pi > 0 else od(n - 1); def od(n: Int): Boolean = if n == 0 then q > 1 else ev(n - 1); trait Lt { def lm(x: Int): Boolean = od(x) }; val lt: Lt = new Lt {}",
      "{ val mm = pi match { case 0 => 1; case _ => 2 }; lt.lm(mm) }", "Boolean", ""),
    Shape("local-trait-member-before-lambda",
      "val q: Int = pi + 2; def ev(n: Int): Boolean = if n == 0 then pi > 0 else od(n - 1); def od(n: Int): Boolean = if n == 0 then q > 1 else ev(n - 1); trait Lt { def lm(x: Int): Boolean = ev(x) }; val lt: Lt = new Lt {}",
      "{ val r = lt.lm(pi); val f = () => od(pi); r && f() }", "Boolean", ""),
    Shape("new-class", "", "new Peer(pi, ps, 2, \"q\")", "Peer", ""), Shape("new-subclass", "", "new Sub(pi)", "Sub", ""),
    Shape("new-generic", "", "new Box[Int](pi)", "Box[Int]", ""), Shape("new-value-class", "", "new V(pi)", "V", ""),
    Shape("new-with-companion", "", "new Made(pi)", "Made", ""), Shape("new-case-class", "", "Pair(pi, ps)", "Pair", ""),
    Shape("new-secondary", "", "new Two(pi)", "Two", ""), Shape("new-anonymous", "", "new Tr { def abs: Int = pi }", "Tr", ""),
    Shape("new-local", "class Loc(val q: Int)", "new Loc(pi)", "Any", ""), Shape("new-inner", "", "new outer.Inner(pi)", "Any", ""),
    Shape("new-inner-local", "val lo: Outer = new Outer", "new lo.Inner(pi)", "Any", ""), Shape("new-inner-branch", "", "new outer.Inner(if pb then 1 else 2)", "Any", ""),
    Shape("new-member", "", "new Mem(pi)", "Any", "members"),
    Shape("new-local-captured", "val seen2: Int = pi + 1; class Ls(val q: Int) { def get: Int = q + seen2 }", "new Ls(if pb then 1 else 2)", "Any", ""),
    Shape("new-local-lazy", "lazy val lzc: String = ps + \"z\"; class Lz { def get: String = lzc }", "new Lz().get", "String", ""),
    Shape("new-local-by-name", "def byn(x: => Int): Int = { class Bn { def get: Int = x + 1 }; new Bn().get }", "byn(pi)", "Int", ""),
    Shape("new-local-through-def", "val k2: Int = pi + 1; def ld2(x: Int): Int = x + k2; class Ld { def get: Int = ld2(2) }", "new Ld().get", "Int", ""),
    Shape("new-anonymous-class", "", "new Base(pi) { override def over(x: Int): Any = x + 1 }", "Base", ""),
    Shape("new-anonymous-inner", "", "new outer.Inner(pi) { def more: Int = z + 1 }", "Any", ""),
    Shape("new-anonymous-local", "class Lb(val q: Int) { def g: Int = q + pi }", "new Lb(2) { override def g: Int = 3 }", "Any", ""),
    Shape("new-anonymous-local-secondary", "class Lw(val q: Int) { def this() = this(pi); def g: Int = q + pi }", "new Lw() { override def g: Int = 1 }", "Any", ""),
    Shape("new-unwritten-inner", "val src = scala.io.Source.fromString(ps)", "new src.LineIterator()", "Any", ""),
    Shape("new-unwritten-inner-arguments", "val it = Iterator(pi, 2, 3)", "new it.GroupedIterator[Int](it, 2, if pb then 1 else 2)", "Any", ""),
    Shape("new-unwritten-inner-path", "", "new topsrc.LineIterator()", "Any", ""),
    Shape("new-unwritten-inner-cast", "val gb: Box[scala.io.Source] = new Box[scala.io.Source](scala.io.Source.fromString(ps))", "new gb.item.LineIterator()", "Any", ""),
    Shape("new-anonymous-unwritten", "val src2 = scala.io.Source.fromString(ps)", "new src2.LineIterator() { override def hasNext: Boolean = false }", "Any", ""),
    Shape("new-secondary-library", "", "new java.lang.StringBuilder(ps)", "Any", ""), Shape("new-secondary-library-none", "", "new java.util.ArrayList[String]()", "java.util.List[String]", ""),
    Shape("new-secondary-library-branch", "", "new RuntimeException(if pb then ps else \"t\")", "Any", ""),
    Shape("new-secondary-inner", "", "new outer.Inner()", "Any", ""), Shape("new-secondary-inner-local", "val lo2: Outer = new Outer", "new lo2.Inner()", "Any", ""),
    Shape("new-secondary-local", "class Lx(val q: Int) { def this() = this(pi); def g: Int = q + pi }", "new Lx()", "Any", ""),
    Shape("new-value-class-string", "", "new VS(ps)", "VS", ""), Shape("new-value-class-branch", "", "new V(if pb then pi else 2)", "V", ""),
    Shape("new-value-class-generic-int", "", "new VG[Int](pi)", "VG[Int]", ""), Shape("new-value-class-generic-string", "", "new VG[String](ps)", "VG[String]", ""),
    Shape("new-value-class-universal", "", "new VO(new VU(pi))", "VO", ""),
    Shape("new-enum-value", "", "hn.En.Q", "Any", ""),
    Shape("default-jdk", "", "new RuntimeException()", "Any", ""), Shape("default-jdk-second", "", "new IllegalStateException(ps)", "Any", ""),
    Shape("default-companion", "", "new Dc()", "Any", ""), Shape("default-companion-second", "", "new Dc(pi)", "Any", ""),
    Shape("default-prior", "", "new Dm(pi)()", "Any", ""),
    Shape("default-file-member", "", "topd()", "String", ""), Shape("default-file-member-second", "", "topd(pi)", "String", ""),
    // A method's default on the receiver the typer selected its getter on, over the arguments before it: the getter's
    // call on the receiver and those arguments, written in place (a file member's on the file's object).
    Shape("default-receiver", "", "other.dflt()", "String", ""), Shape("default-receiver-named", "", "other.dflt(y = ps)", "String", ""),
    Shape("default-receiver-second", "", "other.dflt(pi)", "String", ""), Shape("default-receiver-prior", "", "other.dfp(pi)()", "Int", ""),
    Shape("default-receiver-reordered", "", "other.dfr(c = ps, a = pi)", "String", ""),
    Shape("default-receiver-new", "", "new Peer(pi, ps, 1, ps).dfp(pi)()", "Int", ""),
    Shape("default-trait", "", "tr.td()", "Int", ""), Shape("default-generic", "", "pbox.dfg()", "Int", ""),
    Shape("default-module", "", "Om.g(pi)()", "Int", ""), Shape("default-file-member-prior", "", "topp(pi)()", "Int", ""),
    Shape("default-extension", "", "ps.extd()", "Int", ""),
    // A constructor's default after an earlier list, on each route's getter, and `super`'s over an earlier list.
    Shape("default-prior-curried", "", "new Dq(pi)(ps)()", "Any", ""), Shape("default-inner-companion-prior", "", "new ou.Ip(pi)()", "Any", ""),
    Shape("default-local-companion-prior", "class Lp(val a: Int)(val b: Int = a + pi)", "new Lp(pi)()", "Any", ""),
    Shape("default-case-apply-prior", "", "Cp(pi)().a", "Int", ""),
    // A creation of a class that reads a lazy local of the method: its holder and, beside the captures, the instance
    // of the class whose code declares the val (the class's `Extras`).
    Shape("new-extras-home", "lazy val lz: Int = pi + 2; class Lh { def g: Int = lz }", "new Lh().g", "Int", "method"),
    Shape("new-extras-anonymous", "lazy val la: Int = pi + 3", "(new Tr { def abs: Int = la }).abs", "Int", "method"),
    Shape("new-extras-default", "lazy val ld: Int = pi + 4; class Ld(val x: Int = ld) { def g: Int = x + ld }", "new Ld().g", "Int", "method"),
    // A constructor whose superclass's call goes through a selected constructor: a Java superclass's, a secondary one,
    // a captured superclass's.
    Shape("ctor-super-java", "class Jc(v: Int) extends java.util.concurrent.atomic.AtomicInteger(v)", "new Jc(pi).get", "Int", "method"),
    Shape("ctor-super-secondary", "class Sb2(val x: Int) { def this(s: String) = this(s.length) }; class Sc2 extends Sb2(ps)", "new Sc2().x", "Int", "method"),
    Shape("ctor-super-captured", "class Lb(val a: Int) { def g: Int = a + pi }; class Lc extends Lb(2)", "new Lc().g", "Int", "method"),
    Shape("ctor-super-unwritten-inner", "val srcx = scala.io.Source.fromString(ps); class Lix extends srcx.LineIterator", "new Lix().hasNext", "Boolean", "method"),
    Shape("default-jar", "", "new scala.collection.mutable.ArrayDeque[Int]()", "Any", ""),
    Shape("default-inner-companion", "", "new ou.In()", "Any", ""),
    Shape("default-local-companion", "class Lc(val x: Int = pi)", "new Lc()", "Any", ""),
    // A local class's default met after the code before it was left to the old generator (a `return` in a value). Its
    // getter on a local companion held as a lazy val, in order, is the corpus's local_ctor_captures_default_argument:
    // here, a secondary constructor's default of a class that captures is refused in some positions by every binary
    // ("the local Lq is not in scope of the generated method", 23 Nov), a row of its own.
    Shape("default-local-companion-after-escape", "def eq2(n: Int): String = { val q: Int = (if (n > 100) return \"\" else n); class Lr(val tag: String = \"r\"); new Lr().tag + q }", "eq2(pi)", "String", ""),
    // The companion's getter of a jar class where the companion's class file is read, its static forwarder where not.
    Shape("default-jar-companion", "val st0 = scala.collection.mutable.Stack[Int]()", "new scala.collection.mutable.Stack[String]", "Any", ""),
    Shape("default-given", "", "summon[EncP].cs.name", "String", ""),
    Shape("default-enum-block", "enum Eb { case Bb(x: Int = pi) }", "Eb.Bb().x", "Int", ""),
    Shape("new-unwritten-no-instance", "", "cvo(cvb)", "Int", ""),
    Shape("new-secondary-unwritten", "", "new Enumeration { val A = new Val(\"A\"); val B = new Val(3, \"B\") }", "Any", ""),
    // A block's enum whose class case's defaults read nothing of the block: its companion's getters call the case's own.
    Shape("default-local-enum-case", "enum Ld { case B(x: Int = 1)(y: String = \"d\" + x) }", "{ val lb: Ld.B = Ld.B()(); lb.x }", "Int", "method"),
    Shape("block-val", "", "{ val bv: Int = pi + 1; bv }", "Int", ""),
    Shape("block-var", "", "{ var bw: Int = pi; bw = bw + 1; bw }", "Int", ""),
    Shape("block-captured-var", "", "{ var bc: Int = pi; val bump = () => { bc = bc + 1 }; bump(); bc }", "Int", ""),
    Shape("block-captured-val", "", "{ val bs: String = ps + \"b\"; val get = () => bs; get() }", "String", ""),
    Shape("block-def", "", "{ def bd(x: Int): Int = x + pi; bd(3) }", "Int", ""),
    Shape("block-nested", "", "{ val bo: Int = { val bi: Long = pl + 1L; bi.toInt }; bo + 1 }", "Int", ""),
    Shape("block-unit", "", "{ other.mu(); () }", "Unit", ""),
    Shape("block-statements", "", "{ other.mu(); val bt: String = ps; other.mu(); bt }", "String", ""),
    Shape("op-int-add", "", "(pi + 2)", "Int", ""), Shape("op-int-sub", "", "(pi - pi)", "Int", ""),
    Shape("op-int-mul", "", "(pi * 3)", "Int", ""), Shape("op-int-div", "", "(pi / 2)", "Int", ""),
    Shape("op-int-rem", "", "(pi % 3)", "Int", ""), Shape("op-int-shl", "", "(pi << 2)", "Int", ""),
    Shape("op-int-shr", "", "(pi >> 1)", "Int", ""), Shape("op-int-ushr", "", "(pi >>> 1)", "Int", ""),
    Shape("op-int-and", "", "(pi & 7)", "Int", ""), Shape("op-int-or", "", "(pi | 8)", "Int", ""),
    Shape("op-int-xor", "", "(pi ^ pi)", "Int", ""), Shape("op-long-add", "", "(pl + 2L)", "Long", ""),
    Shape("op-long-mul", "", "(pl * 3L)", "Long", ""), Shape("op-long-div", "", "(pl / 2L)", "Long", ""),
    Shape("op-long-rem", "", "(pl % 3L)", "Long", ""), Shape("op-long-shl", "", "(pl << 3)", "Long", ""),
    Shape("op-long-shr-long", "", "(pl >> pl)", "Long", ""), Shape("op-long-ushr", "", "(pl >>> pi)", "Long", ""),
    Shape("op-long-and", "", "(pl & 7L)", "Long", ""), Shape("op-long-or", "", "(pl | 1L)", "Long", ""),
    Shape("op-long-xor", "", "(pl ^ 5L)", "Long", ""), Shape("op-double-add", "", "(pd + 1.0)", "Double", ""),
    Shape("op-double-div", "", "(pd / 2.0)", "Double", ""), Shape("op-double-rem", "", "(pd % 2.0)", "Double", ""),
    Shape("op-float-sub", "", "(pf - 1.5f)", "Float", ""), Shape("op-float-mul", "", "(pf * pf)", "Float", ""),
    Shape("op-float-rem", "", "(pf % 2.0f)", "Float", ""), Shape("op-mixed-int-long", "", "(pi + pl)", "Long", ""),
    Shape("op-mixed-int-double", "", "(pi * pd)", "Double", ""), Shape("op-mixed-char-int", "", "(pc + 1)", "Int", ""),
    Shape("op-mixed-byte-short", "", "(pby + psh)", "Int", ""), Shape("op-mixed-long-float", "", "(pl + pf)", "Float", ""),
    Shape("op-bool-and", "", "(pb & pb)", "Boolean", ""), Shape("op-bool-or", "", "(pb | pb)", "Boolean", ""),
    Shape("op-bool-xor", "", "(pb ^ pb)", "Boolean", ""), Shape("op-neg-int", "", "(-pi)", "Int", ""),
    Shape("op-neg-long", "", "(-pl)", "Long", ""), Shape("op-neg-double", "", "(-pd)", "Double", ""),
    Shape("op-neg-float", "", "(-pf)", "Float", ""), Shape("op-not-int", "", "(~pi)", "Int", ""),
    Shape("op-not-long", "", "(~pl)", "Long", ""), Shape("op-not-bool", "", "(!pb)", "Boolean", ""),
    Shape("conv-int-long", "", "pi.toLong", "Long", ""), Shape("conv-long-int", "", "pl.toInt", "Int", ""),
    Shape("conv-double-int", "", "pd.toInt", "Int", ""), Shape("conv-double-long", "", "pd.toLong", "Long", ""),
    Shape("conv-char-int", "", "pc.toInt", "Int", ""), Shape("conv-char-long", "", "pc.toLong", "Long", ""),
    Shape("conv-int-char", "", "pi.toChar", "Char", ""), Shape("conv-int-byte", "", "pi.toByte", "Byte", ""),
    Shape("conv-int-short", "", "pi.toShort", "Short", ""), Shape("conv-int-float", "", "pi.toFloat", "Float", ""),
    Shape("conv-int-double", "", "pi.toDouble", "Double", ""), Shape("conv-byte-int", "", "pby.toInt", "Int", ""),
    Shape("conv-byte-short", "", "pby.toShort", "Short", ""), Shape("conv-short-int", "", "psh.toInt", "Int", ""),
    Shape("conv-long-float", "", "pl.toFloat", "Float", ""), Shape("conv-long-double", "", "pl.toDouble", "Double", ""),
    Shape("conv-float-int", "", "pf.toInt", "Int", ""), Shape("conv-float-long", "", "pf.toLong", "Long", ""),
    Shape("conv-float-double", "", "pf.toDouble", "Double", ""), Shape("conv-double-float", "", "pd.toFloat", "Float", ""),
    Shape("conv-double-char", "", "pd.toChar", "Char", ""), Shape("conv-long-byte", "", "pl.toByte", "Byte", ""),
    Shape("conv-literal-double", "", "1.toDouble", "Double", ""), Shape("cmp-int-lt", "", "(pi < 3)", "Boolean", ""),
    Shape("cmp-int-ge", "", "(pi >= pi)", "Boolean", ""), Shape("cmp-long-le", "", "(pl <= 2L)", "Boolean", ""),
    Shape("cmp-double-gt", "", "(pd > 1.0)", "Boolean", ""), Shape("cmp-double-lt", "", "(pd < pd)", "Boolean", ""),
    Shape("cmp-float-ge", "", "(pf >= 1.5f)", "Boolean", ""), Shape("cmp-float-lt", "", "(pf < 2.5f)", "Boolean", ""),
    Shape("cmp-char-lt", "", "(pc < 'd')", "Boolean", ""), Shape("cmp-mixed-lt", "", "(pi < pl)", "Boolean", ""),
    Shape("cmp-mixed-gt", "", "(pi > pd)", "Boolean", ""), Shape("cmp-string-lt", "", "(ps < \"t\")", "Boolean", ""),
    Shape("cmp-string-ge", "", "(ps >= ps)", "Boolean", ""), Shape("eq-int", "", "(pi == 3)", "Boolean", ""),
    Shape("ne-int", "", "(pi != 3)", "Boolean", ""), Shape("eq-long", "", "(pl == 2L)", "Boolean", ""),
    Shape("eq-double", "", "(pd == 1.5)", "Boolean", ""), Shape("ne-double", "", "(pd != pd)", "Boolean", ""),
    Shape("eq-float", "", "(pf == pf)", "Boolean", ""), Shape("eq-mixed", "", "(pi == pl)", "Boolean", ""),
    Shape("eq-char", "", "(pc == 'a')", "Boolean", ""), Shape("eq-bool", "", "(pb == pb)", "Boolean", ""),
    Shape("ne-bool", "", "(pb != true)", "Boolean", ""), Shape("eq-string", "", "(ps == \"s\")", "Boolean", ""),
    Shape("ne-string", "", "(ps != \"t\")", "Boolean", ""), Shape("eq-any", "", "(pa == ps)", "Boolean", ""),
    Shape("ne-any", "", "(pa != pa)", "Boolean", ""), Shape("eq-any-int", "", "(pa == 1)", "Boolean", ""),
    Shape("eq-generic", "", "(pt == pt)", "Boolean", ""), Shape("eq-value-class", "", "(pv == pv)", "Boolean", ""),
    Shape("eq-box", "", "(pbox == pbox)", "Boolean", ""), Shape("eq-case-class", "", "(Pair(pi, ps) == Pair(1, \"s\"))", "Boolean", ""),
    Shape("eq-null", "", "(ps == null)", "Boolean", ""), Shape("ne-null", "", "(other != null)", "Boolean", ""),
    // An array's `==` with `null` loads both sides, `eq` and a `null` on the left test the array alone
    // (`Comparing::ArrayEquality`).
    Shape("eq-null-array", "val arn = Array(ps)", "(arn == null)", "Boolean", ""), Shape("ne-null-array-left", "val arl = Array(pi)", "(null != arl)", "Boolean", ""),
    Shape("same-null-array", "val ars = Array(ps)", "(ars eq null)", "Boolean", ""), Shape("cond-null-array", "val arc = Array(ps)", "(if (arc != null) 1 else 2)", "Int", ""),
    // A null an ascription holds is no literal: both sides loaded (`Kind::Typed` over `Null`); `{ null }`
    // as an argument is a block in dotty's tree, as the receiver it is lifted and stays the literal.
    Shape("eq-null-ascribed", "", "(other == (null: AnyRef))", "Boolean", ""), Shape("same-null-ascribed-left", "", "((null: AnyRef) eq other)", "Boolean", ""),
    Shape("ne-null-cast", "", "(other ne null.asInstanceOf[AnyRef])", "Boolean", ""), Shape("eq-null-braced", "", "(other == { null })", "Boolean", ""),
    Shape("same-null-braced-left", "", "({ null } eq other)", "Boolean", ""), Shape("cond-null-ascribed", "", "(if ((null: AnyRef) != other) 1 else 2)", "Int", ""),
    Shape("eq-unit", "", "(() == ())", "Boolean", ""), Shape("same-string", "", "(ps eq ps)", "Boolean", ""),
    Shape("not-same-any", "", "(other ne sub)", "Boolean", ""), Shape("same-null", "", "(other eq null)", "Boolean", ""),
    Shape("and", "", "(pb && pi < 3)", "Boolean", ""), Shape("or", "", "(pb || pi > 3)", "Boolean", ""),
    Shape("not-and", "", "(!(pb && pb))", "Boolean", ""), Shape("and-or-not", "", "((pb || pi == 1) && !pb)", "Boolean", ""),
    Shape("and-three", "", "(pb && pi > 0 && pl < 9L)", "Boolean", ""), Shape("or-and", "", "(pb || pi > 0 && pd < 9.0)", "Boolean", ""),
    Shape("not-compare", "", "(!(pi < 3))", "Boolean", ""), Shape("and-call", "", "(pb && ps.isEmpty)", "Boolean", ""),
    Shape("or-equal", "", "(ps == \"s\" || pa == ps)", "Boolean", ""), Shape("test-string", "", "pa.isInstanceOf[String]", "Boolean", ""),
    Shape("test-int", "", "pa.isInstanceOf[Int]", "Boolean", ""), Shape("test-class", "", "pa.isInstanceOf[Peer]", "Boolean", ""),
    Shape("test-generic", "", "pt.isInstanceOf[Tr]", "Boolean", ""), Shape("test-and", "", "(pa.isInstanceOf[String] && pb)", "Boolean", ""),
    Shape("test-not", "", "(!pa.isInstanceOf[Sub])", "Boolean", ""), Shape("if-int", "", "(if pb then pi else 3)", "Int", ""),
    Shape("if-string", "", "(if pi < 3 then ps else \"t\")", "String", ""), Shape("if-unit", "", "(if pb then other.mu())", "Unit", ""),
    Shape("if-unit-else", "", "(if pb then other.mu() else ())", "Unit", ""), Shape("if-long", "", "(if pb && pi > 0 then pl else 1L)", "Long", ""),
    Shape("if-test", "", "(if pa.isInstanceOf[String] then 1 else 2)", "Int", ""), Shape("if-any", "", "(if pb then pi else ps)", "Any", ""),
    Shape("if-not", "", "(if !pb then pd else 2.5)", "Double", ""), Shape("if-nested", "", "(if pb then (if pi > 1 then 1 else 2) else 3)", "Int", ""),
    Shape("if-value-class", "", "(if pb then pv else new V(1))", "V", ""), Shape("if-generic", "", "(if pb then pt else pt)", "T", ""),
    Shape("if-char", "", "(if ps == \"s\" then 'a' else pc)", "Char", ""), Shape("if-bool", "", "(if pb then true else pi > 2)", "Boolean", ""),
    Shape("if-peer", "", "(if pb || pi == 2 then other else new Peer(1, \"s\", 2, \"q\"))", "Peer", ""), Shape("if-throw", "", "(if pb then pi else throw new RuntimeException(\"x\"))", "Int", ""),
    Shape("if-constant", "", "(if true then pi else 2)", "Int", ""), Shape("if-widened", "", "(if pb then 1 else 2L)", "Long", ""),
    // Constants reduced as dotty's FirstTransform reduces them, in every position, and constants left as conditions.
    Shape("fold-true-and", "", "(true && pb)", "Boolean", ""), Shape("fold-false-and", "", "(false && pb)", "Boolean", ""),
    Shape("fold-true-or", "", "(true || pb)", "Boolean", ""), Shape("fold-false-or", "", "(false || pb)", "Boolean", ""),
    Shape("fold-not-true", "", "(!true)", "Boolean", ""), Shape("fold-not-inline", "", "(!topno)", "Boolean", ""),
    Shape("if-false-no-else", "", "(if false then other.mu())", "Unit", ""), Shape("if-inline-constant", "", "(if topb then pi else 2)", "Int", ""),
    Shape("if-constant-any", "", "(if true then pi else ps)", "Any", ""),
    // A pure block with a constant result, which dotty folds (`FirstTransform.transformBlock`) and teq tests: a row.
    Shape("if-pure-block-constant", "", "(if ({ val kq = 1; true }) pi else 2)", "Int", ""),
    Shape("cond-right-true", "", "(pb && true)", "Boolean", ""), Shape("cond-right-false", "", "(pb || false)", "Boolean", ""),
    Shape("guard-true", "", "(pi match { case 0 if true => 1; case _ => 2 })", "Int", ""),
    // A constant left by a fold at the place of the tree it stands for, the connective's right operand on another line:
    // that operand where it is what the fold leaves (`true && b`, `false || b`), the connective where the left one is.
    Shape("fold-true-and-line", "", "(pb && (true &&\n      true))", "Boolean", ""), Shape("fold-false-or-line", "", "(pb && (false ||\n      true))", "Boolean", ""),
    Shape("fold-nested-line", "", "(pb && (true &&\n      (false ||\n      true)))", "Boolean", ""),
    Shape("fold-false-and-line", "", "(pb || (false &&\n      pb))", "Boolean", ""), Shape("fold-true-or-line", "", "(pb && (true ||\n      pb))", "Boolean", ""),
    Shape("guard-true-and-line", "", "(pi match { case 0 if true &&\n      true => 1; case _ => 2 })", "Int", ""),
    Shape("while-true-return", "def wl(n: Int): Int = { var k = n; while (true) { if (k > 3) return k; k += 1 }; 0 }", "wl(pi)", "Int", ""),
    Shape("match-never-made", "", "(pa match { case _: NeverMade => 1; case _ => 2 })", "Int", ""),
    Shape("if-or", "", "(if pb || ps.isEmpty then pf else 1.5f)", "Float", ""), Shape("if-condition-if", "", "(if (if pb then pi > 1 else pi < 1) then ps else \"u\")", "String", ""),
    Shape("if-block", "", "(if pb then { val ib: Int = pi + 1; ib } else { other.mu(); 2 })", "Int", ""), Shape("while-count", "", "{ var wi: Int = 0; while wi < pi do wi = wi + 1; wi }", "Int", ""),
    Shape("while-and", "", "{ var wj: Int = pi; while wj > 0 && pb do { wj = wj - 2 }; wj }", "Int", ""), Shape("while-long", "", "{ var wk: Long = 0L; while (wk < 3L) { wk = wk + 1L; other.mu() }; wk }", "Long", ""),
    Shape("while-do", "", "{ var wd: Int = 0; while { wd = wd + 1; wd < pi } do (); wd }", "Int", ""), Shape("while-false", "", "{ while (pb && false) other.mu(); ps }", "String", ""),
    Shape("cell-read", "var cr: Int = pi; val bump = () => { cr = cr + 1 }; bump()", "cr", "Int", ""),
    Shape("cell-assign", "var ca: String = ps; val read = () => ca", "{ ca = ca + \"x\"; read() }", "String", ""),
    Shape("cell-long", "var cl: Long = pl; val add = (d: Long) => { cl = cl + d }; add(2L)", "{ cl = cl * 2L; cl }", "Long", ""),
    Shape("cell-in-lambda", "var ci: Int = pi", "{ val f = () => { ci = ci + 1; ci }; f() + ci }", "Int", ""),
    Shape("cell-generic", "var cg: T = pt; val set = (t: T) => { cg = t }; set(pt)", "cg", "T", ""),
    Shape("cell-value-class", "var cv2: V = pv; val set = () => { cv2 = new V(2) }; set()", "cv2", "V", ""),
    Shape("capture-local-class", "val seen: Int = pi + 1; class Lc { def get: Int = seen + 1 }", "new Lc().get", "Int", ""),
    Shape("capture-local-class-var", "var count: Int = pi; class Lv { def bump(): Int = { count = count + 1; count } }", "new Lv().bump()", "Int", ""),
    Shape("capture-anonymous", "val base: String = ps + \"q\"", "(new Tr { def abs: Int = base.length }).abs", "Int", ""),
    Shape("capture-anonymous-var", "var hits: Int = 0", "{ val t = new Tr { def abs: Int = { hits = hits + pi; hits } }; t.abs + hits }", "Int", ""),
    Shape("by-name-local", "def twice(x: => Int): Int = x + x", "twice(pi + 1)", "Int", ""),
    Shape("throw-branch", "", "(if pb then ps else throw new IllegalStateException(ps))", "String", ""),
    Shape("throw-statement", "", "{ if pi > 100 then throw new RuntimeException(\"big\"); pi }", "Int", ""),
    Shape("throw-both", "", "(if pb then throw new RuntimeException(\"a\") else throw new IllegalStateException(\"b\"))", "Unit", ""),
    Shape("tail-loop-own", "", "tl(pi, 2L)", "Long", "members"), Shape("tail-loop-own-block", "", "tls(pi, ps)", "String", "members"),
    Shape("tail-loop-other", "", "other.tl(pi, 1L)", "Long", ""), Shape("tail-loop-top", "", "toploop(pi, 0)", "Int", ""),
    Shape("tail-loop-local", "def lloop(n: Int, acc: Int): Int = if n <= 0 then acc else lloop(n - 1, acc + pi)", "lloop(3, 0)", "Int", ""),
    Shape("array-format", "", "f\"$pi%d and $ps%s and $pd%.1f\"", "String", ""),
    Shape("array-packed-filter", "", "(for (x <- List(pi); y = ps + x; if y.length > 0) yield y + x)", "List[String]", ""),
    Shape("array-packed-generator", "", "(for (x <- List(pi); y = x + pd; z <- List(y, pd)) yield x + y + z)", "List[Double]", ""),
    Shape("array-tuple-xxl", "", "(pi, ps, pl, pd, pf, pb, pc, pby, psh, pa, pt, pv, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23)", "Any", ""),
    Shape("array-format-own", "", "f\"$fi%d and $fs%s\"", "String", "members"),
    Shape("seq-int", "", "other.va() + other.va(pi) + other.va(1, pi, 3)", "Any", "method"),
    Shape("seq-long", "", "other.vl() + other.vl(pl) + other.vl(1L, pl, 3)", "Any", "method"),
    Shape("seq-double", "", "other.vd() + other.vd(pd) + other.vd(1, pd, 2.5, pf)", "Any", "method"),
    Shape("seq-float", "", "other.vf() + other.vf(pf) + other.vf(1, pf, 2.5f)", "Any", "method"),
    Shape("seq-boolean", "", "other.vb() + other.vb(pb) + other.vb(true, pb, false)", "Any", "method"),
    Shape("seq-char", "", "other.vc() + other.vc(pc) + other.vc('a', pc, 'z')", "Any", "method"),
    Shape("seq-byte", "", "other.vby() + other.vby(pby) + other.vby(1, pby, 3)", "Any", "method"),
    Shape("seq-short", "", "other.vsh() + other.vsh(psh) + other.vsh(1, psh, 3)", "Any", "method"),
    Shape("seq-unit", "", "other.vu() + other.vu(()) + other.vu((), other.mu())", "Any", "method"),
    Shape("seq-string", "", "other.vs() + other.vs(ps) + other.vs(\"a\", ps, \"z\")", "Any", "method"),
    Shape("seq-any", "", "other.vaa() + other.vaa(pa) + other.vaa(pi, ps, pv, pd, pa, pt)", "Any", "method"),
    Shape("seq-value-class", "", "other.vv() + other.vv(pv) + other.vv(pv, new V(pi), pv)", "Any", "method"),
    Shape("seq-generic", "", "other.vg() + other.vg(pt) + other.vg(pi, pi) + other.vg(ps, \"z\") + other.vg(pv, pv) + other.vg(pi, ps)", "Any", "method"),
    Shape("seq-mixed", "", "other.va(1, pi) + other.vs(ps, \"z\") + other.vaa(pi, ps, pv) + other.vd(1, pd) + other.vg(pt)", "Int", ""),
    Shape("seq-own", "", "vs(fs, \"z\") + vl(fv, 2L) + vg(fi, fs) + va()", "Int", "members"), Shape("seq-of", "", "Seq(pi, 2)", "Seq[Int]", "method"),
    Shape("seq-of-typed", "", "Seq[Long](1, pl, 3)", "Seq[Long]", "method"), Shape("seq-of-none", "", "Seq[String]()", "Seq[String]", "method"),
    Shape("list-of", "", "List(ps, \"b\")", "List[String]", "method"), Shape("list-of-generic", "", "List(pt, pt)", "List[T]", "method"),
    Shape("list-of-value-class", "", "List(pv, new V(pi))", "List[V]", "method"),
    Shape("vector-of", "", "Vector(pd, 2.5, 1)", "Vector[Double]", "method"),
    Shape("collections-of", "", "(Seq(pi, 2), List(ps, \"b\"), Vector(pd, 2.5), Set(pc), Map(pi -> ps))", "Any", ""),
    Shape("array-of-numbers", "", "(Array(pi, 3), Array(pl, 2L), Array(pd, 1.5), Array(pf, 1.5f))", "Any", ""),
    Shape("array-of-small", "", "(Array(pb, true), Array(pc, 'b'), Array(pby, pby), Array(psh, psh), Array((), ()))", "Any", ""),
    Shape("array-of-references", "", "(Array(ps, \"b\"), Array[Any](pi, ps), Array(pv, pv), Array(pi), Array(pbox))", "Any", ""),
    Shape("java-array-objects", "", "String.format(\"%s %s\", pa, ps)", "String", ""),
    Shape("java-array-none", "", "String.format(ps)", "String", "method"),
    Shape("java-array-interface", "", "java.lang.String.join(\"-\", ps, ps)", "String", ""),
    Shape("java-array-strings", "", "java.nio.file.Paths.get(ps, ps, \"x\")", "java.nio.file.Path", "method"),
    Shape("java-array-ints", "", "java.util.stream.IntStream.of(pi, 2)", "java.util.stream.IntStream", ""),
    Shape("java-array-longs", "", "java.util.stream.LongStream.of(pl, 2L, 3)", "java.util.stream.LongStream", "method"),
    Shape("java-array-doubles", "", "java.util.stream.DoubleStream.of(pd, 1.5, 2)", "java.util.stream.DoubleStream", "method"),
    Shape("java-array-spliced-ints", "", "java.util.stream.IntStream.of(Array(pi, 2)*)", "java.util.stream.IntStream", ""),
    Shape("java-array-spliced-interface", "", "java.lang.String.join(\"-\", Array(ps, ps)*)", "String", "method"),
    Shape("java-array-spliced-objects", "", "String.format(ps, Array[AnyRef](ps, pbox)*)", "String", "method"),
    Shape("java-array-copied-objects", "", "String.format(ps, List(pa, ps)*)", "String", ""),
    Shape("java-array-copied-interface", "", "java.lang.String.join(\"-\", List(ps, ps)*)", "String", "method"),
    Shape("java-array-copied-ints", "", "java.util.stream.IntStream.of(List(pi, 2)*)", "java.util.stream.IntStream", ""),
    Shape("java-array-copied-longs", "", "java.util.stream.LongStream.of(List(pl, 2L)*)", "java.util.stream.LongStream", "method"),
    Shape("java-array-copied-doubles", "", "java.util.stream.DoubleStream.of(List(pd, 1.5)*)", "java.util.stream.DoubleStream", "method"),
    Shape("java-array-wrapped-copied", "", "String.format(ps, Array(pi, 2)*)", "String", "method"),
    Shape("java-array-null", "", "String.format(ps, null*)", "String", "method"),
    Shape("java-array-chars", "", "new String(Array(pc, 'b'))", "String", ""),
    Shape("java-array-bytes", "", "new java.io.ByteArrayInputStream(Array(pby, pby))", "java.io.ByteArrayInputStream", "method"),
    Shape("array-argument", "", "other.arr(Array(pi, 2)) + other.arrs(Array(ps))", "Int", ""),
    Shape("spread-sequence", "", "other.va(List(pi, 2)*) + other.vs(Seq(ps)*)", "Int", ""),
    Shape("spread-array-int", "", "other.va(Array(pi, 2)*)", "Int", ""), Shape("spread-array-long", "", "other.vl(Array(pl, 2L)*)", "Int", "method"),
    Shape("spread-array-double", "", "other.vd(Array(pd, 1.5)*)", "Int", "method"),
    Shape("spread-array-float", "", "other.vf(Array(pf, 1.5f)*)", "Int", "method"),
    Shape("spread-array-boolean", "", "other.vb(Array(pb, true)*)", "Int", "method"),
    Shape("spread-array-char", "", "other.vc(Array(pc, 'b')*)", "Int", "method"),
    Shape("spread-array-byte", "", "other.vby(Array(pby, pby)*)", "Int", "method"),
    Shape("spread-array-short", "", "other.vsh(Array(psh, psh)*)", "Int", "method"),
    Shape("spread-array-unit", "", "other.vu(Array((), ())*)", "Int", "method"),
    Shape("spread-array-string", "", "other.vs(Array(ps, \"b\")*)", "Int", ""),
    Shape("spread-array-string-as-any", "", "other.vaa(Array(ps, \"b\")*)", "Int", "method"),
    Shape("spread-array-any", "", "other.vaa(Array[Any](pi, ps)*)", "Int", "method"),
    Shape("spread-array-generic", "", "other.vg(Takes.one(pt)*)", "Int", "method"),
    Shape("spread-array-value-class", "", "other.vv(Array(pv, pv)*)", "Int", "method"),
    Shape("spread-array-nested", "", "other.vaa(Array(Array(pi), Array(2))*)", "Int", "method"),
    Shape("spread-own", "", "va(Array(fi, 2)*) + vs(Array(fs, \"z\")*)", "Int", "members"),
    // The array's code holds a slot (an extension's receiver, a reordered argument) and a local follows the call.
    Shape("spread-held-receiver", "", "{ val sh: Int = other.va(Array(pv.twice.u, 2)*); sh + pi }", "Int", ""),
    Shape("spread-held-order", "", "{ val so: Int = other.vs(Array(pi +: ps)*); so + pi }", "Int", ""),
    Shape("super-call", "", "super.im(pi)", "Int", "members"), Shape("super-def", "", "super.sd", "Int", "members"),
    Shape("super-default", "", "super.td()", "Int", "members"), Shape("super-default-prior", "", "super.tdp(pi)()", "Int", "members"),
    Shape("bridge-value-class", "", "((new GivesV: Gives[V]).give().u + (new TakesV: TakesIn[V]).take(pv))", "Int", "method"),
    Shape("jar-trait-super-object", "", "new CloneCell(pi).clone().n", "Int", "method"),
    Shape("super-tail-receiver", "", "new TailBoth().loop(pi, pb)", "Boolean", "method"),
    Shape("super-named-trait", "", "super[Tr].im(pi)", "Int", "members"), Shape("super-object-member", "", "super.hashCode", "Int", "members"),
    // Not a Java interface's default method through `super`: the typer does not find it among the parents today.
    Shape("super-class-member", "class Lb(n: Int) extends Base(n) { def g(x: Int): Int = super.inherited(x) }", "new Lb(pi).g(2)", "Int", ""),
    Shape("super-java-class", "class Lal extends java.util.ArrayList[String] { def g: Int = { super.add(\"a\"); super.size() } }", "new Lal().g", "Int", "method"),
    Shape("early-parameter", "", "vx", "Int", "constructor"),
    Shape("value-underlying", "", "u", "Int", "value"), Shape("value-own-call", "", "vm()", "Int", "value"),
    Shape("try-catch", "", "(try other.m0() catch { case _: RuntimeException => 0 })", "Int", ""),
    Shape("try-finally", "", "(try ps finally other.mu())", "String", ""),
    Shape("try-catch-finally", "", "(try pd catch { case _: Exception => 1.5 } finally other.mu())", "Double", ""),
    Shape("ret-statement", "def rst(x: Int): Int = { if x > 0 then return 1; x }", "rst(pi)", "Int", ""),
    Shape("ret-branch-int", "def rb(x: Int): Int = { val y: Int = if x > 0 then return x else x - 1; y * 2 }", "rb(pi)", "Int", ""),
    Shape("ret-branch-long", "def rbl(x: Long): Long = { val y: Long = if x > 0L then return x else x - 1L; y * 2L }", "rbl(pl)", "Long", ""),
    Shape("ret-branch-double", "def rbd(x: Double): Double = { val y: Double = if x > 0.0 then return x else 1.5; y * 2.0 }", "rbd(pd)", "Double", ""),
    Shape("ret-branch-boolean", "def rbb(x: Int): Boolean = { val y: Boolean = if x > 0 then return true else x < -3; !y }", "rbb(pi)", "Boolean", ""),
    Shape("ret-branch-char", "def rbc(x: Char): Char = { val y: Char = if x > 'a' then return x else 'b'; y }", "rbc(pc)", "Char", ""),
    Shape("ret-branch-string", "def rbs(x: Int): String = { val y: String = if x > 0 then return ps else \"n\"; y + x }", "rbs(pi)", "String", ""),
    Shape("ret-verify", "def rvf(x: Int): String = { val y: String = if x > 0 then return \"p\" else \"n\"; y + x }", "rvf(pi) + rvf(-pi)", "String", ""),
    Shape("ret-branch-unit", "def rbu(x: Int): Unit = { val y: Unit = if x > 0 then return else other.mu(); y }", "rbu(pi)", "Unit", ""),
    Shape("ret-branch-any", "def rba(x: Int): Any = { val y: Any = if x > 0 then return x else ps; y }", "rba(pi)", "Any", ""),
    Shape("ret-branch-generic", "def rbg[A](a: A, b: Boolean): A = { val y: A = if b then return a else a; y }", "rbg(pt, pb)", "T", ""),
    Shape("ret-branch-value-class", "def rbv(v: V, b: Boolean): V = { val y: V = if b then return v else new V(1); y }", "rbv(pv, pb)", "V", ""),
    Shape("ret-else", "def re(x: Int): Int = { val y: Int = if x > 0 then x else return 2; y + 1 }", "re(pi)", "Int", ""),
    Shape("ret-both", "def rbo(x: Int): Int = { val y: Int = if x > 0 then return 1 else return 2; y }", "rbo(pi)", "Int", ""),
    Shape("ret-argument", "def ra(x: Int): Int = Math.max(if x > 0 then return 1 else x, 3)", "ra(pi)", "Int", ""),
    Shape("ret-operands", "def ro(x: Int): String = ps + (if x > 0 then return \"r\" else \"e\")", "ro(pi)", "String", ""),
    Shape("ret-block", "def rk(x: Int): Int = { val y: Int = { other.mu(); return x }; y }", "rk(pi)", "Int", ""),
    Shape("ret-branch-block", "def rbk(x: Int): Int = { val y: Int = if x > 0 then { val z = x * 2; return z } else 1; y + 1 }", "rbk(pi)", "Int", ""),
    Shape("ret-branch-nested", "def rbn(x: Int): Int = { val y: Int = if x > 0 then (if x > 5 then return 1 else return 2) else x; y }", "rbn(pi)", "Int", ""),
    Shape("ret-branch-statement", "def rbt(x: Int): Int = { val y: Int = if x > 0 then { if x > 5 then return 1; 5 } else 1; y }", "rbt(pi)", "Int", ""),
    Shape("ret-tail", "def rt(x: Int): Int = return x + 1", "rt(pi)", "Int", ""),
    Shape("ret-tail-unit", "def rtu(x: Int): Unit = return", "rtu(pi)", "Unit", ""),
    Shape("ret-tail-if", "def rti(x: Int): Int = return (if x > 0 then x else 2)", "rti(pi)", "Int", ""),
    Shape("ret-tail-string", "def rts2(x: Int): String = { other.mu(); return ps + x }", "rts2(pi)", "String", ""),
    Shape("ret-while", "def rw(x: Int): Int = { var i: Int = 0; while i < 10 do { if i == x then return i * 2; i = i + 1 }; -1 }", "rw(pi)", "Int", ""),
    Shape("ret-match", "def rm(x: Int): Int = { val y: Int = x match { case 0 => return 5; case n => n }; y + 1 }", "rm(pi)", "Int", ""),
    Shape("ret-lambda", "def rl(xs: List[Int]): Int = { xs.foreach(x => if x > 0 then return x); 0 }", "rl(List(pi))", "Int", ""),
    Shape("ret-lambda-value", "def rlv(xs: List[Int]): Int = { xs.map(x => if x > 0 then return x else x + 1); 0 }", "rlv(List(pi))", "Int", ""),
    Shape("try-ret-finally", "def rf(x: Int): Int = try { val y: Int = if x > 0 then return 1 else x; y } finally other.mu()", "rf(pi)", "Int", ""),
    Shape("try-ret-finally-long", "def rfl(x: Long): Long = try { val y: Long = if x > 0L then return x else 2L; y } finally other.mu()", "rfl(pl)", "Long", ""),
    Shape("try-ret-finally-string", "def rfs(x: Int): String = try { val y: String = if x > 0 then return ps else \"n\"; y } finally other.mu()", "rfs(pi)", "String", ""),
    Shape("try-ret-finally-unit", "def rfu(x: Int): Unit = try { if x > 0 then return; other.mu() } finally other.mu()", "rfu(pi)", "Unit", ""),
    Shape("try-ret-catch", "def rc(x: Int): Int = try { val y: Int = if x > 0 then return 1 else x; y } catch { case _: RuntimeException => 0 }", "rc(pi)", "Int", ""),
    Shape("try-ret-in-case", "def rcc(x: Int): Int = try other.m0() catch { case _: RuntimeException => return 7 } finally other.mu()", "rcc(pi)", "Int", ""),
    Shape("try-ret-in-finalizer", "def rif(x: Int): Int = { try other.mu() finally { if x > 3 then return 9 }; x }", "rif(pi)", "Int", ""),
    Shape("try-ret-nested", "def rn(x: Int): Int = try { try { if x > 0 then return 1; 2 } finally other.mu() } finally other.mu()", "rn(pi)", "Int", ""),
    Shape("try-ret-operands", "def rop(x: Int): String = try { ps + (if x > 0 then return \"r\" else \"e\") } finally other.mu()", "rop(pi)", "String", ""),
    Shape("try-ret-lambda", "def rtl(xs: List[Int]): Int = try { xs.foreach(x => if x > 0 then return x); 0 } finally other.mu()", "rtl(List(pi))", "Int", ""),
    Shape("ret-sync", "def rsy(x: Int): Int = other.synchronized { val y: Int = if x > 0 then return x else 1; y + 1 }", "rsy(pi)", "Int", ""),
    Shape("ret-sync-unit", "def rsu(x: Int): Unit = other.synchronized { if x > 0 then return; other.mu() }", "rsu(pi)", "Unit", ""),
    Shape("ret-sync-nested", "def rsn(x: Int): String = other.synchronized { sub.synchronized { if x > 1 then return ps }; \"n\" }", "rsn(pi)", "String", ""),
    Shape("ret-tail-call", "def rtc(n: Int): Int = if n == 0 then 0 else return rtc(n - 1)", "rtc(pi)", "Int", ""),
    Shape("ret-tail-call-wraps", "def rtw2(n: Int): Int = return (if n == 0 then 0 else rtw2(n - 1))", "rtw2(pi)", "Int", ""),
    Shape("ret-tail-call-block", "def rtbk(n: Int): Int = return (if n > 0 then { val m = n - 1; rtbk(m) } else 7)", "rtbk(pi)", "Int", ""),
    Shape("ret-tail-call-nested", "def rtn(n: Int): Int = return (if n > 0 then (if n % 2 == 0 then rtn(n - 1) else rtn(n - 2)) else 7)", "rtn(pi)", "Int", ""),
    Shape("ret-tail-call-statement", "def rts3(n: Int): Int = { if n > 0 then return rts3(n - 1); 0 }", "rts3(pi)", "Int", ""),
    Shape("ret-tail-call-val", "def rtv(n: Int): Long = { val r: Long = if n > 0 then return rtv(n - 1) else 7L; r }", "rtv(pi)", "Long", ""),
    Shape("ret-tail-call-case", "def rtm(n: Int): Int = n match { case 0 => 0; case _ => return rtm(n - 1) }", "rtm(pi)", "Int", ""),
    Shape("ret-tail-call-case-value", "def rtm2(n: Int): Int = { val r: Int = n match { case 0 => 0; case _ => return rtm2(n - 1) }; r }", "rtm2(pi)", "Int", ""),
    Shape("ret-tail-call-operands", "def rto(n: Int): String = ps + (if n > 0 then return rto(n - 1) else \"e\")", "rto(pi)", "String", ""),
    Shape("ret-tail-call-catch", "def rtk(n: Int): Int = try { if n > 0 then throw new RuntimeException(); 0 } catch { case _: RuntimeException => return rtk(n - 1) }", "rtk(pi)", "Int", ""),
    Shape("ret-tail-call-while", "def rtw(n: Int): Int = { var i: Int = 0; while i < 2 do { if n > 0 then return rtw(n - 1); i = i + 1 }; i }", "rtw(pi)", "Int", ""),
    Shape("ret-tail-call-lambda", "def rtl2(n: Int): Int = { List(1).foreach(x => if n > 0 then return rtl2(n - 1)); 0 }", "rtl2(pi)", "Int", ""),
    Shape("ret-tail-call-sync", "def rty(n: Int): Int = other.synchronized { if n > 0 then return rty(n - 1); 0 }", "rty(pi)", "Int", ""),
    Shape("ret-tail-call-and", "def rta(n: Int): Boolean = { if n > 0 then return (n > -1 && rta(n - 1)); true }", "rta(pi)", "Boolean", ""),
    Shape("ret-tail-call-or", "def rtor(n: Int): Boolean = { if n > 0 then return (n < -1 || rtor(n - 1)); false }", "rtor(pi)", "Boolean", ""),
    Shape("ret-tail-call-lazy", "def rtlz(n: Int): Int = { lazy val x: Int = { if n > 0 then return rtlz(n - 1); 5 }; x + 1 }", "rtlz(pi)", "Int", ""),
    Shape("ret-tail-call-capture", "def rtcp(k: Int): Int = { if k > 0 then return rtcp(k - 1); pi * 10 + k }", "rtcp(3)", "Int", ""),
    Shape("try-ret-tail-call-finally", "def rtf(n: Int): Int = try { if n > 0 then return rtf(n - 1); 0 } finally other.mu()", "rtf(pi)", "Int", ""),
    Shape("try-ret-tail-call-body", "def rtb(n: Int): Int = try { if n > 0 then return rtb(n - 1); 0 } catch { case _: RuntimeException => 1 }", "rtb(pi)", "Int", ""),
    Shape("try-ret-sync", "def rts(x: Int): Double = try other.synchronized { val y: Double = if x > 0 then return pd else 1.5; y } finally other.mu()", "rts(pi)", "Double", ""),
    Shape("sync-int", "", "other.synchronized(pi + 1)", "Int", ""), Shape("sync-string", "", "ps.synchronized(ps + pi)", "String", ""),
    Shape("sync-unit", "", "other.synchronized(other.mu())", "Unit", ""), Shape("sync-long", "", "sub.synchronized(pl * 2L)", "Long", ""),
    Shape("sync-double", "", "pbox.synchronized(pd)", "Double", ""), Shape("sync-boolean", "", "other.synchronized(pb && pi > 0)", "Boolean", ""),
    Shape("sync-generic", "", "other.synchronized(pt)", "T", ""), Shape("sync-value-class", "", "other.synchronized(pv)", "V", ""),
    Shape("sync-widened", "", "other.synchronized(if pb then pi else ps)", "Any", ""), Shape("sync-module", "", "O.synchronized(O.k)", "Int", ""),
    Shape("sync-this", "", "synchronized(fi + 1)", "Int", "members"), Shape("sync-nested", "", "other.synchronized(sub.synchronized(pl))", "Long", ""),
    Shape("sync-throw", "", "other.synchronized(if pb then pi else throw new RuntimeException(\"s\"))", "Int", ""),
    Shape("sync-lock-call", "", "Takes.id(other).synchronized(ps)", "String", ""),
    Shape("sync-lock-lambda", "val lk = () => sub", "lk().synchronized(pi)", "Int", ""),
    Shape("sync-lambda-body", "", "other.synchronized({ val g = (x: Int) => x + pi; g(2) })", "Int", ""),
    Shape("sync-captured", "var sc: Int = pi; val bump = () => { sc = sc + 1 }", "other.synchronized({ bump(); sc })", "Int", ""),
    Shape("sync-return", "def sr(x: Int): Int = other.synchronized { if x > 0 then return x; x + 1 }", "sr(pi)", "Int", ""),
    Shape("sync-return-unit", "def su(x: Int): Unit = other.synchronized { if x > 0 then return; other.mu() }", "su(pi)", "Unit", ""),
    Shape("sync-return-value", "def sv(x: Long): Long = sub.synchronized { val y: Long = if x > 0L then return x else x - 1L; y * 2L }", "sv(pl)", "Long", ""),
    Shape("sync-return-nested", "def sn(x: Int): String = other.synchronized { sub.synchronized { if x > 1 then return ps }; \"n\" }", "sn(pi)", "String", ""),
    Shape("try-around-sync", "", "(try other.synchronized(ps) finally other.mu())", "String", ""),
    Shape("try-in-sync", "", "other.synchronized(try pi finally other.mu())", "Int", ""),
    Shape("try-sync-return", "def st(x: Int): String = try other.synchronized { if x > 0 then return ps; \"n\" } finally other.mu()", "st(pi)", "String", ""),
    Shape("lazy-int", "lazy val lzi: Int = pi + 1", "lzi", "Int", ""), Shape("lazy-long", "lazy val lzl: Long = pl * 2L", "lzl", "Long", ""),
    Shape("lazy-double", "lazy val lzd: Double = pd + 1.0", "lzd", "Double", ""),
    Shape("lazy-primitives", "lazy val zf: Float = pf; lazy val zb: Boolean = pb; lazy val zc: Char = pc; lazy val zy: Byte = pby; lazy val zs: Short = psh",
      "(\"\" + zf + zb + zc + zy + zs)", "String", ""),
    Shape("lazy-unit", "lazy val lzu: Unit = other.mu()", "lzu", "Unit", ""), Shape("lazy-generic", "lazy val lzt: T = pt", "lzt", "T", ""),
    Shape("lazy-value-class", "lazy val lzv: V = new V(pi)", "lzv", "V", ""),
    Shape("lazy-block", "", "{ lazy val bl: Int = pi + 1; bl + 1 }", "Int", ""),
    Shape("lazy-mutual", "", "{ lazy val a1: Int = b1 + 1; lazy val b1: Int = pi; a1 }", "Int", ""),
    Shape("lazy-captured", "", "{ lazy val lc: String = ps + pi; val f = () => lc; f() }", "String", ""),
    Shape("lazy-forward", "", "{ def rd: Int = lr + 1; lazy val lr: Int = pi; rd }", "Int", "method"),
    Shape("lazy-object", "", "{ object LO { def v: Int = 3 }; LO.v + pi }", "Int", "method"),
    Shape("lazy-return", "def lret(x: Int): Int = { lazy val q: Int = if x > 0 then return 7 else x; q + 1 }", "lret(pi)", "Int", ""),
    Shape("lazy-two", "lazy val l1: Int = pi + 1; val mid: Int = l1 * 2; lazy val l2: String = ps + mid", "l2 + l1", "String", ""),
    Shape("lazy-chain", "lazy val lk1: Int = pi + 1; lazy val lk2: Int = lk1 + 1", "lk2", "Int", ""),
    Shape("lazy-nested", "", "{ lazy val ln: Int = pi + 2; { lazy val lm: Long = ln + 1L; lm } }", "Long", ""),
    Shape("lazy-finally", "val lfin: Int = try { lazy val lq: Int = pi; lq } finally { lazy val lr: String = ps; println(lr) }", "lfin", "Int", ""),
    Shape("lazy-concat", "", "({ lazy val lz2: String = ps + 1; lz2 } + pi)", "String", ""),
    Shape("lazy-tail", "def ltl(n: Int): Int = { lazy val h: Int = n * 2; if n <= 0 then h else ltl(n - 1) }", "ltl(pi)", "Int", ""),
    Shape("sync-lazy", "", "other.synchronized({ lazy val sz: Int = pi; sz + 1 })", "Int", ""),
    Shape("try-lazy-finally", "", "(try ps finally { lazy val fz: Int = pi; Takes.any(fz) })", "String", ""),
    Shape("lazy-this", "lazy val lzf: Int = fi + 1", "lzf", "Int", "members"),
    Shape("lazy-in-lambda", "lazy val ll2: Int = pi * 2; val f2 = () => ll2 + 1", "f2()", "Int", ""),
    Shape("lazy-in-def", "lazy val ld2: String = ps + \"d\"; def rd2(x: Int): String = ld2 + x", "rd2(pi)", "String", ""),
    Shape("lazy-key-lambda", "def lk(xs: List[Int]): Int = { lazy val q: Int = { xs.foreach(x => if x > 0 then return x); 0 }; q + 1 }", "lk(List(pi))", "Int", ""),
    Shape("class-of-string", "", "classOf[String]", "Class[String]", ""), Shape("class-of-class", "", "classOf[Peer]", "Class[Peer]", ""),
    Shape("class-of-trait", "", "classOf[Tr]", "Class[Tr]", ""), Shape("class-of-generic", "", "classOf[Box[Int]]", "Class[Box[Int]]", ""),
    Shape("class-of-java", "", "classOf[java.lang.Runnable]", "Class[java.lang.Runnable]", ""),
    Shape("class-of-value-class", "", "classOf[V]", "Class[V]", ""), Shape("class-of-any", "", "classOf[Any]", "Class[Any]", ""),
    Shape("class-of-any-ref", "", "classOf[AnyRef]", "Class[AnyRef]", ""), Shape("class-of-nothing", "", "classOf[Nothing]", "Class[Nothing]", ""),
    Shape("class-of-null", "", "classOf[Null]", "Class[Null]", ""), Shape("class-of-unit", "", "classOf[Unit]", "Class[Unit]", ""),
    Shape("class-of-int", "", "classOf[Int]", "Class[Int]", ""), Shape("class-of-long", "", "classOf[Long]", "Class[Long]", ""),
    Shape("class-of-double", "", "classOf[Double]", "Class[Double]", ""), Shape("class-of-float", "", "classOf[Float]", "Class[Float]", ""),
    Shape("class-of-bool", "", "classOf[Boolean]", "Class[Boolean]", ""), Shape("class-of-char", "", "classOf[Char]", "Class[Char]", ""),
    Shape("class-of-byte", "", "classOf[Byte]", "Class[Byte]", ""), Shape("class-of-short", "", "classOf[Short]", "Class[Short]", ""),
    Shape("class-of-array-int", "", "classOf[Array[Int]]", "Class[Array[Int]]", ""),
    Shape("class-of-array-string", "", "classOf[Array[String]]", "Class[Array[String]]", ""),
    Shape("class-of-array-class", "", "classOf[Array[Peer]]", "Class[Array[Peer]]", ""),
    Shape("class-of-array-value-class", "", "classOf[Array[V]]", "Class[Array[V]]", ""),
    Shape("class-of-array-any", "", "classOf[Array[Any]]", "Class[Array[Any]]", ""),
    Shape("class-of-array-nested", "", "classOf[Array[Array[Double]]]", "Class[Array[Array[Double]]]", ""),
    Shape("class-tag-class", "", "summon[scala.reflect.ClassTag[Peer]]", "scala.reflect.ClassTag[Peer]", ""),
    Shape("class-tag-array", "", "summon[scala.reflect.ClassTag[Array[String]]]", "scala.reflect.ClassTag[Array[String]]", ""),
    Shape("cast-down", "", "pa.asInstanceOf[Sub]", "Sub", ""), Shape("cast-down-base", "", "(sub: Base).asInstanceOf[Sub]", "Sub", ""),
    Shape("cast-up", "", "sub.asInstanceOf[Base]", "Base", ""), Shape("cast-trait", "", "pa.asInstanceOf[Tr]", "Tr", ""),
    Shape("cast-string", "", "pa.asInstanceOf[String]", "String", ""), Shape("cast-generic", "", "pa.asInstanceOf[T]", "T", ""),
    Shape("cast-generic-int", "", "pi.asInstanceOf[T]", "T", ""), Shape("cast-generic-string", "", "ps.asInstanceOf[T]", "T", ""),
    Shape("cast-box", "", "pa.asInstanceOf[Box[Int]]", "Box[Int]", ""),
    Shape("cast-java", "", "pa.asInstanceOf[java.util.List[String]]", "java.util.List[String]", ""),
    Shape("cast-function", "", "pa.asInstanceOf[Int => Int]", "Int => Int", ""),
    Shape("cast-function-wide", "", "pa.asInstanceOf[" + wideFunction + "]", wideFunction, ""),
    Shape("cast-intersection", "", "pa.asInstanceOf[Base & Tr]", "Base & Tr", ""),
    Shape("cast-unbox-int", "", "pa.asInstanceOf[Int]", "Int", ""), Shape("cast-unbox-long", "", "pa.asInstanceOf[Long]", "Long", ""),
    Shape("cast-unbox-double", "", "pa.asInstanceOf[Double]", "Double", ""), Shape("cast-unbox-float", "", "pa.asInstanceOf[Float]", "Float", ""),
    Shape("cast-unbox-bool", "", "pa.asInstanceOf[Boolean]", "Boolean", ""), Shape("cast-unbox-char", "", "pa.asInstanceOf[Char]", "Char", ""),
    Shape("cast-unbox-byte", "", "pa.asInstanceOf[Byte]", "Byte", ""), Shape("cast-unbox-short", "", "pa.asInstanceOf[Short]", "Short", ""),
    Shape("cast-unbox-generic", "", "pt.asInstanceOf[Int]", "Int", ""), Shape("cast-unbox-string", "", "ps.asInstanceOf[Int]", "Int", ""),
    Shape("cast-box-any", "", "pi.asInstanceOf[Any]", "Any", ""), Shape("cast-convert-widen", "", "pi.asInstanceOf[Long]", "Long", ""),
    Shape("cast-convert-narrow", "", "pd.asInstanceOf[Int]", "Int", ""), Shape("cast-convert-char", "", "pc.asInstanceOf[Int]", "Int", ""),
    Shape("cast-value-class", "", "pa.asInstanceOf[V]", "V", ""), Shape("cast-value-class-any", "", "pv.asInstanceOf[Any]", "Any", ""),
    Shape("cast-value-class-generic", "", "pt.asInstanceOf[V]", "V", ""),
    Shape("cast-unit", "", "pa.asInstanceOf[Unit]", "Unit", ""), Shape("cast-unit-int", "", "pi.asInstanceOf[Unit]", "Unit", ""),
    Shape("cast-null", "", "pa.asInstanceOf[Null]", "Null", ""), Shape("cast-nothing", "", "pa.asInstanceOf[Nothing]", "Nothing", ""),
    Shape("cast-nothing-int", "", "pi.asInstanceOf[Nothing]", "Nothing", ""), Shape("cast-nothing-call", "", "other.m0().asInstanceOf[Nothing]", "Nothing", ""),
    Shape("cast-array-int", "", "pa.asInstanceOf[Array[Int]]", "Array[Int]", ""),
    Shape("cast-array-string", "", "pa.asInstanceOf[Array[String]]", "Array[String]", ""),
    Shape("cast-array-generic", "", "pa.asInstanceOf[Array[T]]", "Array[T]", ""),
    Shape("cast-array-nested", "", "pa.asInstanceOf[Array[Array[String]]]", "Array[Array[String]]", ""),
    Shape("cast-twice", "", "pa.asInstanceOf[Sub].asInstanceOf[Tr]", "Tr", ""), Shape("cast-then-field", "", "pa.asInstanceOf[Sub].bn", "Int", ""),
    Shape("cast-then-call", "", "pa.asInstanceOf[String].length", "Int", ""), Shape("cast-of-call", "", "other.mg(pa).asInstanceOf[String]", "String", ""),
    Shape("cast-of-null", "", "null.asInstanceOf[String]", "String", ""), Shape("cast-of-null-int", "", "null.asInstanceOf[Int]", "Int", ""),
    Shape("cast-of-null-generic", "", "null.asInstanceOf[T]", "T", ""),
    Shape("cast-mirror", "", "summon[scala.deriving.Mirror.ProductOf[Pair]]", "scala.deriving.Mirror.ProductOf[Pair]", ""),
    Shape("cast-value-class-reference", "", "pa.asInstanceOf[VS]", "VS", ""),
    Shape("cast-stored-body", "inline def held(y: Int): String = { case class Held(a: Int, b: String); val c = Held(y, \"x\"); c.toString + (c == Held(y, \"x\")) }",
      "held(pi)", "String", ""),
    Shape("test-tuple", "", "pa.isInstanceOf[(Int, String)]", "Boolean", ""), Shape("test-tuple-family", "", "pa.isInstanceOf[Tuple]", "Boolean", ""),
    Shape("test-array-generic", "", "pa.isInstanceOf[Array[T]]", "Boolean", ""), Shape("test-function", "", "pa.isInstanceOf[Int => Int]", "Boolean", ""),
    Shape("test-union", "", "pa.isInstanceOf[String | Int]", "Boolean", ""), Shape("test-intersection", "", "pa.isInstanceOf[Base & Tr]", "Boolean", ""),
    Shape("test-null", "", "pa.isInstanceOf[Null]", "Boolean", ""), Shape("test-anyref", "", "pa.isInstanceOf[AnyRef]", "Boolean", ""),
    Shape("test-anyval", "", "pa.isInstanceOf[AnyVal]", "Boolean", ""), Shape("test-singleton", "", "pa.isInstanceOf[O.type]", "Boolean", ""),
    Shape("test-literal", "", "pa.isInstanceOf[1]", "Boolean", ""), Shape("test-literal-string", "", "pa.isInstanceOf[\"s\"]", "Boolean", ""),
    Shape("test-inner", "", "pa.isInstanceOf[outer.Inner]", "Boolean", ""), Shape("test-boxes", "", "(pa.isInstanceOf[Long] || pa.isInstanceOf[Double] || pa.isInstanceOf[Char] || pa.isInstanceOf[Boolean] || pa.isInstanceOf[Unit] || pa.isInstanceOf[Byte] || pa.isInstanceOf[Short] || pa.isInstanceOf[Float])", "Boolean", ""),
    Shape("test-proven", "", "pbox.isInstanceOf[Box[Int]]", "Boolean", ""), Shape("test-value-class", "", "pa.isInstanceOf[V]", "Boolean", ""),
    Shape("test-of-value-class", "", "(pv: Any).isInstanceOf[V]", "Boolean", ""),
    Shape("match-type-string", "", "(pa match { case s: String => s.length; case _ => 0 })", "Int", ""),
    Shape("match-type-boxes", "", "(pa match { case i: Int => i.toLong; case l: Long => l; case d: Double => d.toLong; case c: Char => c.toLong; case b: Boolean => 1L; case _ => 0L })", "Long", ""),
    Shape("match-type-class", "", "(pa match { case p: Peer => p.fi; case t: Tr => t.abs; case _ => 0 })", "Int", ""),
    Shape("match-type-tuple", "", "(pa match { case t: (Int, String) @unchecked => t._1; case _ => 0 })", "Int", ""),
    Shape("match-tuple", "", "((pi, ps) match { case (a, b) => b + a })", "String", ""),
    Shape("match-type-array", "", "(pa match { case a: Array[Int] => a.length; case a: Array[String] => a.length; case a: Array[T] @unchecked => 2; case _ => 0 })", "Int", ""),
    Shape("match-type-function", "", "(pa match { case f: (Int => Int) @unchecked => f(1); case _ => 0 })", "Int", ""),
    Shape("match-type-union", "", "(pa match { case u: (String | Int) => 1; case x: (Base & Tr) => x.bn; case _ => 0 })", "Int", ""),
    Shape("match-singleton", "", "(pa match { case O => 1; case _: O.type => 2; case _ => 0 })", "Int", ""),
    Shape("match-inner", "", "(pa match { case i: outer.Inner => i.z; case _ => 0 })", "Int", ""),
    Shape("match-int", "", "(pi match { case 1 => \"a\"; case 2 | 3 => \"b\"; case _ => \"c\" })", "String", ""),
    Shape("match-long", "", "(pl match { case 2L => 1; case _ => 2 })", "Int", ""),
    Shape("match-double", "", "(pd match { case 3.5 => 1; case _ => 2 })", "Int", ""),
    Shape("match-float", "", "(pf match { case 4.5f => 1; case _ => 2 })", "Int", ""),
    Shape("match-char", "", "(pc match { case 'c' => 1; case 'd' => 2; case _ => 3 })", "Int", ""),
    Shape("match-bool", "", "(pb match { case true => 1; case false => 2 })", "Int", ""),
    Shape("match-byte", "", "(pby match { case 5 => 1; case _ => 2 })", "Int", ""),
    Shape("match-string", "", "(ps match { case \"s\" => 1; case \"t\" => 2; case _ => 3 })", "Int", ""),
    Shape("match-null", "", "(pa match { case null => 1; case _ => 2 })", "Int", ""),
    Shape("match-null-alternative", "", "(ps match { case null | \"\" => 1; case _ => 2 })", "Int", ""),
    Shape("match-bind", "", "(pa match { case s: String => s; case x => x.toString })", "String", ""),
    Shape("match-bind-at", "", "(pa match { case s @ (_: String) => s.toString; case _ => \"n\" })", "String", ""),
    Shape("match-bind-captured", "", "(pa match { case x => (() => x.toString)() })", "String", ""),
    Shape("match-case-class", "", "(pa match { case Pair(a, \"s\") => \"s\" + a; case Pair(a, b) => b + a; case _ => \"n\" })", "String", ""),
    Shape("match-case-class-nested", "", "(Option(Nest(Pair(pi, ps), pv)) match { case Some(Nest(Pair(a, _), v)) => a + v.u; case _ => 0 })", "Int", ""),
    Shape("match-value-class", "", "(pv match { case v: V => v.u })", "Int", ""),
    Shape("match-value-class-bind", "", "(pv match { case v if v.u > 0 => v.u; case _ => 0 })", "Int", ""),
    Shape("match-value-class-any", "", "((pv: Any) match { case v: V => v.u; case _ => 0 })", "Int", ""),
    Shape("match-value-class-field", "", "(Option(pv) match { case Some(v) => v.u; case None => 0 })", "Int", ""),
    Shape("match-value-class-result", "", "(pi match { case 1 => pv; case _ => new V(2) })", "V", ""),
    // An intersection of two value classes erases to the first by scalac's order, `V`, whichever is written first.
    Shape("match-value-class-intersection", "val vi: VD & V = pv.asInstanceOf[VD & V]", "(vi match { case y if y.u > 0 => y.u })", "Int", ""),
    Shape("match-value-class-intersection-test", "val vi: VD & V = pv.asInstanceOf[VD & V]", "(vi match { case y: V if y.u > 0 => y.u; case _ => 0 })", "Int", ""),
    Shape("match-enum", "", "((if pb then Color.Red else Color.Green) match { case Color.Red => 1; case Color.Green => 2 })", "Int", ""),
    Shape("match-enum-class", "", "((if pb then Figure.Circle(pi) else Figure.Square(pi)) match { case Figure.Circle(r) => r; case Figure.Square(_) => 0 })", "Int", ""),
    Shape("match-alternatives", "", "(pa match { case _: String | _: Int => 1; case _ => 0 })", "Int", ""),
    Shape("match-seq", "", "(Seq(pi, 2) match { case Seq(a, b) => a + b; case Seq(a, rest*) => a + rest.length; case _ => 0 })", "Int", ""),
    Shape("match-list", "", "(List(pi, 2) match { case h :: t => h + t.length; case Nil => 0 })", "Int", ""),
    Shape("match-array", "", "(Array(pi, 2) match { case Array(a, b) => a + b; case _ => 0 })", "Int", ""),
    Shape("match-option", "", "(Option(ps) match { case Some(s) => s; case None => \"n\" })", "String", ""),
    Shape("match-extractor-boolean", "", "(pi match { case Even() => 1; case _ => 0 })", "Int", ""),
    Shape("match-extractor-product", "", "(pi match { case Halves(a, b) => a + b })", "Int", ""),
    Shape("match-extractor-option", "", "(pi match { case Twice(a, b) if a > 0 => a + b; case _ => 0 })", "Int", ""),
    Shape("match-extractor-seq", "", "(pi match { case Digits(a, b) => a + b; case _ => 0 })", "Int", ""),
    Shape("match-guard", "", "(pi match { case x if x > 0 => 1; case _ => 0 })", "Int", ""),
    Shape("match-guard-bind", "", "(pa match { case s: String if s.length > 0 => s; case _ => \"e\" })", "String", ""),
    Shape("match-nested", "", "(pi match { case 0 => (pa match { case _: String => 1; case _ => 2 }); case _ => 3 })", "Int", ""),
    Shape("match-unit", "", "(pa match { case _: String => other.mu(); case _ => () })", "Unit", ""),
    Shape("match-any", "", "(pi match { case 1 => ps; case _ => pi })", "Any", ""),
    Shape("match-generic", "", "(pi match { case 1 => pt; case _ => pt })", "T", ""),
    Shape("match-statement", "val mr: Int = pi match { case 1 => 2; case _ => 3 }", "mr", "Int", ""),
    Shape("val-pattern-tuple", "val (pa1, pb1) = (pi, ps)", "(pb1 + pa1)", "String", ""),
    Shape("val-pattern-option", "val Some(px) = Option(pi): @unchecked", "px", "Int", ""),
    Shape("val-pattern-cons", "val ph :: pr = List(pi, 2): @unchecked", "(ph + pr.length)", "Int", ""),
    Shape("try-typed", "", "(try pi catch { case e: RuntimeException => 1 })", "Int", ""),
    Shape("try-several", "", "(try pi catch { case e: IllegalStateException => 1; case e: RuntimeException => 2 })", "Int", ""),
    Shape("try-guard", "", "(try pi catch { case e: RuntimeException if e.getMessage == null => 1 })", "Int", ""),
    Shape("try-bind", "", "(try ps catch { case e: RuntimeException => e.toString })", "String", ""),
    Shape("try-wildcard", "", "(try ps catch { case _: Throwable => \"x\" })", "String", ""),
    Shape("try-unit", "", "(try other.mu() catch { case _: RuntimeException => () })", "Unit", ""),
    Shape("try-cases-finally", "", "(try pi catch { case _: RuntimeException => 1 } finally other.mu())", "Int", ""),
    Shape("cond-block", "", "({ other.mu(); pi < 3 })", "Boolean", ""), Shape("cond-block-test", "", "({ other.mu(); pa.isInstanceOf[String] })", "Boolean", ""),
    Shape("cond-block-not-test", "", "({ other.mu(); !pa.isInstanceOf[String] })", "Boolean", ""),
    Shape("cond-block-constant", "", "({ other.mu(); true })", "Boolean", ""), Shape("cond-block-null", "", "({ other.mu(); pa == null })", "Boolean", ""),
    Shape("cond-block-nested", "", "({ other.mu(); { other.mu(); pi > 1 && pb } })", "Boolean", ""),
    // A constant reduced inside a block's result, and a block under a reduced connective.
    Shape("cond-block-constant-and", "", "({ other.mu(); true && pb })", "Boolean", ""),
    Shape("cond-block-constant-not", "", "({ other.mu(); !false })", "Boolean", ""),
    Shape("cond-block-constant-or", "", "({ other.mu(); false || pi > 1 })", "Boolean", ""),
    Shape("cond-constant-and-block", "", "(true && { other.mu(); pb })", "Boolean", ""),
    Shape("if-block-constant", "", "(if ({ other.mu(); true }) pi else 2)", "Int", ""),
    Shape("cond-null", "", "(pa eq null)", "Boolean", ""), Shape("cond-null-left", "", "(null ne other)", "Boolean", ""),
    Shape("to-string-int", "", "pi.toString", "String", ""), Shape("to-string-long", "", "pl.toString", "String", ""),
    Shape("to-string-double", "", "pd.toString", "String", ""), Shape("to-string-float", "", "pf.toString", "String", ""),
    Shape("to-string-bool", "", "pb.toString", "String", ""), Shape("to-string-char", "", "pc.toString", "String", ""),
    Shape("to-string-byte", "", "pby.toString", "String", ""), Shape("to-string-short", "", "psh.toString", "String", ""),
    Shape("to-string-string", "", "ps.toString", "String", ""), Shape("to-string-generic", "", "pt.toString", "String", ""),
    Shape("to-string-value-class", "", "pv.toString", "String", ""), Shape("to-string-box", "", "pbox.toString", "String", ""),
    Shape("to-string-class", "", "other.toString", "String", ""), Shape("to-string-trait", "", "tr.toString", "String", ""),
    Shape("to-string-function", "", "pfn.toString", "String", ""), Shape("to-string-unit", "", "other.mu().toString", "String", ""),
    Shape("to-string-this", "", "this.toString", "String", "members"), Shape("to-string-object", "", "O.toString", "String", ""),
    Shape("to-string-literal", "", "1.toString", "String", ""), Shape("to-string-literal-string", "", "\"s\".toString", "String", ""),
    Shape("to-string-sum", "", "(pi + 1).toString", "String", ""), Shape("to-string-cast", "", "pa.asInstanceOf[String].toString", "String", ""),
    Shape("to-string-call", "", "other.m0().toString", "String", ""), Shape("to-string-generic-int", "", "pbox.item.toString", "String", ""),
    Shape("to-string-generic-string", "", "psb.item.toString", "String", ""), Shape("to-string-java", "", "ps.length.toString", "String", ""),
    Shape("to-string-new-value-class", "", "new V(pi).toString", "String", ""), Shape("to-string-array", "", "java.util.Arrays.asList(ps).toString", "String", "method"),
    Shape("concat-two", "", "(ps + pi)", "String", ""), Shape("concat-constant-first", "", "(\"a\" + pi)", "String", ""),
    Shape("concat-constant-last", "", "(ps + \"a\")", "String", ""), Shape("concat-three", "", "(ps + pi + \"z\")", "String", ""),
    Shape("concat-int-first", "", "(pi + \"a\")", "String", ""), Shape("concat-char-first", "", "(pc + ps)", "String", ""),
    Shape("concat-primitives", "", "(\"p\" + pi + pl + pd + pf + pb + pc + pby + psh)", "String", ""),
    Shape("concat-references", "", "(\"r\" + pa + null + pv + pt + pbox + other + ())", "String", "method"),
    Shape("concat-unit", "", "(ps + other.mu() + ())", "String", ""), Shape("concat-null", "", "((null: String) + ps + null)", "String", ""),
    Shape("concat-value-class", "", "(\"v\" + pv + new V(pi))", "String", ""),
    Shape("concat-value-class-string", "val vs = new VS(ps)", "(\"v\" + vs + ps)", "String", ""),
    Shape("concat-constants", "", "(\"a\" + \"b\")", "String", ""), Shape("concat-empty", "", "(\"\" + pi)", "String", ""),
    Shape("concat-empty-only", "", "(\"\" + \"\")", "String", ""),
    Shape("concat-folded", "", "(\"a\" + 1 + 2L + true + 'c' + pi + 3 + \"z\")", "String", ""),
    Shape("concat-folded-int", "", "(1 + \"a\" + pi)", "String", ""), Shape("concat-folded-double", "", "(\"a\" + 1.5 + 2 + pi)", "String", ""),
    Shape("concat-folded-narrow", "", "(\"a\" + 1.toByte + 2.toLong + pi)", "String", ""),
    Shape("concat-one-constant", "", "(1 + ps)", "String", ""),
    Shape("concat-nested", "", "(ps + (ps + pi))", "String", ""), Shape("concat-nested-head", "", "((ps + pi) + ps + (\"b\" + pl))", "String", ""),
    Shape("concat-block-head", "", "({ val q = pi + 1; \"a\" + q } + ps)", "String", ""),
    Shape("concat-block-head-twice", "", "({ val q = pi + 1; { val r = q + 1; \"a\" + q + r } + pl } + ps)", "String", ""),
    Shape("concat-block-head-statements", "", "({ other.mu(); def bq(x: Int): String = \"d\" + x; var bw: Int = pi; bw = bw + 1; bq(bw) + \"a\" } + ps)", "String", ""),
    Shape("concat-block-head-cell", "", "({ var cq: Int = pi; val bump = () => { cq = cq + 1 }; bump(); \"a\" + cq } + ps)", "String", ""),
    Shape("concat-block-head-pattern", "", "({ val (qa, qb) = (pi, ps); \"a\" + qa + qb } + ps)", "String", ""),
    Shape("concat-block-head-lazy", "", "({ lazy val lq: String = ps + \"z\"; \"a\" + lq } + ps)", "String", ""),
    Shape("concat-block-head-mixed", "", "({ val q = pi + 1; { lazy val lr: Int = q + 1; \"a\" + q + lr } + pl } + ps)", "String", ""),
    Shape("concat-block-operand", "", "(ps + { val q = pi + 1; q })", "String", ""),
    Shape("concat-if-head", "", "((if (pb) \"a\" else \"b\") + ps)", "String", ""),
    Shape("concat-marks", "", "(\"a\\u0001b\" + pi + \"\\u0002\" + ps)", "String", ""),
    Shape("concat-to-string", "", "(pi.toString + ps + pa.toString)", "String", ""),
    Shape("concat-own", "", "(fs + fi + fv)", "String", "members"), Shape("concat-call", "", "(other.ms(ps, pi) + other.m0())", "String", ""),
    Shape("concat-many", "", "(ps + pi + \"a\" + pl + 'c' + pd + true + pa + 1 + pv + \"z\" + pt + pf + pb)", "String", "method"),
    Shape("interpolation", "", "s\"a${pi}b${ps}c\"", "String", ""), Shape("interpolation-one", "", "s\"$ps\"", "String", ""),
    Shape("interpolation-one-int", "", "s\"$pi\"", "String", ""), Shape("interpolation-none", "", "s\"only\"", "String", ""),
    Shape("interpolation-empty", "", "s\"\"", "String", ""), Shape("interpolation-adjacent", "", "s\"$pl$pd$pc\"", "String", ""),
    Shape("interpolation-references", "", "s\"${pv}-${pa}-${pt}-${()}-${null}\"", "String", "method"),
    Shape("interpolation-constants", "", "s\"${1}${\"a\"}${true}$pi\"", "String", ""),
    Shape("interpolation-raw", "", "raw\"a\\n$pi\"", "String", ""),
    Shape("interpolator-s", "", "StringContext(\"a\", \"b\").s(pi)", "String", ""),
    Shape("interpolator-raw", "", "StringContext(ps, \"\\n\").raw(pd)", "String", ""),
    Shape("interpolation-expression", "", "s\"${pi + 1}${ps.length}${other.m0()}\"", "String", ""),
    Shape("concat-past-slots", "", Vector.fill(210)("pi").mkString("(\"\" + ", " + ", ")"), "String", "method"),
    Shape("concat-past-slots-wide", "", Vector.fill(70)("pl + \"-\" + pd").mkString("(ps + ", " + ", ")"), "String", "method"),
    Shape("concat-past-slots-twice", "", Vector.fill(450)("pc").mkString("(ps + ", " + ", ")"), "String", "method"),
    Shape("concat-past-recipe", "", "(\"" + "a" * 40000 + "\" + pi + \"" + "b" * 40000 + "\" + ps)", "String", "method"),
    // The members the back end synthesizes for case classes and value classes, called: `equals` over every kind of
    // field and through an enclosing instance, `hashCode` by mixing and through the elements, `canEqual`, and a
    // value class's over a primitive, a wide and a reference underlying value.
    Shape("synth-equals-prims", "", "(Prims(pl, pd, pf, pi, pb, pc, ps, pv) == Prims(pl, pd, pf, pi, pb, pc, ps, pv))", "Boolean", ""),
    Shape("synth-hash-prims", "", "Prims(pl, pd, pf, pi, pb, pc, ps, pv).hashCode", "Int", ""),
    Shape("synth-equals-open", "", "Open(pi, ps).equals(pa)", "Boolean", ""),
    Shape("synth-can-equal", "", "Open(pi, ps).canEqual(pa)", "Boolean", ""),
    Shape("synth-hash-refs", "", "Refs(ps, List(pi)).hashCode", "Int", ""),
    Shape("synth-equals-refs", "", "(Refs(ps, List(pi)) == Refs(ps, List(pi)))", "Boolean", ""),
    Shape("synth-equals-outer", "", "{ val h = new Holder; h.made == h.In(pi) }", "Boolean", ""),
    Shape("synth-vc-equals-double", "", "(new VD(pd) == new VD(pd))", "Boolean", ""),
    Shape("synth-vc-hash-long", "", "new VJ(pl).hashCode", "Int", ""),
    Shape("synth-vc-equals-long", "", "new VJ(pl).equals(pa)", "Boolean", ""),
    Shape("synth-vc-case-string", "", "CV(pi).toString", "String", ""),
    Shape("synth-vc-case-can-equal", "", "CV(pi).canEqual(pa)", "Boolean", ""),
    Shape("synth-vc-case-arity", "", "CV(pi).productArity", "Int", ""),
    Shape("synth-vc-case-prefix", "", "CV(pi).productPrefix", "String", ""),
    Shape("synth-vc-case-equals", "", "(CV(pi) == CV(pi + 1))", "Boolean", ""),
    Shape("synth-vc-reference-equals", "", "(CVS(ps) == CVS(ps))", "Boolean", ""),
    Shape("synth-vc-reference-hash", "", "CVS(ps).hashCode", "Int", ""),
    // An enum companion's lookups: by name over values its fields hold, by ordinal through a chain where a case
    // takes parameters, and over an instance's values.
    Shape("enum-value-of", "", "Paint.valueOf(\"Green\")", "Paint", ""),
    Shape("enum-from-ordinal", "", "Paint.fromOrdinal(pi)", "Paint", ""),
    Shape("enum-values", "", "Paint.values.length", "Int", ""),
    Shape("enum-from-ordinal-chain", "", "Form.fromOrdinal(pi - 1)", "Form", ""),
    Shape("enum-instance-values", "", "{ val p = new Palette; p.Tone.values.length }", "Int", ""),
    Shape("enum-instance-from-ordinal", "", "{ val p = new Palette; p.Tone.fromOrdinal(pi).ordinal }", "Int", ""),
    Shape("enum-instance-value-of", "", "{ val p = new Palette; p.Tone.valueOf(\"Light\").ordinal }", "Int", ""),
    Shape("enum-own-companion", "", "(Suit.count + Suit.first.ordinal + Suit.valueOf(\"Spades\").ordinal)", "Int", ""),
    Shape("enum-lazy-companion", "", "(Coin.face + Coin.Tails.ordinal)", "Int", ""),
    Shape("lambda-none", "", "((x: Int) => x + 1)", "Int => Int", ""),
    Shape("lambda-param", "", "((x: Int) => x + pi)", "Int => Int", ""),
    Shape("lambda-this", "", "((x: Int) => x + fi)", "Int => Int", "members"),
    Shape("lambda-this-call", "", "(() => m0())", "() => Int", "members"),
    Shape("lambda-val", "val lv: Int = pi + 1", "((x: Int) => x + lv)", "Int => Int", ""),
    Shape("lambda-var", "var lc: Int = pi", "((x: Int) => { lc = lc + x; lc })", "Int => Int", ""),
    Shape("lambda-lazy", "lazy val ll: Int = pi + 2", "((x: Int) => x + ll)", "Int => Int", ""),
    Shape("lambda-by-name", "def byn(x: => Int): Int => Int = (y: Int) => x + y", "byn(pi + 1)", "Int => Int", ""),
    Shape("lambda-generic", "", "(() => pt)", "() => T", ""),
    Shape("lambda-value-class", "", "(() => pv)", "() => V", ""),
    Shape("lambda-value-class-parameter", "", "((v: V) => v.twice)", "V => V", ""),
    Shape("lambda-nested", "", "((x: Int) => (y: Int) => x + y + pi)", "Int => Int => Int", ""),
    Shape("lambda-two-parameters", "", "((x: Int, y: String) => y + x + ps)", "(Int, String) => String", ""),
    Shape("lambda-arguments-around-match", "def three(f: Int => Int, m: Int, g: Int => Int): Int = f(m) + g(m)", "three((x: Int) => x + pi, pi match { case 0 => 1; case _ => 2 }, (y: Int) => y - pi)", "Int", ""),
    Shape("lambda-tuple-around-match", "", "((x: Int) => x + pi, pi match { case 0 => 1; case _ => 2 }, (y: Int) => y - pi)", "(Int => Int, Int, Int => Int)", ""),
    Shape("lambda-return", "def find(xs: Seq[Int]): Int = { xs.foreach(x => if x > pi then return x); 0 }", "find(Seq(1, 2, 3))", "Int", ""),
    Shape("lambda-return-nested", "def find2(xs: Seq[Int]): Int = { xs.foreach(x => xs.foreach(y => if x + y > pi then return y)); 0 }", "find2(Seq(1, 2, 3))", "Int", ""),
    Shape("lambda-return-this", "def findOwn(xs: Seq[Int]): Int = { xs.foreach(x => if x > fi then return x); 0 }", "findOwn(Seq(1, 2, 3))", "Int", "members"),
    Shape("lambda-lazy-key", "def keyed(): Int = { lazy val lk: Int = { if (pb) return 7; pi }; val f = () => () => lk; f()() }", "keyed()", "Int", ""),
    Shape("try-finally-lambda", "", "(try pi finally Takes.any(() => ps))", "Int", ""),
    Shape("try-catch-finally-lambda", "", "(try pi catch { case _: RuntimeException => 0 } finally Takes.any(() => ps))", "Int", ""),
    Shape("value-lambda", "", "((x: Int) => x + u)", "Int => Int", "value"),
    Shape("value-lambda-call", "", "(() => vm())", "() => Int", "value"),
    Shape("local-def-recursive", "def sum(n: Int): Int = if n <= 0 then pi else n + sum(n - 1)", "sum(3)", "Int", ""),
    Shape("local-defs-mutual", "def ev(n: Int): Boolean = if n == 0 then pb else od(n - 1); def od(n: Int): Boolean = if n == 0 then ps.isEmpty else ev(n - 1)", "ev(pi)", "Boolean", ""),
    Shape("local-def-forward", "", "{ Takes.any(later(pi)); def later(n: Int): Int = n + pl.toInt; later(1) }", "Int", ""),
    Shape("local-def-default", "def dfl(a: Int, b: Int = pi + 1): Int = a + b", "dfl(2)", "Int", ""),
    Shape("local-def-default-clause", "def dfc(a: Int)(b: Int = a + pi): Int = a + b", "dfc(3)()", "Int", ""),
    Shape("local-def-value", "def inc(x: Int): Int = x + pi", "(inc: Int => Int)", "Int => Int", ""),
    Shape("local-def-in-lambda", "def twice(x: Int): Int = x * 2 + pi", "((x: Int) => twice(x))", "Int => Int", ""),
    Shape("local-def-this", "def own(x: Int): Int = x + fi", "own(pi)", "Int", "members"),
    Shape("local-def-var", "var acc: Int = pi; def add(x: Int): Unit = { acc = acc + x }; add(2)", "acc", "Int", ""),
    Shape("local-def-unit", "def note(x: Int): Unit = Takes.any(x + pi)", "note(1)", "Unit", ""),
    Shape("local-def-generic", "def same[A](a: A): A = { Takes.any(pi); a }", "same(ps)", "String", ""),
    Shape("local-def-nested", "def outerDef(x: Int): Int = { def innerDef(y: Int): Int = y + x + pi; innerDef(x) }", "outerDef(2)", "Int", ""),
    Shape("local-def-outer-instance", "def od(x: Int): Int = x + fi; class Lh { val got: Int = od(1); def again: Int = od(2) }", "new Lh().again", "Int", "members"),
    Shape("local-defs-after-match", "", "{ pi match { case 0 => Takes.any(gg(1)); case _ => () }; def ff(i: Int): Int = if i <= 0 then 0 else gg(i - 1) + pi; def gg(i: Int): Int = if i <= 0 then 1 else ff(i - 1) + ps.length; ff(2) }", "Int", ""),
    Shape("local-defs-before-match", "", "{ def ff(i: Int): Int = if i <= 0 then 0 else gg(i - 1) + pi; def gg(i: Int): Int = if i <= 0 then 1 else ff(i - 1) + ps.length; pi match { case 0 => Takes.any(gg(1)); case _ => () }; ff(2) }", "Int", ""),
    Shape("local-defs-before-lambda", "", "{ def ff(i: Int): Int = if i <= 0 then 0 else gg(i - 1) + pi; def gg(i: Int): Int = if i <= 0 then 1 else ff(i - 1) + ps.length; val h = () => gg(1); ff(2) + h() }", "Int", ""),
    Shape("value-local-def", "def more(x: Int): Int = x + u", "more(2)", "Int", "value"),
    Shape("sam-runnable", "", "{ val r: Runnable = () => other.mu(); r.run(); pi }", "Int", "method"),
    Shape("sam-java-function", "", "{ val jf: java.util.function.Function[Int, Int] = (x: Int) => x + pi; jf.apply(2) }", "Int", "method"),
    Shape("sam-trait", "trait Sam1 { def run1(x: Int): Int }", "{ val s: Sam1 = (x: Int) => x + pi; s.run1(2) }", "Int", "method"),
    Shape("by-name-this", "", "{ Takes.lazily(fi + 1); fi }", "Int", "members"),
    Shape("by-name-var", "var bn: Int = pi", "{ Takes.lazily({ bn = bn + 1; bn }); bn }", "Int", ""),
    Shape("by-name-lambda", "", "{ Takes.lazily((x: Int) => x + pi); pi }", "Int", ""),
    Shape("lazy-local-in-local-class", "lazy val lo: Int = fi + 1; class Lo { def get(): Int = lo }", "new Lo().get()", "Int", "members"),
    Shape("template-virtual", "", "ps.codePointAt(0)", "Int", ""), Shape("template-virtual-two", "", "ps.substring(0, 1)", "String", ""),
    Shape("template-virtual-bool", "", "ps.startsWith(\"s\")", "Boolean", ""),
    Shape("template-virtual-unit", "", "new java.lang.StringBuilder(ps).setLength(pi)", "Unit", ""),
    Shape("template-virtual-long", "", "java.nio.ByteBuffer.allocate(16).getLong(0)", "Long", ""),
    Shape("template-virtual-inherited", "", "new RuntimeException(ps).getMessage", "String", ""),
    Shape("template-virtual-chain", "", "new java.lang.StringBuilder().append(ps).append(pi).toString", "String", ""),
    Shape("template-virtual-array", "", "new java.lang.StringBuilder(ps).getChars(0, 1, Array('a', pc), 0)", "Unit", ""),
    Shape("template-virtual-own", "", "tconcat(ps, \"x\")", "String", ""),
    Shape("template-interface", "", "(ps: CharSequence).length", "Int", ""),
    Shape("template-interface-two", "", "(ps: CharSequence).subSequence(0, 1)", "CharSequence", ""),
    Shape("template-static", "", "java.lang.Math.abs(pd)", "Double", ""), Shape("template-static-two", "", "java.lang.Integer.toString(pi, 16)", "String", ""),
    Shape("template-static-none", "", "java.lang.System.nanoTime()", "Long", ""),
    Shape("template-static-array", "", "java.lang.String.valueOf(Array('a', pc))", "String", ""),
    Shape("template-static-unit", "", "java.lang.System.exit(pi)", "Unit", ""),
    Shape("template-static-value-class", "", "java.lang.String.valueOf(pv)", "String", ""),
    Shape("template-static-generic", "", "java.lang.String.valueOf(pt)", "String", ""), Shape("template-static-own", "", "tabs(pd)", "Double", ""),
    Shape("template-tree", "", "ps.indexOf(\"x\")", "Int", ""), Shape("template-tree-three", "", "ps.replace(\"a\", \"b\")", "String", ""),
    // The template rows of the host "method": in a constructor the product mode refuses them today, the reference's
    // binary as the head's ("cannot write TASTy for a reference to the type parameter A outside its scope").
    Shape("template-tree-receiver", "", "ps.getBytes", "Array[Byte]", "method"), Shape("template-tree-own-type", "", "ps.split(\"s\")", "Array[String]", ""),
    Shape("template-tree-interface", "", "Some(pi).productArity", "Int", ""), Shape("template-tree-interface-two", "", "Some(ps).productElement(0)", "Any", ""),
    Shape("template-tree-static", "", "java.util.Base64.getEncoder()", "java.util.Base64.Encoder", ""),
    Shape("template-tree-value-class", "", "tshow(pv)", "String", ""), Shape("template-tree-unit", "", "tshow(())", "String", ""),
    Shape("template-tree-generic", "", "tshow(pt)", "String", ""),
    Shape("template-field", "", "java.lang.System.out", "java.io.PrintStream", ""), Shape("template-field-class", "", "java.lang.Integer.TYPE", "Class[?]", ""),
    Shape("template-field-operand", "", "ps.toUpperCase(java.util.Locale.ROOT)", "String", ""), Shape("template-field-receiver", "", "pa.##", "Int", "method"),
    Shape("template-runtime", "", "classOf[String].getDeclaredMethod(\"length\")", "java.lang.reflect.Method", "method"),
    Shape("template-new", "", "new java.lang.Integer(pi)", "java.lang.Integer", "method"), Shape("template-new-own", "", "tnew(ps)", "java.lang.StringBuilder", ""),
    Shape("template-recorded", "", "tget(new java.util.ArrayList[String](), 0)", "String", ""),
    Shape("template-recorded-int", "", "tget[Int](new java.util.ArrayList[Int](), 0)", "Int", ""),
    Shape("template-drop", "", "tdrop(new java.lang.StringBuilder(), ps)", "Unit", ""),
    Shape("template-throw", "", "(if pb then pi else tthrow(ps))", "Int", ""), Shape("template-throw-statement", "", "{ if pi > 100 then tthrow(ps); ps }", "String", ""),
    Shape("template-arithmetic", "", "tadd(pi, 2)", "Int", ""), Shape("template-arithmetic-long", "", "txor(pl, 3L)", "Long", ""),
    Shape("template-arithmetic-double", "", "tmul(pd, 2.0)", "Double", ""), Shape("template-arithmetic-constants", "", "tshift(pi)", "Int", ""),
    Shape("template-conversion", "", "twide(pi)", "Long", ""), Shape("template-conversion-two", "", "tnarrow(pd)", "Char", ""),
    Shape("template-lines", "", "ps.substring(\n      0, 1)", "String", ""), Shape("template-tree-lines", "", "ps.indexOf(\n      \"x\")", "Int", ""),
    Shape("template-cast", "", "(ps: Comparable[String]).compareTo(\"b\")", "Int", ""),
    Shape("template-cast-root", "", "java.util.List.of(ps, \"b\")", "java.util.List[String]", "method"),
    Shape("template-cast-receiver", "", "java.util.Base64.getEncoder().encodeToString(Array[Byte](1, 2))", "String", ""),
    Shape("template-cast-new", "", "new String(Array[Byte](65, 66))", "String", "method"),
    Shape("template-erased", "", "unsafeCast[Any, String](pa)", "String", "method"), Shape("template-erased-boxed", "", "unsafeCast[Int, Any](pi)", "Any", "method"),
    Shape("template-erased-lines", "", "unsafeCast[Any, String](\n      pa)", "String", "method"),
    Shape("template-array-load", "val xs = Array(pi, 2)", "xs(1)", "Int", ""),
    Shape("template-array-load-string", "val xs = Array(ps, \"b\")", "xs(0)", "String", ""),
    Shape("template-array-load-generic", "val xs = Array[Any](pt, pt).asInstanceOf[Array[T]]", "xs(0)", "T", ""),
    Shape("template-array-store", "val xs = Array(pl, 2L)", "{ xs(0) = pl + 1; xs(0) }", "Long", ""),
    Shape("template-array-store-boxed", "val xs = new Array[Any](2)", "{ xs(1) = pi; xs }", "Array[Any]", ""),
    Shape("template-array-store-generic", "val xs = Array[Any](pt, pt).asInstanceOf[Array[T]]", "{ xs(0) = pt; xs(0) }", "T", ""),
    Shape("template-array-length", "val xs = Array(pd, 2.0)", "xs.length", "Int", ""),
    Shape("template-array-length-generic", "val xs = Array[Any](pt, pt).asInstanceOf[Array[T]]", "xs.length", "Int", ""),
    Shape("template-array-clone", "", "Array(pc, 'd').clone()", "Array[Char]", ""),
    Shape("template-array-clone-generic", "val xs = Array[Any](pt, pt).asInstanceOf[Array[T]]", "xs.clone()", "Array[T]", ""),
    Shape("template-array-new", "", "new Array[Int](pi)", "Array[Int]", "method"), Shape("template-array-new-string", "", "new Array[String](2)", "Array[String]", "method"),
    Shape("template-array-new-nested", "", "new Array[Array[Byte]](pi)", "Array[Array[Byte]]", "method"),
    Shape("typer-hash", "", "pa.hashCode", "Int", ""), Shape("typer-hash-class", "", "other.hashCode", "Int", ""),
    Shape("typer-hash-generic", "", "pt.hashCode", "Int", ""), Shape("typer-hash-value-class", "", "pv.hashCode", "Int", ""),
    Shape("typer-equals", "", "pa.equals(ps)", "Boolean", ""), Shape("typer-equals-int", "", "pi.equals(pa)", "Boolean", ""),
    Shape("typer-get-class", "", "pa.getClass", "Class[?]", ""), Shape("typer-get-class-own", "", "other.getClass", "Class[?]", ""),
    Shape("typer-get-class-int", "", "pi.getClass", "Class[?]", ""), Shape("typer-get-class-value-class", "", "pv.getClass", "Class[?]", ""),
    Shape("typer-identity-hash", "", "super.hashCode", "Int", "members"),
    Shape("typer-ordinal", "", "Hue.fromOrdinal(pi % 2).ordinal", "Int", ""),
    Shape("typer-mirror-ordinal", "", "summon[scala.deriving.Mirror.SumOf[Hue]].ordinal(Hue.Green)", "Int", ""),
    Shape("typer-mirror-ordinal-nested", "val tones = new Tones", "summon[scala.deriving.Mirror.SumOf[tones.Tone]].ordinal(tones.Tone.High)", "Int", ""),
    Shape("typer-enum-values", "", "Hue.values", "Array[Hue]", ""), Shape("typer-enum-value-of", "", "Hue.valueOf(\"Red\")", "Hue", ""),
    Shape("typer-enum-from-ordinal", "", "Hue.fromOrdinal(0)", "Hue", ""),
    Shape("typer-partial-function", "", "({ case 1 => \"one\" }: PartialFunction[Int, String])", "PartialFunction[Int, String]", ""),
    // A value class's box over its operand and its opening after a null test (`Gen::lower_value_class`): over a
    // call, a cast, `null`, a stable path (read twice), a binder, a `var`; a `String` and a `Long` underlying;
    // a lazy local's box; a case value class's box (its members' are the `synth-vc-case` rows).
    Shape("vc-open-call", "", "Option(pv).get", "V", ""), Shape("vc-open-null", "", "null.asInstanceOf[V]", "V", ""),
    Shape("vc-open-cast-string", "", "pa.asInstanceOf[VS]", "VS", ""), Shape("vc-open-cast-long", "", "pa.asInstanceOf[VL]", "VL", ""),
    Shape("vc-open-call-long", "", "Option(new VL(pl)).get", "VL", ""), Shape("vc-open-stable", "", "HoldsV.bv.item", "V", ""),
    Shape("vc-open-bind-some", "", "(Option(pv) match { case Some(v) => v; case None => new V(0) })", "V", ""),
    Shape("vc-open-bind-pair", "", "((pv, pi) match { case (v, n) => v })", "V", ""),
    Shape("vc-open-var", "var wv: V = Option(pv).get; wv = Option(new V(pi)).get", "wv", "V", ""),
    Shape("vc-box-lazy", "lazy val lzv: V = new V(pi + 1)", "lzv", "V", ""),
    Shape("vc-box-lambda-result", "", "((n: Int) => new V(n))", "Int => V", ""),
    Shape("vc-case-new", "", "new CV(pi)", "CV", "")) ++ jumpFamily

  // The def's own jump under each branch form, for the writer's decision whether code follows a branch (where a
  // conditional writes its `goto` and binds its join): a def `jf` whose body is a `return`'s value or its result,
  // at `Int` and at `Unit` (where a case's jump is a statement of its block), holding a branch form (an `if`'s
  // `then` or `else`, a match's case, a `try`'s body, a `while`'s body, a block's result) with what stands there:
  // a value, the jump, a `return` of the jump before a value, a `return`, a `throw`, a match or an `if` one of whose
  // branches jumps, and one all of whose branches jump. The def loops where it calls itself in those places.
  def jumpFamily: Vector[Shape] =
    val places = Vector(
      "then" -> "(if c then X else D)", "else" -> "(if c then D else X)", "case" -> "(n % 3 match { case 0 => D; case _ => X })",
      "try" -> "(try X catch { case _: IllegalStateException => D })", "while" -> "{ var i = 0; while i < 1 do { i += 1; X }; D }",
      "block" -> "{ val q = n + 1; Takes.any(q); X }")
    val jump = "jf(n - 1, c)"
    def whats(tpe: String): Vector[(String, String)] =
      val v = if tpe == "Int" then "{ Takes.any(n); n * 10 + 1 }" else "Takes.any(n)"
      Vector("value" -> v, "jump" -> jump, "return-jump" -> s"{ if n > 2 then return jf(n - 2, c); $v }",
        "return" -> (if tpe == "Int" then "{ Takes.any(n); return n + 100 }" else "{ Takes.any(n); return }"),
        "throw" -> "throw new RuntimeException(\"t\")", "match-one" -> s"(n % 2 match { case 0 => $jump; case _ => $v })",
        "if-one" -> s"(if n % 2 == 0 then $jump else $v)", "match-all" -> s"(n % 2 match { case 0 => $jump; case _ => jf(n - 2, c) })",
        "if-all" -> s"(if n % 2 == 0 then $jump else jf(n - 2, c))")
    for
      tpe <- Vector("Int", "Unit")
      (place, form) <- places
      (what, x) <- whats(tpe)
      (position, body) <- Vector("return" -> "return E", "result" -> "E")
    yield
      val (d, z) = if tpe == "Int" then ("{ Takes.any(-n); 7 }", "0") else ("Takes.any(-n)", "()")
      val e = form.replace("X", x).replace("D", d)
      val definition = s"def jf(n: Int, c: Boolean): $tpe = { if n < 0 then return $z; ${body.replace("E", e)} }"
      Shape(s"jump-$place-$what-$position-${tpe.toLowerCase}", definition, "jf(pi, pb)", tpe, "members")

  // The positions that take the value at a type wanted.
  val positions: Vector[Position] = Vector(
    Position("statement", "def $n[T]($P): Unit = { $S; $E; () }"),
    Position("val", "def $n[T]($P): Unit = { $S; val w: $W = $E; Takes.any(w) }"),
    Position("argument", "def $n[T]($P): Unit = { $S; def take(w: $W): Unit = (); take($E) }"),
    Position("result", "def $n[T]($P): $W = { $S; $E }"),
    Position("return", "def $n[T]($P): $W = { $S; if (pb) return $E; $E }"),
    Position("branch", "def $n[T]($P): $W = { $S; if (pb) $E else $E }"),
    Position("lambda", "def $n[T]($P): () => $W = { $S; () => $E }"),
    Position("by-name", "def $n[T]($P): Unit = { $S; Takes.lazily[$W]($E) }"),
    Position("try-body", "def $n[T]($P): $W = { $S; try $E catch { case _: RuntimeException => $E } }"),
    Position("finalizer", "def $n[T]($P): Unit = { $S; try Takes.any(0) finally { val w: $W = $E; Takes.any(w) } }"),
    Position("case", "def $n[T]($P): $W = { $S; pi match { case 0 => $E; case _ => $E } }"),
    Position("assignment", "def $n[T]($P): Unit = { $S; var w: $W = $E; w = $E; Takes.any(w) }"),
    Position("element", "def $n[T]($P): Seq[$W] = { $S; Seq[$W]($E, $E) }"),
    Position("constructor-argument", "def $n[T]($P): Box[$W] = { $S; new Box[$W]($E) }"),
    Position("generic-argument", "def $n[T]($P): $W = { $S; Takes.id[$W]($E) }"))
  // The positions that take the value at its own type: every type but Unit, or the ones named.
  val ownPositions: Vector[Position] = Vector(
    Position("receiver", "def $n[T]($P): Int = { $S; ($E).hashCode }"),
    Position("concatenation", "def $n[T]($P): String = { $S; \"x\" + $E + s\"$${$E}\" }"),
    Position("equality", "def $n[T]($P): Boolean = { $S; $E == $E }"),
    Position("condition", "def $n[T]($P): Int = { $S; if ($E) 1 else 2 }", "Boolean"),
    Position("operand", "def $n[T]($P): Unit = { $S; Takes.any($E + 1) }", "numeric"))

  // One statement per way the lowering escapes (`--after-escape`), each a tree in a block that is lowered, so that
  // what follows it in the same method is lowered after typed IR was left: a `return` that is no result, a match the
  // lowering does not take (`case null`, a sequence's pattern, a test of `Null`, of `AnyRef` and of an enclosing
  // instance), a block with a definition by a pattern, `isInstanceOf`, a `try` whose case is `null`, a top-level
  // inline string read as a value, a given object selected on a receiver evaluated, a value class's method, and a
  // default on a receiver, of a local def and after arguments, and a concatenation whose first operand is such a
  // block, a local class's secondary constructor's default that takes what it captures, and in a value class's method
  // an anonymous class that captures a lazy val of the method. An `AnyVal` test and an inline Boolean leave at the
  // same place as an `AnyRef` test and an inline string.
  val escapes: Vector[Escape] = Vector(
    Escape("return", "if topv < 0 then return (throw new IllegalStateException(String.valueOf($X)))", expression = false),
    Escape("match-null", "Takes.any(Takes.id[Any]($X) match { case null => 0; case _ => $X })"),
    Escape("match-sequence", "Takes.any(Seq[Any]($X, 2) match { case Seq(a, _*) => a; case _ => $X })"),
    Escape("match-null-type", "Takes.any(Takes.id[Any]($X) match { case _: Null => 0; case _ => $X })"),
    Escape("match-anyref", "Takes.any(Takes.id[Any]($X) match { case _: AnyRef => 0; case _ => $X })"),
    Escape("match-outer", "Takes.any(Takes.id[Any]($X) match { case _: outer.Inner => 0; case _ => $X })"),
    Escape("pattern-definition", "{ val Some(esc1) = Option[Any]($X): @unchecked; Takes.any($X); Takes.any(esc1) }"),
    Escape("type-test", "Takes.any(Takes.id[Any]($X).isInstanceOf[String])"),
    Escape("catch-null", "Takes.any(try Takes.id[Any]($X) catch { case null => $X })"),
    Escape("inline-string", "Takes.any(topstr)", operand = false),
    Escape("given-receiver", "Takes.any(Takes.id[O.type]({ Takes.any($X); O }).og)"),
    Escape("value-class-call", "Takes.any(new V(Takes.id[Any]($X).hashCode).twice)"),
    Escape("default-receiver", "Takes.any(new Sub(Takes.id[Any]($X).hashCode).td())"),
    Escape("default-local-def", "{ def escd(x: Int = topv): Int = x; Takes.any(escd()) }", operand = false),
    Escape("default-after-argument", "Takes.any(new Dm(Takes.id[Any]($X).hashCode)())"),
    Escape("concat-statements", "Takes.any({ val (esc1: Int) = topv; \"c\" + esc1 } + String.valueOf($X))"),
    Escape("default-ctor-held", "{ val escp = \"p\"; class EscImpl(tag: String, n: Int) { def this(tag: String = \"s\") = this(tag, 1); " +
      "def show: String = tag + escp + n }; Takes.any(new EscImpl().show) }", operand = false),
    Escape("captured-new-value-class", "{ lazy val escx: Int = u + 1; Takes.any(new java.util.concurrent.Callable[Int] { def call(): Int = escx }.call()) }",
      operand = false, expression = false, host = "value"))

  def escapeText(escape: Escape, operand: String): String = escape.text.replace("$X", operand)

  // What only the programs after an escape hold, the run's control: a value class an object given by `given` holds,
  // whose creation after an escape the lowering leaves to the generator there (a compiler that wrote it with no node
  // there panicked). Every run after an escape takes them, whatever `--shapes` and `--family-sample` take
  // (`selfTest`).
  val escapePrelude = """trait GvBase
given GV: GvBase with {
  class Meters(val v: Int) extends AnyVal { def twice: Int = v * 2 }
}
"""
  val escapeShapes: Vector[Shape] = Vector(
    Shape("given-vc-new", "", "new GV.Meters(pi)", "GV.Meters", ""), Shape("given-vc-twice", "", "new GV.Meters(pi).twice", "Int", ""))

  // The positions of a shape after an escape that hold it in a form the lowering leaves to the generator there, which
  // lowers it where it writes it: the escape's own tree (`$X`), the creation of a local class that captures (through
  // its primary constructor, an anonymous class's and a secondary), a local trait's member's call, a local def's call
  // and its body, a local lazy val's read and its initialiser; and a lambda that holds the shape and reads a lazy val
  // or a def declared after it. At the shape's own type; "class" where a local class may stand (not a value class).
  val escapePositions: Vector[Position] = Vector(
    Position("in-escape", "def $n[T]($P): Unit = { $S; $X }"),
    Position("captured-new", "def $n[T]($P): Unit = { $S; class EscCap(val w: $W) { def g: Int = pi }; Takes.any(new EscCap($E).g) }", "class"),
    Position("anonymous-new", "def $n[T]($P): Unit = { $S; Takes.any(new Box[$W]($E) { def g: Int = pi }) }", "class"),
    Position("via-new", "def $n[T]($P): Unit = { $S; class EscVia(val w: $W, val k: Int) { def this(w: $W) = this(w, pi) }; Takes.any(new EscVia($E).k) }", "class"),
    Position("local-trait", "def $n[T]($P): Unit = { $S; trait EscLt { def lm(w: $W): Int = pi }; Takes.any((new EscLt {}).lm($E)) }", "class"),
    Position("local-def", "def $n[T]($P): $W = { $S; def escHeld(): $W = $E; escHeld() }"),
    Position("lazy-local", "def $n[T]($P): $W = { $S; lazy val escLazy: $W = $E; escLazy }"),
    Position("later-lazy", "def $n[T]($P): Unit = { $S; Takes.any(() => { Takes.any(escLater); Takes.id[$W]($E) }); lazy val escLater: Int = pi + 1; () }"),
    Position("later-def", "def $n[T]($P): Unit = { $S; Takes.any(() => { Takes.any(escLater); Takes.id[$W]($E) }); def escLater: Int = pi + 1; () }"))

  val hosts = Vector("class", "open", "object", "trait")
  // The shapes a value class's method holds beside its own.
  val inValueClass = Set("lit-int", "param-int", "param-string", "param-generic", "this")

  def wantsOf(shape: Shape, host: String): Vector[String] =
    val own = wants.find(_._1 == shape.tpe).map(_._2).getOrElse(
      if shape.tpe.startsWith("Class[") then Vector(shape.tpe, "Class[?]", "Any", "AnyRef")
      else if shape.tpe.startsWith("Array[") then Vector(shape.tpe, "Any", "AnyRef")
      else Vector(shape.tpe))
    val held = if host == "value" then own.filter(w => w != "AnyRef" && w != "Tr") else own
    if shape.expr == "1" then held ++ narrowed else held

  // The cases of a shape in a method of a host: (position, want, the method's text).
  def cases(shape: Shape, host: String, escape: Option[Escape] = None): Vector[(String, String, String)] =
    val all = wantsOf(shape, host)
    // A concatenation past what one call takes is long: at its own type, in two positions.
    val past = shape.name.startsWith("concat-past-")
    val out = Vector.newBuilder[(String, String, String)]
    for want <- all; position <- positions do
      // The statement position wants nothing: once. The JVM back end refuses a `try` as an argument of a
      // constructor, with its place. A narrowed literal's type is written by the literal alone.
      val left = position.name == "statement" && want != all.head ||
        shape.name.startsWith("try-") && (position.name == "element" || position.name == "constructor-argument") ||
        narrowed.contains(want) && shape.tpe == "Int" && position.name == "generic-argument" ||
        past && (want != all.head || position.name != "val" && position.name != "result")
      if !left then out += ((position.name, want, position.text))
    for position <- ownPositions do
      val fits = shape.tpe != "Unit" && !past && (position.fits match
        case "Boolean" => shape.tpe == "Boolean"
        case "numeric" => numeric(shape.tpe)
        case _ => true)
      if fits then out += ((position.name, shape.tpe, position.text))
    // After an escape, the forms that hold the shape; not a `try` where a constructor's or a sequence's argument holds it.
    for escape <- escape.toVector; position <- escapePositions do
      val held = if position.name == "in-escape" then escape.operand && !(shape.name.startsWith("try-") && constructed(escape.name))
        else !(shape.name.startsWith("try-") && newPositions(position.name))
      if held && !past && (position.fits != "class" || host != "value") then out += ((position.name, all.head, position.text))
    out.result()

  // The escapes whose operand a constructor's or a sequence's argument holds, and the positions a constructor's
  // argument holds the shape in: the JVM back end refuses a `try` there.
  val constructed = Set("return", "match-sequence", "value-class-call", "default-receiver", "default-after-argument")
  val newPositions = Set("captured-new", "anonymous-new", "via-new")

  def method(text: String, n: String, shape: Shape, want: String, escape: Option[Escape] = None): String =
    val before = if shape.before.isEmpty then "()" else shape.before
    val written = text.replace("$$", "\u0000").replace("$n", n).replace("$P", params).replace("$S", escape.fold(before)(e => escapeText(e, "topv") + "; " + before))
      .replace("$E", shape.expr).replace("$W", want)
    escape.fold(written)(e => written.replace("$X", escapeText(e, s"(${shape.expr}: $want)"))).replace("\u0000", "$")

  // The expression after the escape, in a constructor or a field's initialiser, where a statement cannot stand first.
  def escaped(expr: String, escape: Option[Escape]): String =
    escape.filter(_.expression).fold(expr)(e => s"{ ${escapeText(e, "topv")}; $expr }")

  // Whether the expression names none of a method's parameters, so that a field's initialiser may hold it.
  def closed(shape: Shape): Boolean =
    shape.before.isEmpty && !"\\b(p(i|l|d|f|b|c|by|sh|s|a|t|v|box|fn|f0|f2)|psb|other|sub|tr)\\b".r.findFirstIn(shape.expr).isDefined

  // A program and the number of its cases.
  def program(host: String, shape: Shape, escape: Option[Escape] = None): (String, Int) =
    if host == "constructor" then constructors(shape, escape)
    else
      val made = cases(shape, host, escape).zipWithIndex.map { case ((_, want, text), i) => method(text, s"g$i", shape, want, escape) }
      val calls = made.indices.map(i => s"    Takes.any(h.g$i[String]($arguments))\n").mkString
      // A field's initialiser, where the expression stands in the class's body.
      val fields = if host != "value" && shape.hosts != "" && shape.hosts != "method" && closed(shape) then wantsOf(shape, host).zipWithIndex.map { case (want, i) => s"  val init$i: $want = ${escaped(shape.expr, escape)}\n" } else Vector.empty
      val body = made.map("  " + _ + "\n").mkString + fields.mkString
      val (text, make, self) = host match
        case "class" => (s"class H(val fi: Int, val fs: String, pp: Int, pq: String) extends Tr {$members$body}\n", "new H(1, \"s\", 2, \"q\")", "H")
        case "open" =>
          (s"class H(val fi: Int, val fs: String, pp: Int, pq: String) extends Tr {$members$body}\nclass HSub extends H(1, \"s\", 2, \"q\") {\n  override def m0(): Int = 5\n}\n",
            "new H(1, \"s\", 2, \"q\")", "H")
        case "object" => (s"object H extends Tr {\n  val fi: Int = 1\n  val fs: String = \"s\"$members$body}\n", "H", "H.type")
        case "trait" => (s"trait H extends Tr {\n  val fi: Int = 1\n  val fs: String = \"s\"$members$body}\nclass HImpl extends H\n", "new HImpl", "H")
        case _ => (s"final class H(val u: Int) extends AnyVal {\n  def vm(): Int = u + 1\n$body}\n", "new H(3)", "H")
      val main = s"object Main {\n  def main(args: Array[String]): Unit = {\n    val h = $make\n$calls  }\n}\n"
      ((prelude + escape.fold("")(_ => escapePrelude) + text + main).replace("THIS", self), made.length + fields.length)

  // The shapes the product mode does not pickle as a superclass's argument today: it refuses a reference to the
  // superclass's type parameter outside its scope.
  val unpickled = Set("call-java-array", "array-format", "array-packed-filter", "array-tuple-xxl", "array-of-references", "java-array-interface", "java-array-chars",
    "match-extractor-boolean", "enum-instance-values", "enum-instance-from-ordinal", "enum-instance-value-of")

  // A class per want whose constructor holds the expression: its superclass's argument, a trait's argument, a
  // val's and a var's initialisers, a statement, and the statement after a secondary constructor's call of the
  // primary. Not a superclass's or a trait's argument: a `try`, which the JVM back end refuses there, and the
  // shapes of `unpickled` and a string's order, which the product mode does not pickle there today.
  def constructors(shape: Shape, escape: Option[Escape] = None): (String, Int) =
    val all = wantsOf(shape, "constructor")
    val bare = shape.name.startsWith("try-") || unpickled(shape.name) || shape.name.startsWith("cmp-string-")
    val names = "\\b(\\w+): ".r.findAllMatchIn(params).map(_.group(1)).mkString(", ")
    val classes = all.zipWithIndex.map { case (want, i) =>
      val parent = if bare then "" else s" extends CBase[$want](${shape.expr}) with KTr[$want](${shape.expr})"
      val (init, statement) = (escaped(shape.expr, escape), escaped(s"Takes.any(${shape.expr})", escape))
      s"class K$i[T](val vx: Int, $params)$parent {\n  val init: $want = $init\n  var later: $want = $init\n  $statement\n" +
        s"  def this(vz: Boolean, $params) = { this(1, $names); $statement }\n  def held: Int = vx\n}\n"
    }
    val calls = all.indices.map(i => s"    Takes.any(new K$i[String](0, $arguments).held)\n    Takes.any(new K$i[String](true, $arguments).held)\n").mkString
    val main = s"object Main {\n  def main(args: Array[String]): Unit = {\n$calls  }\n}\n"
    (prelude + escape.fold("")(_ => escapePrelude) + "trait KTr[A](val kt: A)\n" + classes.mkString + main, all.length * (if bare then 4 else 6))

  def hostsOf(shape: Shape, escape: Option[Escape] = None): Vector[String] =
    val own = shape.hosts match
      case "" => if shape.before.isEmpty then Vector("class", "constructor") else Vector("class")
      case "class" => Vector("class", "open")
      case "method" => Vector("class")
      case "members" => hosts
      case other => Vector(other)
    val all = if inValueClass(shape.name) || shape.name.startsWith("jump-") || escapeShapes.contains(shape) then own :+ "value" else own
    // A constructor no statement can stand first in has no escape before its expression.
    all.filter(host => (host != "constructor" || escape.forall(_.expression)) && escape.forall(e => e.host.isEmpty || e.host == host))

  // `--family-sample <n>`: n shapes of each family (the name up to its first dash), the n after the escape's own
  // place among the escapes times n, so that the escapes take the family's shapes in turn.
  // The run after an escape proves nothing where its control does not stand after the escape: each of the control's
  // programs makes the class, and each method and constructor part that makes it (a superclass's and a trait's
  // argument apart, which come before any statement) holds the escape's statement first.
  def selfTest(escape: Escape): Unit =
    val first = escapeText(escape, "topv")
    def escapedFirst(shape: Shape, host: String, written: Option[Escape]): Boolean =
      val made = program(host, shape, written)._1.linesIterator.filter(line => line.contains(shape.expr) && !line.startsWith("class K")).toVector
      made.nonEmpty && made.forall(line => line.indexOf(first) >= 0 && line.indexOf(first) < line.indexOf(shape.expr))
    if escapeShapes.forall(shape => hostsOf(shape, Some(escape)).isEmpty) then Log.die(s"lowered-shapes self-test: no control after the escape ${escape.name}")
    for shape <- escapeShapes; host <- hostsOf(shape, Some(escape)) do
      if !escapedFirst(shape, host, Some(escape)) then Log.die(s"lowered-shapes self-test: ${shape.name}.$host makes the class without the escape ${escape.name} first")
      // A planted bad record: the same program with no escape must fail the test.
      if escapedFirst(shape, host, None) then Log.die(s"lowered-shapes self-test: ${shape.name}.$host passes with no escape")

  def sample(taken: Vector[Shape], n: Int, escape: Option[Escape]): Vector[Shape] =
    val turn = escape.map(escapes.indexOf).getOrElse(0) * n
    val families = taken.groupBy(_.name.takeWhile(_ != '-'))
    taken.filter { shape =>
      val family = families(shape.name.takeWhile(_ != '-'))
      val at = family.indexOf(shape)
      (0 until n.min(family.length)).exists(j => (turn + j) % family.length == at)
    }

@main def loweredShapes(argv: String*): Unit = Script.run {
  import LoweredShapes.*
  val args = new Args(argv, "tests/support/lowered-shapes.scala --reference <binary> [--teq <binary>] [--out <dir>] [--jobs <n>] [--shapes <regex>] " +
    s"[--after-escape <${escapes.map(_.name).mkString("|")}>] [--family-sample <n>]")
  val reference = Paths.get(args.option("reference").getOrElse(args.fail("no --reference"))).toAbsolutePath.toString
  val teq = Paths.get(args.teq("./target/release/teq")).toAbsolutePath.toString
  val out = Paths.get(args.option("out").getOrElse("out/lowered-shapes"))
  val jobs = args.option("jobs").map(_.toInt).getOrElse(Runtime.getRuntime.availableProcessors)
  // The shapes whose names the expression matches whole, for a slice's own rows; all of them by default.
  val only = args.option("shapes").map(_.r)
  // Every shape after the escape named, in the same method; the forms that hold a shape there beside its positions.
  val escape = args.option("after-escape").map(name => escapes.find(_.name == name).getOrElse(args.fail(s"no escape $name")))
  val perFamily = args.option("family-sample").map(_.toInt)
  args.exactly(0)
  if !Files.isDirectory(Paths.get("tests/support")) then Log.die("tests/support/lowered-shapes.scala runs from the repository's root")
  val m2 = Option(System.getenv("COURSIER_CACHE")).filter(_.nonEmpty).getOrElse(System.getProperty("user.home") + "/.cache/coursier/v1") + "/https/repo1.maven.org/maven2"
  val lib = s"$m2/org/scala-lang/scala-library/3.8.4/scala-library-3.8.4.jar"
  if !Files.exists(Paths.get(lib)) then Log.die(s"lowered-shapes: no scala-library 3.8.4 in the coursier cache ($lib)")

  val src = out.resolve("src")
  Sh.remove(out)
  Files.createDirectories(src)
  var total = 0
  val programs = Vector.newBuilder[String]
  val matched = shapes.filter(shape => only.forall(_.matches(shape.name)))
  val taken = perFamily.fold(matched)(sample(matched, _, escape)) ++ escape.fold(Vector.empty[Shape])(_ => escapeShapes)
  escape.foreach(selfTest)
  for shape <- taken; host <- hostsOf(shape, escape) do
    val (text, n) = program(host, shape, escape)
    val name = s"${shape.name}.$host"
    Files.writeString(src.resolve(s"$name.scala"), text)
    programs += name
    total += n
  val names = programs.result()
  val after = escape.fold("")(e => s", after the escape ${e.name}")
  println(s"lowered-shapes: ${taken.length} shapes, ${names.length} programs, $total cases (shape by host by position by want)$after")

  // A build's files by their paths, each with its bytes: a jar whole, a products directory file by file.
  def files(p: Path): Vector[(String, Path)] =
    if Files.isDirectory(p) then
      val walk = Files.walk(p)
      try walk.toArray.map(_.asInstanceOf[Path]).filter(f => Files.isRegularFile(f)).map(f => p.relativize(f).toString -> f).sortBy(_._1).toVector finally walk.close()
    else if Files.isRegularFile(p) then Vector("" -> p)
    else Vector.empty
  def same(a: Path, b: Path): Boolean =
    val (x, y) = (files(a), files(b))
    x.nonEmpty && x.map(_._1) == y.map(_._1) && x.zip(y).forall { case ((_, f), (_, g)) => java.util.Arrays.equals(Files.readAllBytes(f), Files.readAllBytes(g)) }

  // The four ways a program is built: as a program or as products, by one worker or eight.
  val ways = Vector(("program", "1"), ("program", "8"), ("products", "1"), ("products", "8"))
  def target(side: String, name: String, mode: String, threads: String): Path =
    out.resolve(side).resolve(s"$mode-$threads").resolve(if mode == "program" then s"$name.jar" else name)
  final case class Build(side: String, binary: String, name: String, mode: String, threads: String)
  val builds = for (side, binary) <- Vector(("reference", reference), ("head", teq)); name <- names; (mode, threads) <- ways yield Build(side, binary, name, mode, threads)
  val status = scala.collection.mutable.Map.empty[Build, Int]
  Sh.pool(jobs, builds) { b =>
    val to = target(b.side, b.name, b.mode, b.threads)
    Files.createDirectories(to.getParent)
    val where = if b.mode == "program" then Vector("-o", to.toString) else Vector("--products", to.toString)
    val cmd = Vector(b.binary, "compiler", "build", src.resolve(s"${b.name}.scala").toString, "--std=scala-library", "--classpath", lib, "--target", "jvm", "--threads", b.threads) ++ where
    Sh.Then(Sh(cmd*).stdout(Paths.get(to.toString + ".log")).mergeErr.timeout(120).check(false), r => { status(b) = r.status; Sh.End })
  }

  var failed = false
  for (mode, threads) <- ways do
    var identical = 0
    var differ = 0
    var unbuilt = 0
    for name <- names do
      val (a, b) = (target("reference", name, mode, threads), target("head", name, mode, threads))
      val codes = Vector("reference", "head").map(side => status.getOrElse(Build(side, if side == "head" then teq else reference, name, mode, threads), -1))
      if codes.exists(_ != 0) || files(a).isEmpty then
        unbuilt += 1
        println(s"NOT BUILT $name ($mode, $threads): the reference's exit ${codes(0)}, the head's ${codes(1)} (${b.toString}.log)")
      else if same(a, b) then identical += 1
      else
        differ += 1
        println(s"differs: $name ($mode, $threads)")
    val workers = if threads == "1" then "one worker" else s"$threads workers"
    println(s"lowered-shapes $mode, $workers: $identical of ${names.length} programs identical, $differ differ, $unbuilt not built")
    if differ > 0 || unbuilt > 0 then failed = true
  // The control after an escape, on a line of its own: a head that cannot write it reads "not built" there.
  for e <- escape do
    val control = names.filter(name => escapeShapes.exists(shape => name.startsWith(shape.name + ".")))
    val built = control.count(name => ways.forall { case (mode, threads) => status.get(Build("head", teq, name, mode, threads)).contains(0) })
    println(s"lowered-shapes control (a given object's value class made after the escape ${e.name}): ${control.length} programs, $built built by the head four ways")
  // One binary's own builds must not depend on the workers either.
  var workersDiffer = 0
  for name <- names; mode <- Vector("program", "products") if !same(target("head", name, mode, "1"), target("head", name, mode, "8")) do
    workersDiffer += 1
    println(s"differs between one worker and eight: $name ($mode)")
  println(s"lowered-shapes workers: $workersDiffer of ${names.length * 2} head builds differ between one worker and eight")
  if failed || workersDiffer > 0 then Script.exit(1)
}
