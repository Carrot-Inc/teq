// A type-class codec: a case-class tree encoded to a string and decoded back through instances
// written by hand for Int, Long, Boolean, String, Option, List, a sorted Map and the two case
// classes, found through context bounds and a given search at the call site. Strings are
// length-prefixed and numbers terminated, so nothing is escaped, and every number is parsed by
// a character loop. Only
// Longs and Ints reach the checksum.
object CodecBench:
  // Rounds, timed iterations and whether the per-iteration line is printed on stderr, when the
  // command line gives none; bench/runtime.sh compiles copies with its own values in this line,
  // since Scala.js gives main no arguments.
  val defaults = "200 1 0"

  val modulus = 1000000007L

  def mix(acc: Long, x: Long): Long = (acc * 31L + x) % modulus

  final class Cursor(val text: String):
    var pos: Int = 0
    def peek: Char = text.charAt(pos)
    def take(c: Char): Unit =
      if text.charAt(pos) != c then throw new IllegalArgumentException("expected " + c + " at " + pos)
      pos += 1
    def digits: Long =
      var negative = false
      if text.charAt(pos) == '-' then
        negative = true
        pos += 1
      var v = 0L
      while pos < text.length && text.charAt(pos) >= '0' && text.charAt(pos) <= '9' do
        v = v * 10L + (text.charAt(pos) - '0').toLong
        pos += 1
      if negative then -v else v
    def chars(n: Int): String =
      val s = text.substring(pos, pos + n)
      pos += n
      s

  trait Codec[A]:
    def encode(a: A, sb: StringBuilder): Unit
    def decode(in: Cursor): A

  object Codec:
    def apply[A](using c: Codec[A]): Codec[A] = c

    given Codec[Int] with
      def encode(a: Int, sb: StringBuilder): Unit = sb.append(a).append(';')
      def decode(in: Cursor): Int =
        val v = in.digits.toInt
        in.take(';')
        v

    given Codec[Long] with
      def encode(a: Long, sb: StringBuilder): Unit = sb.append(a).append(';')
      def decode(in: Cursor): Long =
        val v = in.digits
        in.take(';')
        v

    given Codec[Boolean] with
      def encode(a: Boolean, sb: StringBuilder): Unit = sb.append(if a then 'T' else 'F')
      def decode(in: Cursor): Boolean =
        val c = in.peek
        in.take(c)
        c == 'T'

    given Codec[String] with
      def encode(a: String, sb: StringBuilder): Unit = sb.append(a.length).append(':').append(a)
      def decode(in: Cursor): String =
        val n = in.digits.toInt
        in.take(':')
        in.chars(n)

    given [A](using c: Codec[A]): Codec[Option[A]] with
      def encode(a: Option[A], sb: StringBuilder): Unit = a match
        case Some(x) =>
          sb.append('S')
          c.encode(x, sb)
        case None => sb.append('N')
      def decode(in: Cursor): Option[A] =
        val tag = in.peek
        in.take(tag)
        if tag == 'S' then Some(c.decode(in)) else None

    given [A](using c: Codec[A]): Codec[List[A]] with
      def encode(as: List[A], sb: StringBuilder): Unit =
        sb.append('[').append(as.length).append(':')
        as.foreach(a => c.encode(a, sb))
        sb.append(']')
      def decode(in: Cursor): List[A] =
        in.take('[')
        val n = in.digits.toInt
        in.take(':')
        var out: List[A] = Nil
        var i = 0
        while i < n do
          out = c.decode(in) :: out
          i += 1
        in.take(']')
        out.reverse

    // Entries sorted by key, so the text is the same whatever the map's iteration order.
    given [V](using c: Codec[V]): Codec[Map[String, V]] with
      def encode(m: Map[String, V], sb: StringBuilder): Unit =
        val entries = m.toList.sortBy(_._1)
        sb.append('{').append(entries.length).append(':')
        entries.foreach { (k, v) =>
          Codec[String].encode(k, sb)
          c.encode(v, sb)
        }
        sb.append('}')
      def decode(in: Cursor): Map[String, V] =
        in.take('{')
        val n = in.digits.toInt
        in.take(':')
        var out = Map.empty[String, V]
        var i = 0
        while i < n do
          val k = Codec[String].decode(in)
          out = out.updated(k, c.decode(in))
          i += 1
        in.take('}')
        out

  final case class Tag(key: String, value: Int)
  final case class Tree(label: String, weight: Long, active: Boolean, note: Option[String],
                        tags: List[Tag], counts: Map[String, Int], children: List[Tree])

  given Codec[Tag] with
    def encode(t: Tag, sb: StringBuilder): Unit =
      Codec[String].encode(t.key, sb)
      Codec[Int].encode(t.value, sb)
    def decode(in: Cursor): Tag =
      val key = Codec[String].decode(in)
      Tag(key, Codec[Int].decode(in))

  given Codec[Tree] with
    def encode(t: Tree, sb: StringBuilder): Unit =
      sb.append('(')
      Codec[String].encode(t.label, sb)
      Codec[Long].encode(t.weight, sb)
      Codec[Boolean].encode(t.active, sb)
      Codec[Option[String]].encode(t.note, sb)
      Codec[List[Tag]].encode(t.tags, sb)
      Codec[Map[String, Int]].encode(t.counts, sb)
      Codec[List[Tree]].encode(t.children, sb)
      sb.append(')')
    def decode(in: Cursor): Tree =
      in.take('(')
      val label = Codec[String].decode(in)
      val weight = Codec[Long].decode(in)
      val active = Codec[Boolean].decode(in)
      val note = Codec[Option[String]].decode(in)
      val tags = Codec[List[Tag]].decode(in)
      val counts = Codec[Map[String, Int]].decode(in)
      val children = Codec[List[Tree]].decode(in)
      in.take(')')
      Tree(label, weight, active, note, tags, counts, children)

  extension [A](a: A)(using c: Codec[A])
    def encoded: String =
      val sb = new StringBuilder
      c.encode(a, sb)
      sb.toString

  def decode[A: Codec](text: String): A = summon[Codec[A]].decode(new Cursor(text))

  val words = Vector("alpha", "beta", "gamma", "delta", "eps", "zeta", "eta", "theta")

  final class Gen(var state: Int):
    def next(bound: Int): Int =
      state = state * 1103515245 + 12345
      ((state >>> 8) & 0x7fffffff) % bound

  // Elements made in index order, which List.tabulate does not promise for an effectful function.
  def listOf[A](n: Int)(f: Int => A): List[A] =
    var out: List[A] = Nil
    var i = 0
    while i < n do
      out = f(i) :: out
      i += 1
    out.reverse

  def build(g: Gen, depth: Int): Tree =
    val label = words(g.next(8)) + g.next(100)
    val tags = listOf(g.next(4))(i => Tag(words((i + g.next(3)) % 8), g.next(1000) - 500))
    val counts = listOf(g.next(4))(i => (words((i * 3 + g.next(2)) % 8), g.next(50))).toMap
    val note = if g.next(3) == 0 then None else Some(words(g.next(8)) + "-" + words(g.next(8)))
    val children = if depth == 0 then Nil else listOf(g.next(4))(_ => build(g, depth - 1))
    Tree(label, g.next(1000000).toLong * 1000L - 7L, g.next(2) == 1, note, tags, counts, children)

  def weightSum(t: Tree): Long =
    t.children.foldLeft(t.weight % modulus)((a, c) => (a + weightSum(c)) % modulus)

  def charHash(text: String): Long =
    var h = 0L
    var i = 0
    while i < text.length do
      h = (h * 131L + text.charAt(i).toLong) % modulus
      i += 1
    h

  def round(seed: Int): Long =
    val g = new Gen(seed)
    val tree = build(g, 4)
    val text = tree.encoded
    val back = decode[Tree](text)
    var acc = text.length.toLong
    acc = mix(acc, charHash(text))
    acc = mix(acc, if back == tree then 1L else 0L)
    acc = mix(acc, weightSum(back))
    acc = mix(acc, if back.encoded == text then 1L else 0L)
    val tags = List.tabulate(32)(i => Tag(words(i % 8), i * seed))
    val tagText = tags.encoded
    acc = mix(acc, if decode[List[Tag]](tagText) == tags then tagText.length.toLong else 0L)
    val opt = decode[Option[List[Int]]]((Some(List(seed, seed + 1, seed + 2)): Option[List[Int]]).encoded)
    acc = mix(acc, opt.map(_.sum).getOrElse(0).toLong)
    acc

  def work(rounds: Int): Long =
    var acc = 0L
    var r = 0
    while r < rounds do
      acc = mix(acc, round(r * 977 + 5))
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
    if report then System.err.println(line("codec", n, times))
    println("codec checksum " + checksum)

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
