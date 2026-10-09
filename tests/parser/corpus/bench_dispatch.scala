// Virtual dispatch and object layout: a shape hierarchy called through its trait, an abstract
// class whose template method calls an abstract step, default methods of three traits mixed
// into one class, anonymous classes over a trait, a visitor with double dispatch, and case classes as Map keys, which
// is structural equality and hashing. Only Longs and Ints reach the checksum.
object DispatchBench:
  // Rounds, timed iterations and whether the per-iteration line is printed on stderr, when the
  // command line gives none; bench/runtime.sh compiles copies with its own values in this line,
  // since Scala.js gives main no arguments.
  val defaults = "80 1 0"

  val modulus = 1000000007L

  def mix(acc: Long, x: Long): Long = (acc * 31L + x) % modulus

  trait Visitor[A]:
    def square(s: Square): A
    def rect(r: Rect): A
    def circle(c: Circle): A
    def tri(t: Tri): A

  trait Shape:
    def area: Long
    def scaled(k: Long): Shape
    def kind: Int
    def accept[A](v: Visitor[A]): A
    def describe: String = "shape" + kind

  final class Square(val side: Long) extends Shape:
    def area: Long = side * side
    def scaled(k: Long): Shape = new Square(side * k)
    def kind: Int = 1
    def accept[A](v: Visitor[A]): A = v.square(this)

  final class Rect(val w: Long, val h: Long) extends Shape:
    def area: Long = w * h
    def scaled(k: Long): Shape = new Rect(w * k, h * k)
    def kind: Int = 2
    def accept[A](v: Visitor[A]): A = v.rect(this)
    override def describe: String = "rect" + w

  final class Circle(val r: Long) extends Shape:
    def area: Long = 3L * r * r
    def scaled(k: Long): Shape = new Circle(r * k)
    def kind: Int = 3
    def accept[A](v: Visitor[A]): A = v.circle(this)

  final class Tri(val base: Long, val height: Long) extends Shape:
    def area: Long = base * height / 2L
    def scaled(k: Long): Shape = new Tri(base * k, height * k)
    def kind: Int = 4
    def accept[A](v: Visitor[A]): A = v.tri(this)

  def shape(i: Int): Shape = (i % 4) match
    case 0 => new Square((i % 7 + 1).toLong)
    case 1 => new Rect((i % 5 + 1).toLong, (i % 3 + 2).toLong)
    case 2 => new Circle((i % 6 + 1).toLong)
    case _ => new Tri((i % 8 + 2).toLong, (i % 4 + 1).toLong)

  // Interface calls over a heterogeneous array, the describe default and its override included.
  def shapesPass(count: Int, seed: Int): Long =
    val shapes = new Array[Shape](count)
    var i = 0
    while i < count do
      shapes(i) = shape(i + seed)
      i += 1
    var acc = 0L
    var pass = 0
    while pass < 4 do
      var j = 0
      while j < count do
        val s = shapes(j)
        acc = mix(acc, s.area + s.kind.toLong)
        if s.kind == 2 then shapes(j) = s.scaled(2L)
        j += 1
      pass += 1
    acc = mix(acc, shapes(seed % count).describe.length.toLong)
    acc

  val perimeter: Visitor[Long] = new Visitor[Long]:
    def square(s: Square): Long = 4L * s.side
    def rect(r: Rect): Long = 2L * (r.w + r.h)
    def circle(c: Circle): Long = 6L * c.r
    def tri(t: Tri): Long = t.base + 2L * t.height

  val corners: Visitor[Int] = new Visitor[Int]:
    def square(s: Square): Int = 4
    def rect(r: Rect): Int = 4
    def circle(c: Circle): Int = 0
    def tri(t: Tri): Int = 3

  def visitorPass(count: Int, seed: Int): Long =
    var acc = 0L
    var i = 0
    while i < count do
      val s = shape(i * 3 + seed)
      acc = mix(acc, s.accept(perimeter) + s.accept(corners).toLong)
      i += 1
    acc

  abstract class Machine:
    protected def step(x: Long): Long
    def run(x0: Long, steps: Int): Long =
      var x = x0
      var i = 0
      while i < steps do
        x = step(x) % modulus
        i += 1
      x

  final class Doubler extends Machine:
    protected def step(x: Long): Long = x * 2L + 1L

  final class Adder(k: Long) extends Machine:
    protected def step(x: Long): Long = x + k

  final class Xorer extends Machine:
    protected def step(x: Long): Long = (x * 3L) ^ 5L

  // The template method over three subclasses.
  def machinePass(steps: Int, seed: Int): Long =
    val machines: List[Machine] = List(new Doubler, new Adder(seed.toLong), new Xorer)
    machines.foldLeft(0L)((acc, m) => mix(acc, m.run(seed.toLong, steps)))

  final class Counter:
    var n: Int = 0

  trait Counted:
    def counter: Counter
    def hit(x: Long): Long =
      counter.n += 1
      x

  trait Doubled:
    def twice(x: Long): Long = x * 2L

  trait Offset:
    def offset: Long
    def shifted(x: Long): Long = x + offset

  final class Worker(val offset: Long) extends Counted with Doubled with Offset:
    val counter = new Counter

  // Default methods of three traits mixed into one class, called through the class.
  def traitPass(count: Int, seed: Int): Long =
    val w = new Worker(seed.toLong)
    var acc = 0L
    var i = 0
    while i < count do
      acc = mix(acc, w.shifted(w.twice(w.hit(i.toLong))))
      i += 1
    mix(acc, w.counter.n.toLong)

  final case class Key(x: Int, y: Int)
  final case class Cell(key: Key, value: Long)

  // Case classes as Map keys: hashing and equality per lookup, and the copy-on-update store.
  def keyPass(count: Int, seed: Int): Long =
    var grid = Map.empty[Key, Long]
    var i = 0
    while i < count do
      val key = Key(i % 16, (i * 7 + seed) % 16)
      grid = grid.updated(key, grid.getOrElse(key, 0L) + i.toLong)
      i += 1
    var acc = grid.size.toLong
    var j = 0
    while j < count do
      acc = mix(acc, grid.getOrElse(Key(j % 16, (j * 5) % 16), -1L))
      j += 1
    val cells = grid.toList.map((k, v) => Cell(k, v)).sortBy(c => c.key.x * 16 + c.key.y)
    acc = mix(acc, cells.head.value)
    acc = mix(acc, if cells.contains(Cell(Key(0, 0), cells.head.value)) then 1L else 0L)
    acc = mix(acc, if Key(1, 2) == Key(1, 2) && Key(1, 2) != Key(2, 1) then 1L else 0L)
    acc

  def round(seed: Int): Long =
    var acc = shapesPass(256, seed)
    acc = mix(acc, visitorPass(256, seed))
    acc = mix(acc, machinePass(256, seed))
    acc = mix(acc, traitPass(256, seed))
    acc = mix(acc, keyPass(256, seed))
    acc

  def work(rounds: Int): Long =
    var acc = 0L
    var r = 0
    while r < rounds do
      acc = mix(acc, round(r * 11 + 1))
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
    if report then System.err.println(line("dispatch", n, times))
    println("dispatch checksum " + checksum)

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
