// `java.util.TreeSet` against the JDK's: over strings and over a comparator, the navigation at
// absent elements and at both ends, the polls of a set of one and of none, the subsets in both
// forms (views that refuse an element out of their range), the descending set and iterator,
// removal through the iterators, the constructors (from a collection; from a sorted set, the
// comparator kept), and `removeAll`, which removes by the set's comparator when the set is the
// larger and by the argument's `contains` otherwise, as `AbstractSet`'s does.
import java.util as ju

object Main:
  def attempt(label: String)(body: => Any): Unit =
    val out =
      try "" + body
      catch
        case e: IllegalArgumentException => s"IllegalArgumentException(${e.getMessage})"
        case e: RuntimeException => e.getClass.getName
    println(s"$label: $out")

  def fruit(): ju.TreeSet[String] =
    val s = new ju.TreeSet[String]()
    for w <- List("pear", "apple", "fig", "kiwi", "date") do s.add(w)
    s

  def main(args: Array[String]): Unit =
    val s = fruit()
    println(s"$s ${s.size()} ${s.comparator()}")
    println(s"${s.add("fig")} ${s.add("lime")} ${s.remove("kiwi")} ${s.remove("kiwi")} ${s.contains("date")} ${s.contains("plum")}")
    println(s"${s.first()} ${s.last()} $s")
    for e <- List("a", "apple", "cherry", "fig", "lime", "pear", "zucchini") do
      println(s"$e: ${s.lower(e)} ${s.floor(e)} ${s.ceiling(e)} ${s.higher(e)}")
    println(s"${s.headSet("fig")} ${s.headSet("fig", true)} ${s.tailSet("fig")} ${s.tailSet("fig", false)}")
    println(s"${s.subSet("b", "m")} ${s.subSet("apple", false, "lime", true)} ${s.subSet("fig", "fig")}")
    attempt("subSet from above to")(s.subSet("m", "b"))

    // Subsets are views: they add and remove through, and refuse what is out of their range.
    val sub = s.subSet("b", true, "m", false)
    println(s"${sub.add("cherry")} ${sub.remove("fig")} ${sub.remove("pear")} ${sub.contains("pear")} $sub $s")
    attempt("add above")(sub.add("zucchini"))
    attempt("add at the exclusive bound")(sub.add("m"))
    attempt("headSet past the bound")(sub.headSet("n"))
    println(s"${sub.first()} ${sub.last()} ${sub.lower("cherry")} ${sub.higher("lime")} ${sub.ceiling("a")} ${sub.floor("z")}")
    println(s"${sub.pollFirst()} ${sub.pollLast()} $sub $s")

    // The descending set and iterator.
    val d = s.descendingSet()
    println(s"$d ${d.first()} ${d.last()} ${d.lower("date")} ${d.higher("date")} ${d.headSet("date")} ${d.tailSet("date", false)}")
    println(s"${d.descendingSet()} ${d.comparator().compare("a", "b")} ${d.subSet("pear", "apple")}")
    d.add("banana")
    println(s)
    val di = s.descendingIterator()
    val seen = new StringBuilder
    while di.hasNext do
      val e = di.next()
      seen.append(e).append(" ")
      if e.startsWith("b") then di.remove()
    println(s"${seen.toString.trim} $s")
    val it = s.iterator()
    attempt("remove before next")(it.remove())
    while it.hasNext do if it.next().length == 4 then it.remove()
    attempt("remove twice")(it.remove())
    println(s)

    // A set of one and of none.
    val one = new ju.TreeSet[String]()
    one.add("only")
    println(s"${one.pollFirst()} ${one.pollFirst()} ${one.pollLast()} ${one.isEmpty()} $one")
    one.add("again")
    println(s"${one.pollLast()} $one ${one.lower("x")} ${one.ceiling("a")}")
    attempt("first of none")(one.first())
    attempt("last of none")(one.last())
    attempt("add null")(one.add(null))

    // A comparator's order, kept by a set built from the sorted set.
    val byLength: ju.Comparator[String] = (a, b) => if a.length != b.length then a.length - b.length else a.compareTo(b)
    val c = new ju.TreeSet[String](byLength)
    for w <- List("pear", "fig", "banana", "kiwi", "apple", "date") do c.add(w)
    println(s"$c ${c.first()} ${c.ceiling("zzz")} ${c.headSet("kiwi")} ${c.comparator() eq byLength}")
    val copy = new ju.TreeSet[String](c)
    copy.add("plum")
    println(s"$copy ${copy.comparator() eq byLength} $c")
    val asCollection: ju.Collection[String] = c
    println(s"${new ju.TreeSet[String](asCollection)} ${new ju.TreeSet[String](asCollection).comparator()}")
    val fromList = new ju.TreeSet[String](ju.Arrays.asList("b", "a", "c", "a"))
    println(s"$fromList ${fromList.size()}")
    val reversed = new ju.TreeSet[String](s.descendingSet())
    reversed.add("cherry")
    println(s"$reversed ${reversed.first()}")
    val byOrdering = new ju.TreeSet[Int](Ordering.Int.reverse)
    for i <- List(4, -2, 9, 0, 4) do byOrdering.add(i)
    println(s"$byOrdering ${byOrdering.higher(4)} ${byOrdering.lower(4)}")

    // `removeAll` by the comparator or by `contains`; `retainAll` by `contains`.
    val caseless: ju.Comparator[String] = (a, b) => a.compareToIgnoreCase(b)
    def letters(): ju.TreeSet[String] =
      val l = new ju.TreeSet[String](caseless)
      for w <- List("a", "B", "c") do l.add(w)
      l
    val l1 = letters()
    println(s"${l1.removeAll(ju.Arrays.asList("A"))} $l1")
    val l2 = letters()
    println(s"${l2.removeAll(ju.Arrays.asList("A", "x", "y", "z"))} $l2")
    val l3 = letters()
    println(s"${l3.retainAll(ju.Arrays.asList("a", "b"))} $l3 ${l3.contains("A")}")
    val l4 = letters()
    println(s"${l4.headSet("c").removeAll(ju.Arrays.asList("b", "C"))} $l4")
    val l5 = letters()
    println(s"${l5.descendingSet().retainAll(ju.Arrays.asList("c", "a"))} $l5")

    // Natural ordering of other kinds.
    val ints = new ju.TreeSet[Int]()
    for i <- List(5, -3, 12, 0, -40, 7) do ints.add(i)
    println(s"$ints ${ints.headSet(0)} ${ints.ceiling(6)} ${ints.floor(-41)}")
    val chars = new ju.TreeSet[Char]()
    for ch <- "teq tree" do chars.add(ch)
    println(s"$chars ${chars.size()}")
