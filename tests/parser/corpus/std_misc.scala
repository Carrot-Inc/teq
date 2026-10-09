import scala.concurrent.duration.*
import scala.math.Ordering.Implicits.*
import scala.util.Random
import scala.util.chaining.*

def show(label: String, values: Any*): Unit = println(label + ": " + values.mkString(" | "))

final case class Version(major: Int, minor: Int) extends Ordered[Version]:
  def compare(that: Version): Int = if major != that.major then major - that.major else minor - that.minor

final case class Price(cents: Int)

given Ordering[Price] = Ordering.by(_.cents)

def total[T](xs: List[T])(using num: Numeric[T]): Double = num.toDouble(xs.sum)

def sumIfAllKnown[T](xs: List[Option[T]])(using num: Numeric[T]): Option[T] =
  xs.foldLeft(Option(num.zero))((acc, x) => for a <- acc; b <- x yield num.plus(a, b))

@main def main(): Unit =
  show("option ordering", List(Some(3), None, Some(1)).sorted, List(Option(2), Option(1)).max, List[Option[Int]](None, Some(0)).min)
  show("tuple3", List((1, "b", true), (1, "a", false), (0, "z", true)).sorted)
  show("tuple4", List((1, 2, 3, 4), (1, 2, 3, 0), (0, 9, 9, 9)).sorted, List((true, false, 1), (false, true, 2), (false, false, 3)).sorted)
  show("tuple5", List((1, 1, 1, 1, 2), (1, 1, 1, 1, 1)).min)
  show("boolean sort", List("libraryId", "name", "branchId").sortBy(c => (c != "libraryId", c != "branchId", c)))
  show("reverse", List(3, 1, 2).sorted(using Ordering.Int.reverse), List("b", "a").sorted(using Ordering.String.reverse), List(2L, 9L).sorted(using summon[Ordering[Long]].reverse))
  show("by", List(Price(5), Price(1)).sorted, List(Price(5), Price(1)).max, Ordering.by[Price, Int](p => -p.cents).compare(Price(1), Price(2)) > 0)
  show("on", List("ccc", "a", "bb").sorted(using Ordering.Int.on[String](_.length)), Ordering.fromLessThan[Int](_ > _).compare(1, 2) > 0)
  show("orElse", List((1, "b"), (0, "z"), (1, "a")).sorted(using Ordering.by[(Int, String), Int](_._1).orElseBy(_._2)))
  show("ordering ops", Ordering.Int.lt(1, 2), Ordering.Int.max(1, 2), Ordering.String.min("a", "b"), Ordering.Int.equiv(1, 1))
  show("infix", Price(1) < Price(2), Price(3) >= Price(4), Price(1).max(Price(7)), (1, "a") < (1, "b"))
  show("ordered", Version(1, 2) < Version(1, 10), Version(2, 0) >= Version(1, 9), List(Version(2, 0), Version(1, 5)).sorted(using Ordering.ordered[Version]))
  show("doubles", List(2.5, -1.0, 0.5).sorted.map(_ * 2 == 5.0), List(1.5, 0.25).min == 0.25, List(1.5, 0.25).sum == 1.75)

  show("numeric", total(List(1, 2, 3)) == 6.0, total(List(1L, 2L)) == 3.0, total(List(0.5, 0.25)) == 0.75)
  show("sumIfAllKnown", sumIfAllKnown(List(Some(1), Some(2))), sumIfAllKnown(List(Some(1), None)), sumIfAllKnown(List.empty[Option[Long]]))
  val num = summon[Numeric[Int]]
  show("numeric ops", num.fromInt(3), num.negate(3), num.abs(-3), num.signum(-9), num.toLong(4), num.compare(1, 2) < 0, num.max(1, 2), num.one + num.zero)
  show("integral", summon[Integral[Int]].quot(7, 2), summon[Integral[Long]].rem(7L, 4L), summon[Fractional[Double]].div(1.0, 4.0) == 0.25)

  show("math", math.hypot(3.0, 4.0) == 5.0, math.floorMod(-7, 3), math.floorMod(7, -3), math.floorDiv(-7, 2), math.floorMod(-7L, 3L), math.signum(-2.5) == -1.0, math.signum(5))
  show("math round", math.round(2.5), math.round(-2.5), math.round(2.4), 2.5.round, (-0.5).round, 1e10.round, math.abs(-3), math.max(1L, 2L), math.min(1.5, 0.5) == 0.5)
  show("math more", math.pow(2.0, 10.0) == 1024.0, math.sqrt(16.0) == 4.0, math.cbrt(27.0) == 3.0, math.toDegrees(math.Pi) == 180.0, Math.floorMod(-1, 5), Math.hypot(6.0, 8.0) == 10.0)
  show("double ops", 2.5.isWhole, 2.0.isWhole, (-2.5).abs == 2.5, 2.5.floor == 2.0, 2.5.ceil == 3.0, (0.0 / 0.0).isNaN, (1.0 / 0.0).isInfinite, 3.7.toInt, (-3.7).toInt)

  show("pipe", 5.pipe(_ + 1).pipe(_ * 2), "a".pipe(s => s + s))
  show("tap", List(1, 2).tap(xs => println("tapped " + xs.length.toString)).map(_ + 1))

  val add = (a: Int, b: Int) => a + b
  val add3 = (a: Int, b: Int, c: Int) => a + b + c
  def render(prefix: String)(id: Int, name: String): String = prefix + id.toString + name
  show("tupled", add.tupled((1, 2)), add3.tupled((1, 2, 3)), List((1, 2), (3, 4)).map(add.tupled), Option((7, "x")).map(render("#").tupled))
  show("curried", add.curried(1)(2), add3.curried(1)(2)(3), List(1, 2).map(add.curried(10)))
  show("function", Function.const(1)("ignored"), List(1, 2).map(Function.const("k")), Function.chain(List[Int => Int](_ + 1, _ * 2))(5), Function.untupled[Int, Int, Int](add.tupled)(2, 3))
  show("compose", ((x: Int) => x + 1).andThen(_ * 2)(5), ((x: Int) => x + 1).compose((s: String) => s.length)("abc"), identity(3), (1, "a").swap)

  val rnd = new Random(42)
  show("random ints", rnd.nextInt(), rnd.nextInt(10), rnd.nextInt(100), rnd.nextInt(16), rnd.nextInt(1000000))
  show("random more", rnd.nextLong(), rnd.nextBoolean(), rnd.nextDouble() < 1.0, rnd.between(5, 10), rnd.nextPrintableChar())
  show("random double", new Random(7).nextDouble() == new Random(7).nextDouble(), (new Random(7).nextDouble() * 1000000).toInt)
  show("shuffle", new Random(1).shuffle(List(1, 2, 3, 4, 5)), new Random(2).shuffle(Vector("a", "b", "c")))
  rnd.setSeed(99L)
  show("reseeded", rnd.nextInt(50), new Random(99L).nextInt(50))
  val unseeded = Random.nextInt(10)
  show("global", unseeded >= 0 && unseeded < 10, Random.nextDouble() < 1.0, Random.shuffle(List(1, 2, 3)).sorted, Random.between(3, 4))

  show("durations", 5.seconds, 1.second, 90.minutes.toHours, 1500.millis.toSeconds, 2.hours.toMinutes, 1.day.toHours, 3L.milli.toMillis, 120000L.milli.toMinutes)
  show("duration math", 5.seconds + 500.millis, 1.minute - 1.second, 2.seconds * 3, 10.seconds / 4, 5.seconds == 5000.millis, 1.minute > 59.seconds, 1.minute.max(2.minutes))
  show("duration sort", List(2.seconds, 1500.millis, 1.minute).sorted, 3.hours.toString, 1.nanos.toNanos, 2.micros.toNanos)
