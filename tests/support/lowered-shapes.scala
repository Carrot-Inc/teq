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
// `./teq interp tests/support/lowered-shapes.scala -- --reference <binary> [--teq <binary>] [--out <dir>] [--jobs <n>]`,
// the reference the base's compiler, the compiler under test `--teq`, else $TEQ, else target/release/teq; the
// programs and the outputs under `--out` (out/lowered-shapes); `--jobs` builds at once (the processors).
import java.nio.file.{Files, Path, Paths}

// What a shape is: the statements before it, the expression, its type, and the hosts it needs ("" a class and a
// constructor, "method" a class alone, "members" the four hosts with members, "class" the two with constructor
// parameters, "value" a value class, "constructor" a constructor alone). `String.formatted` is "method": in a
// constructor the typer does not find it with eight workers today.
final case class Shape(name: String, before: String, expr: String, tpe: String, hosts: String)

// A position: the text of a method `$n` whose body holds the expression `$E` where `$W` is wanted; `$P` the
// parameters, `$S` the statements before.
final case class Position(name: String, text: String, fits: String = "")

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
  val ff: Int => Int = (x: Int) => x + 1
  def abs: Int = fi
  def m0(): Int = fi
  def m1(x: Int): Long = x.toLong
  def ms(x: String, y: Int): String = x + y
  def mu(): Unit = ()
  def mg[A](a: A): A = a
  def dflt(x: Int = 4, y: String = "d"): String = y + x
  def va(xs: Int*): Int = xs.length
"""

  val prelude = """final class V(val u: Int) extends AnyVal {
  def twice: V = new V(u * 2)
}
class Box[A](val item: A) {
  def get: A = item
  def put(a: A): Box[A] = new Box[A](a)
}
trait Tr {
  def abs: Int
  def im(x: Int): Int = x + abs
  def sd: Int = 7
  val tv: String = "tv"
}
class CBase[A](val got: A)
class Base(val bn: Int) {
  def inherited(x: Int): Int = x + bn
  def over(x: Int): Any = x
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
}
extension (x: Int) def +:(s: String): String = s + x
class Peer(val fi: Int, val fs: String, pp: Int, pq: String) extends Tr {""" + members + """}
object Takes {
  def any(w: Any): Unit = ()
  def id[A](a: A): A = a
  def lazily[A](a: => A): Unit = { a; () }
}
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
    "T" -> Vector("T", "Any"),
    "THIS" -> Vector("THIS", "Any", "AnyRef", "Tr"),
    "O.type" -> Vector("O.type", "Any", "AnyRef"),
    "Box[Int]" -> Vector("Box[Int]", "Any", "AnyRef"),
    "Int => Int" -> Vector("Int => Int", "Any", "AnyRef"),
    "java.util.List[String]" -> Vector("java.util.List[String]", "Any"))
  // What the typer narrows an integer literal to, beside the types an Int is wanted as.
  val narrowed = Vector("Byte", "Short", "Char")
  val numeric = Set("Int", "Long", "Float", "Double", "Char", "Byte", "Short")

  val shapes: Vector[Shape] = Vector(
    Shape("lit-int", "", "1", "Int", ""), Shape("lit-long", "", "1L", "Long", ""), Shape("lit-double", "", "1.5", "Double", ""),
    Shape("lit-float", "", "1.5f", "Float", ""), Shape("lit-bool", "", "true", "Boolean", ""), Shape("lit-char", "", "'a'", "Char", ""),
    Shape("lit-string", "", "\"s\"", "String", ""), Shape("lit-unit", "", "()", "Unit", ""),
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
    Shape("field-lazy", "", "lz", "String", "members"), Shape("field-function", "", "ff", "Int => Int", "members"),
    Shape("field-other-int", "", "other.fi", "Int", ""), Shape("field-other-string", "", "other.fs", "String", ""),
    Shape("field-other-var", "", "other.fv", "Long", ""), Shape("field-other-value-class", "", "other.fvc", "V", ""),
    Shape("field-other-lazy", "", "other.lz", "String", ""), Shape("field-box-int", "", "pbox.item", "Int", ""),
    Shape("field-box-string", "", "psb.item", "String", ""), Shape("field-inherited", "", "sub.bn", "Int", ""),
    Shape("field-trait-val", "", "tr.tv", "String", ""), Shape("field-trait-def", "", "tr.abs", "Int", ""),
    Shape("field-object-val", "", "O.k", "Int", ""), Shape("field-object-var", "", "O.kv", "String", ""),
    Shape("field-param-underlying", "", "pv.u", "Int", ""),
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
    Shape("call-right-associative-member", "import O.*", "(pi ++: ps)", "String", ""),
    Shape("call-java-array", "", "java.util.Arrays.asList(ps, \"x\")", "java.util.List[String]", ""),
    Shape("call-java-array-member", "", "ps.formatted(pa, ps)", "String", "method"),
    Shape("apply-one", "", "pfn(pi)", "Int", ""), Shape("apply-none", "", "pf0()", "String", ""),
    Shape("apply-two", "", "pf2(pi, ps)", "String", ""), Shape("apply-field", "", "ff(pi)", "Int", "members"),
    Shape("super-call", "", "super.im(pi)", "Int", "members"), Shape("super-def", "", "super.sd", "Int", "members"),
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
    Shape("try-ret-sync", "def rts(x: Int): Double = try other.synchronized { val y: Double = if x > 0 then return pd else 1.5; y } finally other.mu()", "rts(pi)", "Double", ""))

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

  val hosts = Vector("class", "open", "object", "trait")
  // The shapes a value class's method holds beside its own.
  val inValueClass = Set("lit-int", "param-int", "param-string", "param-generic", "this")

  def wantsOf(shape: Shape, host: String): Vector[String] =
    val own = wants.find(_._1 == shape.tpe).map(_._2).getOrElse(Vector(shape.tpe))
    val held = if host == "value" then own.filter(w => w != "AnyRef" && w != "Tr") else own
    if shape.expr == "1" then held ++ narrowed else held

  // The cases of a shape in a method of a host: (position, want, the method's text).
  def cases(shape: Shape, host: String): Vector[(String, String, String)] =
    val all = wantsOf(shape, host)
    val out = Vector.newBuilder[(String, String, String)]
    for want <- all; position <- positions do
      // The statement position wants nothing: once. The JVM back end refuses a `try` as an argument of a
      // constructor, with its place. A narrowed literal's type is written by the literal alone.
      val left = position.name == "statement" && want != all.head ||
        shape.name.startsWith("try-") && (position.name == "element" || position.name == "constructor-argument") ||
        narrowed.contains(want) && shape.tpe == "Int" && position.name == "generic-argument"
      if !left then out += ((position.name, want, position.text))
    for position <- ownPositions do
      val fits = shape.tpe != "Unit" && (position.fits match
        case "Boolean" => shape.tpe == "Boolean"
        case "numeric" => numeric(shape.tpe)
        case _ => true)
      if fits then out += ((position.name, shape.tpe, position.text))
    out.result()

  def method(text: String, n: String, shape: Shape, want: String): String =
    text.replace("$$", "\u0000").replace("$n", n).replace("$P", params).replace("$S", if shape.before.isEmpty then "()" else shape.before)
      .replace("$E", shape.expr).replace("$W", want).replace("\u0000", "$")

  // Whether the expression names none of a method's parameters, so that a field's initialiser may hold it.
  def closed(shape: Shape): Boolean =
    shape.before.isEmpty && !"\\b(p(i|l|d|f|b|c|by|sh|s|a|t|v|box|fn|f0|f2)|psb|other|sub|tr)\\b".r.findFirstIn(shape.expr).isDefined

  // A program and the number of its cases.
  def program(host: String, shape: Shape): (String, Int) =
    if host == "constructor" then constructors(shape)
    else
      val made = cases(shape, host).zipWithIndex.map { case ((_, want, text), i) => method(text, s"g$i", shape, want) }
      val calls = made.indices.map(i => s"    Takes.any(h.g$i[String]($arguments))\n").mkString
      // A field's initialiser, where the expression stands in the class's body.
      val fields = if host != "value" && shape.hosts != "" && shape.hosts != "method" && closed(shape) then wantsOf(shape, host).zipWithIndex.map { case (want, i) => s"  val init$i: $want = ${shape.expr}\n" } else Vector.empty
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
      ((prelude + text + main).replace("THIS", self), made.length + fields.length)

  // A class per want whose constructor holds the expression: its superclass's argument, a val's and a var's
  // initialisers, a statement. Not a superclass's argument: a `try`, which the JVM back end refuses there, and a
  // Java generic method's call, which the product mode does not pickle there today.
  def constructors(shape: Shape): (String, Int) =
    val all = wantsOf(shape, "constructor")
    val bare = shape.name.startsWith("try-") || shape.name == "call-java-array"
    val classes = all.zipWithIndex.map { case (want, i) =>
      val parent = if bare then "" else s" extends CBase[$want](${shape.expr})"
      s"class K$i[T](val vx: Int, $params)$parent {\n  val init: $want = ${shape.expr}\n  var later: $want = ${shape.expr}\n  Takes.any(${shape.expr})\n  def held: Int = vx\n}\n"
    }
    val calls = all.indices.map(i => s"    Takes.any(new K$i[String](0, $arguments).held)\n").mkString
    val main = s"object Main {\n  def main(args: Array[String]): Unit = {\n$calls  }\n}\n"
    (prelude + classes.mkString + main, all.length * (if bare then 3 else 4))

  def hostsOf(shape: Shape): Vector[String] =
    val own = shape.hosts match
      case "" => if shape.before.isEmpty then Vector("class", "constructor") else Vector("class")
      case "class" => Vector("class", "open")
      case "method" => Vector("class")
      case "members" => hosts
      case other => Vector(other)
    if inValueClass(shape.name) then own :+ "value" else own

