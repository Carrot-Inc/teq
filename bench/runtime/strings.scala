// Strings: StringBuilder, interpolation, split, parsing numbers, mkString, and the character
// loops underneath them. One round formats 48 records, parses them back and folds a hash over
// the result. Separators are plain characters, since teq takes a one-character separator
// literally where scalac reads a regular expression.
object StringsBench:
  // Rounds, timed iterations and whether the per-iteration line is printed on stderr, when the
  // command line gives none; bench/runtime.sh compiles copies with its own values in this line,
  // since Scala.js gives main no arguments.
  val defaults = "200 1 0"

  val modulus = 1000000007L
  val names = Vector("ann", "bob", "cyril", "dee", "evelyn", "fay", "gus", "hal")
  val tags = Vector("north", "south", "east", "west")

  final case class Record(id: Int, name: String, tag: String, qty: Int, cents: Int)

  def mix(acc: Long, x: Long): Long = (acc * 31L + x) % modulus

  def record(k: Int): Record =
    Record(k, names((k * 5) % 8), tags((k * 3) % 4), 1 + (k % 9), 100 + (k * 37) % 9000)

  // Interpolation into one line per record, joined with mkString.
  def format(rs: List[Record]): String =
    rs.map(r => s"${r.id},${r.name},${r.tag},${r.qty},${r.cents}").mkString(";")

  // The same lines built through a StringBuilder, which is what a hot formatter uses.
  def build(rs: List[Record]): String =
    val sb = new StringBuilder
    var rest = rs
    var first = true
    while rest.nonEmpty do
      val r = rest.head
      rest = rest.tail
      if first then first = false else sb.append(';')
      sb.append(r.id)
      sb.append(',')
      sb.append(r.name)
      sb.append(',')
      sb.append(r.tag)
      sb.append(',')
      sb.append(r.qty)
      sb.append(',')
      sb.append(r.cents)
    sb.toString

  // split and toInt, the parsing half.
  def parse(line: String): List[Record] =
    val parts = line.split(";")
    var out: List[Record] = Nil
    var i = parts.length
    while i > 0 do
      i -= 1
      val fields = parts(i).split(",")
      out = Record(fields(0).toInt, fields(1), fields(2), fields(3).toInt, fields(4).toInt) :: out
    out

  // Character work: a case fold, a digit count and a hash, written as an explicit loop so that
  // the Char and String representations of the two platforms cannot differ.
  def scan(text: String): Long =
    var hash = 0L
    var digits = 0
    var upper = 0
    var i = 0
    while i < text.length do
      val c = text.charAt(i)
      hash = (hash * 131L + c.toInt.toLong) % modulus
      if c >= '0' && c <= '9' then digits += 1
      if c >= 'a' && c <= 'z' then upper += 1
      i += 1
    mix(hash, digits.toLong * 1000L + upper.toLong)

  // The string operations an application reaches for: substring, indexOf, replace, trim,
  // startsWith, toUpperCase, reverse, padding through a builder.
  def shapes(rs: List[Record]): Long =
    var acc = 0L
    var rest = rs
    while rest.nonEmpty do
      val r = rest.head
      rest = rest.tail
      val label = "  " + r.name + ":" + r.tag + "  "
      val trimmed = label.trim
      val at = trimmed.indexOf(":")
      val head = trimmed.substring(0, at)
      val tail = trimmed.substring(at + 1)
      acc = mix(acc, head.length.toLong)
      acc = mix(acc, tail.toUpperCase.length.toLong)
      acc = mix(acc, if trimmed.startsWith("ann") then 1L else 0L)
      acc = mix(acc, trimmed.replace("north", "N").length.toLong)
      acc = mix(acc, head.reverse.charAt(0).toInt.toLong)
      acc = mix(acc, (head + "/" + tail).contains("/").toString.length.toLong)
    acc

  // Joining and splitting the other way around, with mkString's three-argument form.
  def joins(rs: List[Record]): Long =
    val ids = rs.map(_.id).mkString("[", ", ", "]")
    val uniqueNames = rs.map(_.name).distinct.sorted.mkString(" ")
    val words = uniqueNames.split(" ")
    var acc = ids.length.toLong
    acc = mix(acc, words.length.toLong)
    var i = 0
    while i < words.length do
      acc = mix(acc, words(i).length.toLong)
      i += 1
    acc = mix(acc, rs.map(r => r.name + r.tag).mkString.length.toLong)
    acc

  def records(seed: Int, count: Int): List[Record] =
    var acc: List[Record] = Nil
    var i = count
    while i > 0 do
      i -= 1
      acc = record(seed + i) :: acc
    acc

  def round(seed: Int): Long =
    val rs = records(seed, 48)
    val formatted = format(rs)
    val built = build(rs)
    var acc = if formatted == built then 1L else 0L
    acc = mix(acc, formatted.length.toLong)
    acc = mix(acc, scan(formatted))
    val parsed = parse(built)
    acc = mix(acc, parsed.foldLeft(0L)((a, r) => mix(a, r.cents.toLong + r.qty.toLong)))
    acc = mix(acc, if parsed == rs then 2L else 0L)
    acc = mix(acc, shapes(rs))
    acc = mix(acc, joins(rs))
    acc

  def work(rounds: Int): Long =
    var acc = 0L
    var r = 0
    while r < rounds do
      acc = mix(acc, round(r * 11))
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
    if report then System.err.println(line("strings", n, times))
    println("strings checksum " + checksum)

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
