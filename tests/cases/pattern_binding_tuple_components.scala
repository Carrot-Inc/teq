// teq: --werror
// The tuple pattern definitions scalac 3.8.4 accepts as `Desugar.makePatDef` lowers them: a
// conforming typed component (`val h: HttpHeader = $1._2`), a literal adapting to its component's
// type (`val b: Long = 2`), an `if` of tuples (`SimpleTuple`, its holder of the join's type), a
// component in parentheses of its own (the general match, exempt by `.runtimeChecked`), members
// and locals, a lazy one over a block evaluated once, and a lazy one over a tuple literal split into
// a `def` per element as 3.8.4 splits it (`DefDef(name, Nil, tpt, elem)`), each read evaluating it.
// A wildcard beside a typed component over a tuple literal keeps the holder (`SimpleTuple`: `val
// $1 = (1, 2); val b: Long = $1._2`), the element adapting to the written type (`Int.int2long`),
// a local as a member, a tuple literal in parentheses too (`forallResults` looks through `Parens`),
// and the results of an `inline if` and an `inline match` (`InlineIf` and `InlineMatch` extend `If` and
// `Match`), expanded where the inline method is called.
package patternbindingtuplecomponents

class HttpHeader:
  override def toString = "header"
class Accept extends HttpHeader:
  override def toString = "accept"
def parseLine(): (Int, HttpHeader) = (1, new Accept)

inline def inlineIf(inline flag: Boolean): Long =
  val (_, b: Long) = inline if flag then ((1, 2)) else ((3, 4))
  b
inline def inlineMatch(inline flag: Boolean): Long =
  val (_, b: Long) = inline flag match
    case true => ((5, 6))
    case false => ((7, 8))
  b

object Members:
  val (n, h: HttpHeader) = parseLine()
  val (a, b: Long) = (1, 2)
  val (_, mb: Long) = (1, 2)
  val (_, pmb: Long) = ((3, 4))
  val (_, accept: Accept) = if n > 0 then (1, new Accept) else (2, new Accept)
  // An annotated right side, not in parentheses of its own, takes the general match
  // (`!rhs.isInstanceOf[Annotated]`), exempt by the annotation.
  val (_, viaAnnot: Accept) = parseLine(): @scala.annotation.internal.RuntimeChecked
  lazy val (x, y: String) = { println("once"); (3, "three") }
  lazy val (p, q) = ({ println("p"); 4 }, { println("q"); 5 })
  // A declared type over a tuple value (dotty's pos/tuplePatDef and pos/t2613 over `null`).
  def pair: (String, Int) = ("s", 6)
  val (s, i): (String, Int) = pair

object Main:
  def main(args: Array[String]): Unit =
    val (n, h: HttpHeader) = parseLine()
    val (a, b: Long) = (1, 2)
    val (i: Int, rest) = parseLine().runtimeChecked
    val (k, (d: Accept)) = parseLine().runtimeChecked
    val (_, accept: Accept) = if args.isEmpty then (1, new Accept) else (2, new Accept)
    println(s"$n $h ${b + 1L} $i $rest $k $d $accept")
    println(s"${Members.n} ${Members.h} ${Members.b + 1L} ${Members.accept} ${Members.viaAnnot}")
    println(s"${Members.y} ${Members.x} ${Members.y}")
    println(Members.p + Members.p + Members.q)
    println(Members.s.length + Members.i)
    val (_, wb: Long) = (1, 2)
    val (_, wc: Long, wd) = (3, 4, "five")
    var (_, wv: Double) = (5, 6)
    wv += 0.5
    println(s"$wb $wc $wd $wv ${Members.mb + 1L}")
    val (_, pb: Long) = ((5, 6))
    val (pa, pc: Long) = (if args.isEmpty then ((7, 8)) else ((9, 10)))
    println(s"${Members.pmb} $pb $pa $pc")
    println(s"${inlineIf(true)} ${inlineIf(false)} ${inlineMatch(true)} ${inlineMatch(false)}")