@main def loweredShapes(argv: String*): Unit = Script.run {
  import LoweredShapes.*
  val args = new Args(argv, "tests/support/lowered-shapes.scala --reference <binary> [--teq <binary>] [--out <dir>] [--jobs <n>]")
  val reference = Paths.get(args.option("reference").getOrElse(args.fail("no --reference"))).toAbsolutePath.toString
  val teq = Paths.get(args.teq("./target/release/teq")).toAbsolutePath.toString
  val out = Paths.get(args.option("out").getOrElse("out/lowered-shapes"))
  val jobs = args.option("jobs").map(_.toInt).getOrElse(Runtime.getRuntime.availableProcessors)
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
  for shape <- shapes; host <- hostsOf(shape) do
    val (text, n) = program(host, shape)
    val name = s"${shape.name}.$host"
    Files.writeString(src.resolve(s"$name.scala"), text)
    programs += name
    total += n
  val names = programs.result()
  println(s"lowered-shapes: ${shapes.length} shapes, ${names.length} programs, $total cases (shape by host by position by want)")

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
  // One binary's own builds must not depend on the workers either.
  var workersDiffer = 0
  for name <- names; mode <- Vector("program", "products") if !same(target("head", name, mode, "1"), target("head", name, mode, "8")) do
    workersDiffer += 1
    println(s"differs between one worker and eight: $name ($mode)")
  println(s"lowered-shapes workers: $workersDiffer of ${names.length * 2} head builds differ between one worker and eight")
  if failed || workersDiffer > 0 then Script.exit(1)
}
