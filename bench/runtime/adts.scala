// ADTs and pattern matching: an expression tree over a sealed hierarchy that is built, evaluated
// against an environment, simplified by matching on nested constructors and printed; Option and
// Either chains through a small parser; tuples built, matched and taken apart. Only Longs and
// Ints reach the checksum.
object AdtsBench:
  // Rounds, timed iterations and whether the per-iteration line is printed on stderr, when the
  // command line gives none; bench/runtime.sh compiles copies with its own values in this line,
  // since Scala.js gives main no arguments.
  val defaults = "50 1 0"

  val modulus = 1000000007L

  def mix(acc: Long, x: Long): Long = (acc * 31L + x) % modulus

  enum Op:
    case Plus, Minus, Times, Max

  sealed trait Expr
  final case class Num(value: Long) extends Expr
  final case class Ref(name: String) extends Expr
  final case class Bin(op: Op, left: Expr, right: Expr) extends Expr
  final case class Neg(inner: Expr) extends Expr
  final case class Let(name: String, bound: Expr, body: Expr) extends Expr
  final case class Cond(test: Expr, ifTrue: Expr, ifFalse: Expr) extends Expr

  type Env = List[(String, Long)]
  val names = Vector("a", "b", "c", "d")

  def lookup(env: Env, name: String): Option[Long] = env match
    case (n, v) :: rest => if n == name then Some(v) else lookup(rest, name)
    case Nil => None

  def apply(op: Op, a: Long, b: Long): Long = op match
    case Op.Plus => a + b
    case Op.Minus => a - b
    case Op.Times => (a * b) % modulus
    case Op.Max => if a > b then a else b

  def eval(e: Expr, env: Env): Long = e match
    case Num(v) => v
    case Ref(n) => lookup(env, n).getOrElse(0L)
    case Bin(op, l, r) => apply(op, eval(l, env), eval(r, env))
    case Neg(x) => -eval(x, env)
    case Let(n, b, body) => eval(body, (n, eval(b, env)) :: env)
    case Cond(t, a, b) => if eval(t, env) != 0L then eval(a, env) else eval(b, env)

  // Folding on nested constructor patterns with an enum case as a stable identifier.
  def simplify(e: Expr): Expr = e match
    case Bin(op, l, r) =>
      (op, simplify(l), simplify(r)) match
        case (Op.Plus, Num(0L), x) => x
        case (Op.Plus, x, Num(0L)) => x
        case (Op.Times, Num(1L), x) => x
        case (Op.Times, x, Num(1L)) => x
        case (Op.Times, Num(0L), _) => Num(0L)
        case (o, Num(a), Num(b)) => Num(apply(o, a, b))
        case (o, a, b) => Bin(o, a, b)
    case Neg(x) =>
      simplify(x) match
        case Neg(y) => y
        case Num(v) => Num(-v)
        case y => Neg(y)
    case Let(n, b, body) =>
      (simplify(b), simplify(body)) match
        case (Num(v), Ref(m)) if m == n => Num(v)
        case (sb, sbody) => Let(n, sb, sbody)
    case Cond(t, a, b) =>
      simplify(t) match
        case Num(v) => if v != 0L then simplify(a) else simplify(b)
        case st => Cond(st, simplify(a), simplify(b))
    case other => other

  def size(e: Expr): Int = e match
    case Num(_) | Ref(_) => 1
    case Bin(_, l, r) => 1 + size(l) + size(r)
    case Neg(x) => 1 + size(x)
    case Let(_, b, body) => 1 + size(b) + size(body)
    case Cond(t, a, b) => 1 + size(t) + size(a) + size(b)

  def render(e: Expr, sb: StringBuilder): Unit = e match
    case Num(v) => sb.append(v)
    case Ref(n) => sb.append(n)
    case Bin(op, l, r) =>
      sb.append('(')
      render(l, sb)
      sb.append(' ').append(op.toString).append(' ')
      render(r, sb)
      sb.append(')')
    case Neg(x) =>
      sb.append('-')
      render(x, sb)
    case Let(n, b, body) =>
      sb.append("let ").append(n).append(" = ")
      render(b, sb)
      sb.append(" in ")
      render(body, sb)
    case Cond(t, a, b) =>
      sb.append("if ")
      render(t, sb)
      sb.append(" then ")
      render(a, sb)
      sb.append(" else ")
      render(b, sb)

  // A deterministic tree from a linear congruential generator; the shape is chosen by the low
  // bits so that every constructor and every simplification rule is reached.
  final class Gen(var state: Int):
    def next(bound: Int): Int =
      state = state * 1103515245 + 12345
      ((state >>> 8) & 0x7fffffff) % bound

  def build(g: Gen, depth: Int): Expr =
    if depth <= 0 then
      if g.next(3) == 0 then Ref(names(g.next(4))) else Num(g.next(5).toLong)
    else
      g.next(9) match
        case 0 => Neg(build(g, depth - 1))
        case 1 => Let(names(g.next(4)), build(g, depth - 1), build(g, depth - 1))
        case 2 => Cond(build(g, depth - 2), build(g, depth - 1), build(g, depth - 1))
        case k =>
          val op = k % 4 match
            case 0 => Op.Plus
            case 1 => Op.Minus
            case 2 => Op.Times
            case _ => Op.Max
          Bin(op, build(g, depth - 1), build(g, depth - 1))

  // Option and Either chains: the rendered text is tokenised and parsed back, every step
  // through flatMap/map on Either and Option, and the result compared with the original.
  def tokens(text: String): List[String] =
    val out = scala.collection.mutable.ListBuffer.empty[String]
    var i = 0
    while i < text.length do
      val c = text.charAt(i)
      if c == ' ' then i += 1
      else if c == '(' || c == ')' || c == '-' || c == '=' then
        out += text.substring(i, i + 1)
        i += 1
      else
        val start = i
        while i < text.length && text.charAt(i) != ' ' && text.charAt(i) != '(' && text.charAt(i) != ')' do i += 1
        out += text.substring(start, i)
    out.toList

  def opOf(token: String): Option[Op] = token match
    case "Plus" => Some(Op.Plus)
    case "Minus" => Some(Op.Minus)
    case "Times" => Some(Op.Times)
    case "Max" => Some(Op.Max)
    case _ => None

  def number(token: String): Option[Long] =
    if token.nonEmpty && token.forall(c => c >= '0' && c <= '9') then Some(token.toLong) else None

  def parse(ts: List[String]): Either[String, (Expr, List[String])] = ts match
    case "(" :: rest =>
      parse(rest).flatMap { (left, afterLeft) =>
        afterLeft match
          case opToken :: afterOp =>
            opOf(opToken).toRight("operator expected: " + opToken).flatMap { op =>
              parse(afterOp).flatMap { (right, afterRight) =>
                afterRight match
                  case ")" :: tail => Right((Bin(op, left, right), tail))
                  case _ => Left("closing paren expected")
              }
            }
          case Nil => Left("operator expected at end")
      }
    case "-" :: rest => parse(rest).map((inner, tail) => (Neg(inner), tail))
    case "let" :: name :: "=" :: rest =>
      for
        bound <- parse(rest)
        body <- bound._2 match
          case "in" :: afterIn => parse(afterIn)
          case _ => Left("in expected")
      yield (Let(name, bound._1, body._1), body._2)
    case "if" :: rest =>
      for
        t <- parse(rest)
        a <- expect(t._2, "then").flatMap(parse)
        b <- expect(a._2, "else").flatMap(parse)
      yield (Cond(t._1, a._1, b._1), b._2)
    case token :: rest =>
      number(token).map(v => Num(v): Expr).orElse(if names.contains(token) then Some(Ref(token)) else None) match
        case Some(e) => Right((e, rest))
        case None => Left("unexpected token: " + token)
    case Nil => Left("unexpected end")

  def expect(ts: List[String], word: String): Either[String, List[String]] = ts match
    case w :: rest if w == word => Right(rest)
    case _ => Left(word + " expected")

  // Tuples: statistics as a triple, pairs zipped and swapped, and a tuple pattern in a fold.
  def stats(e: Expr): (Int, Int, Long) = e match
    case Num(v) => (1, 0, v)
    case Ref(_) => (1, 0, 0L)
    case Bin(_, l, r) =>
      val (sl, dl, vl) = stats(l)
      val (sr, dr, vr) = stats(r)
      (sl + sr + 1, (if dl > dr then dl else dr) + 1, (vl + vr) % modulus)
    case Neg(x) =>
      val (s, d, v) = stats(x)
      (s + 1, d + 1, v)
    case Let(_, b, body) =>
      val (sb, db, vb) = stats(b)
      val (sy, dy, vy) = stats(body)
      (sb + sy + 1, (if db > dy then db else dy) + 1, (vb + vy) % modulus)
    case Cond(t, a, b) =>
      val parts = List(stats(t), stats(a), stats(b))
      val (sizes, depths) = parts.map(p => (p._1, p._2)).unzip
      (sizes.sum + 1, depths.max + 1, parts.map(_._3).sum % modulus)

  def round(seed: Int): Long =
    val g = new Gen(seed)
    val env: Env = names.toList.zipWithIndex.map((n, i) => (n, (i * 7 + seed % 5).toLong))
    var acc = 0L
    var t = 0
    while t < 8 do
      val e = build(g, 7)
      val s = simplify(e)
      acc = mix(acc, eval(e, env))
      acc = mix(acc, eval(s, env))
      acc = mix(acc, size(e).toLong * 1000L + size(s).toLong)
      val sb = new StringBuilder
      render(s, sb)
      val text = sb.toString
      acc = mix(acc, text.length.toLong)
      val parsed = parse(tokens(text))
      acc = mix(acc, parsed.fold(msg => msg.length.toLong, r => if r._1 == s then 1L else if r._2.isEmpty && eval(r._1, env) == eval(s, env) then 3L else 2L))
      val (count, depth, total) = stats(s)
      acc = mix(acc, count.toLong + depth.toLong * 100L + total)
      val pairs = env.zip(env.reverse).map(_.swap)
      acc = pairs.foldLeft(acc) { case (a, ((n1, v1), (n2, v2))) => mix(a, v1 * 3L + v2 + n1.length.toLong + n2.length.toLong) }
      t += 1
    acc

  def work(rounds: Int): Long =
    var acc = 0L
    var r = 0
    while r < rounds do
      acc = mix(acc, round(r * 7919 + 1))
      r += 1
    acc

  def main(args: Array[String]): Unit =
    val d = defaults.split(" ")
    val n = (if args.length > 0 then args(0) else d(0)).toInt
    val iterations = (if args.length > 1 then args(1) else d(1)).toInt
    val report = (if args.length > 2 then args(2) else d(2)).toInt == 1
    val times = new Array[Long](iterations)
    var checksum = 0L
    var i = 0
    while i < iterations do
      val started = System.nanoTime()
      checksum = work(n)
      times(i) = System.nanoTime() - started
      i += 1
    if report then System.err.println(line("adts", n, times))
    println("adts checksum " + checksum)

  def line(name: String, n: Int, times: Array[Long]): String =
    var total = 0L
    var steady = 0L
    var i = 0
    while i < times.length do
      total += times(i)
      if i >= times.length / 2 then steady += times(i)
      i += 1
    "bench " + name + " n=" + n + " k=" + times.length + " first_ns=" + times(0) +
      " total_ns=" + total + " steady_ns=" + steady / (times.length - times.length / 2)
