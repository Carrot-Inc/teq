// Collections: List, Vector, Map and Set building, map/filter/fold, groupBy, sorted, and a
// persistent-map update loop. One round does a fixed amount of work over 64 orders, so the time
// grows linearly with the round count. Nothing whose printed form differs between the JVM and
// JavaScript is printed: the checksum is a modular hash of Longs, and every traversal of a Map
// or a Set goes over sorted keys, since teq keeps insertion order where scalac hashes.
object CollectionsBench:
  // Rounds, timed iterations and whether the per-iteration line is printed on stderr, when the
  // command line gives none; bench/runtime.sh compiles copies with its own values in this line,
  // since Scala.js gives main no arguments.
  val defaults = "200 1 0"

  val modulus = 1000000007L
  val customers = Vector("ann", "bob", "cy", "dee", "eve", "fay", "gus", "hal")
  val items = Vector("tea", "mug", "jar", "pot", "cup", "lid", "tin", "box")

  final case class Order(id: Int, customer: String, item: String, qty: Int, cents: Int)

  def mix(acc: Long, x: Long): Long = (acc * 31L + x) % modulus

  def orders(seed: Int, count: Int): List[Order] =
    var acc: List[Order] = Nil
    var i = count
    while i > 0 do
      i -= 1
      val k = seed + i
      acc = Order(k, customers((k * 5) % 8), items((k * 3) % 8), 1 + (k % 5), 100 + (k * 37) % 900) :: acc
    acc

  // map / filter / fold over a List, and the same over the Vector it is copied into.
  def listPass(os: List[Order]): Long =
    val totals = os.map(o => o.qty.toLong * o.cents.toLong)
    val big = totals.filter(_ > 400L)
    val sum = big.foldLeft(0L)((a, b) => a + b)
    val counted = os.count(o => o.qty > 2)
    val named = os.filter(_.customer == "ann").map(_.item).mkString("|")
    val v = os.toVector
    val vsum = v.map(o => o.cents.toLong).foldLeft(0L)(_ + _)
    var acc = mix(sum, counted.toLong)
    acc = mix(acc, named.length.toLong)
    acc = mix(acc, vsum)
    acc = mix(acc, totals.sum % modulus)
    acc = mix(acc, os.flatMap(o => List(o.qty, o.qty + 1)).sum.toLong)
    acc

  // groupBy gives a Map whose iteration order differs between the two platforms, so the keys are
  // sorted before anything is folded over them.
  def groupPass(os: List[Order]): Long =
    val byCustomer = os.groupBy(_.customer)
    val keys = byCustomer.keys.toList.sorted
    var acc = keys.length.toLong
    var rest = keys
    while rest.nonEmpty do
      val key = rest.head
      rest = rest.tail
      val group = byCustomer(key)
      acc = mix(acc, group.foldLeft(0L)((a, o) => a + o.qty.toLong * o.cents.toLong))
      acc = mix(acc, key.length.toLong)
    val byItem = os.groupBy(_.item).map((item, grouped) => (item, grouped.length))
    val counts = byItem.toList.sortBy(_._1).map(_._2)
    counts.foldLeft(acc)((a, c) => mix(a, c.toLong))

  // sorted / sortBy / sortWith over a List and a Vector, with keys that have no ties so that a
  // sort which is not stable would agree all the same.
  def sortPass(os: List[Order]): Long =
    val byCents = os.sortBy(o => o.cents * 1000 + o.id)
    val ids = byCents.map(_.id)
    val names = os.map(o => o.customer + "-" + o.id).sorted
    val descending = os.toVector.sortWith((a, b) => a.id > b.id)
    var acc = 0L
    acc = mix(acc, ids.foldLeft(0L)((a, i) => (a * 3L + i.toLong) % modulus))
    acc = mix(acc, names.head.length.toLong + names.last.length.toLong)
    acc = mix(acc, descending.head.id.toLong)
    acc = mix(acc, byCents.head.cents.toLong)
    acc

  // Building a Vector by appending, which copies the whole store in teq's array-backed Vector
  // and shares a tree in scalac's, then updating every second element.
  def vectorPass(count: Int): Long =
    var built = Vector.empty[Int]
    var i = 0
    while i < count do
      built = built :+ ((i * 7) % 101)
      i += 1
    var updated = built
    var j = 0
    while j < count do
      updated = updated.updated(j, updated(j) + 1)
      j += 2
    mix(built.foldLeft(0L)((a, x) => a + x.toLong), updated.foldLeft(0L)((a, x) => a + x.toLong))

  // A persistent Map updated key by key, the shape the JVM doc calls out as copying the store,
  // with the lookups and the removal that go with it.
  def mapPass(os: List[Order], count: Int): Long =
    var byId = Map.empty[Int, Order]
    var rest = os
    while rest.nonEmpty do
      byId = byId.updated(rest.head.id, rest.head)
      rest = rest.tail
    var totals = Map.empty[String, Long]
    var i = 0
    while i < count do
      val key = customers((i * 5) % 8)
      totals = totals.updated(key, totals.getOrElse(key, 0L) + i.toLong)
      i += 1
    var acc = byId.size.toLong
    var keys = totals.keys.toList.sorted
    while keys.nonEmpty do
      acc = mix(acc, totals(keys.head))
      keys = keys.tail
    val shrunk = byId.removed(os.head.id)
    acc = mix(acc, shrunk.size.toLong)
    acc = mix(acc, if byId.contains(os.head.id) then 1L else 0L)
    acc = mix(acc, byId.get(-1).map(_.cents).getOrElse(7).toLong)
    acc

  // Sets: built element by element, combined, and compared. The size is what is folded in, never
  // the iteration order.
  def setPass(os: List[Order]): Long =
    var names = Set.empty[String]
    var rest = os
    while rest.nonEmpty do
      names = names + rest.head.customer
      rest = rest.tail
    val kinds = os.map(_.item).toSet
    val ids = os.map(_.id % 17).toSet
    var acc = names.size.toLong
    acc = mix(acc, kinds.size.toLong)
    acc = mix(acc, ids.size.toLong)
    acc = mix(acc, names.union(kinds).size.toLong)
    acc = mix(acc, ids.intersect(Set(1, 2, 3, 4, 5)).size.toLong)
    acc = mix(acc, (names -- kinds).size.toLong)
    acc = mix(acc, if names.contains("ann") then 1L else 0L)
    acc = mix(acc, names.toList.sorted.mkString(",").length.toLong)
    acc

  def round(seed: Int): Long =
    val os = orders(seed, 64)
    var acc = listPass(os)
    acc = mix(acc, groupPass(os))
    acc = mix(acc, sortPass(os))
    acc = mix(acc, vectorPass(64))
    acc = mix(acc, mapPass(os, 64))
    acc = mix(acc, setPass(os))
    acc

  def work(rounds: Int): Long =
    var acc = 0L
    var r = 0
    while r < rounds do
      acc = mix(acc, round(r * 13))
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
    if report then System.err.println(line("collections", n, times))
    println("collections checksum " + checksum)

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
