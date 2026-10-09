import scala.collection.immutable.ListMap

// Random updates of random earlier versions, checked against association lists.
final class Lcg(private var state: Long):
  def next(bound: Int): Int =
    state = (state * 6364136223846793005L + 1442695040888963407L) & 0x7fffffffffffffffL
    ((state >>> 33) % bound.toLong).toInt

def modelUpdated(model: List[(Int, Int)], key: Int, value: Int): List[(Int, Int)] =
  if model.exists(_._1 == key) then model.map(e => if e._1 == key then (key, value) else e)
  else model :+ (key, value)

def run(seed: Long, steps: Int, keys: Int, removals: Int): String =
  val rnd = new Lcg(seed)
  var maps = Vector(ListMap.empty[Int, Int])
  var sets = Vector(Set.empty[Int])
  var models = Vector(List.empty[(Int, Int)])
  var failures = 0
  var step = 0
  while step < steps do
    val from = if rnd.next(4) == 0 then rnd.next(maps.length) else maps.length - 1
    val key = rnd.next(keys)
    val remove = rnd.next(10) < removals
    val map = maps(from)
    val set = sets(from)
    val model = models(from)
    if remove then
      maps = maps :+ (map - key)
      sets = sets :+ (set - key)
      models = models :+ model.filter(_._1 != key)
    else
      maps = maps :+ map.updated(key, step)
      sets = sets :+ (set + key)
      models = models :+ modelUpdated(model, key, step)
    val check = rnd.next(maps.length)
    val expected = models(check)
    val actual = maps(check)
    if actual.toList != expected || actual.size != expected.length || sets(check) != expected.map(_._1).toSet then failures += 1
    if expected.nonEmpty then
      val probe = expected(rnd.next(expected.length))
      if actual.get(probe._1) != Some(probe._2) || !sets(check).contains(probe._1) then failures += 1
    if actual.contains(keys) || sets(check).contains(keys) then failures += 1
    step += 1
  val last = maps.last
  "failures " + failures.toString + ", versions " + maps.length.toString + ", last size " + last.size.toString +
    ", sum " + last.values.sum.toString + ", first " + last.headOption.toString

@main def main(): Unit =
  println(run(1L, 3000, 12, 3))
  println(run(2L, 3000, 300, 5))
  println(run(3L, 2000, 60, 8))
  println(run(4L, 2000, 1000, 1))
